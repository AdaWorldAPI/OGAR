//! REST discovery, registration and the capability fetch of the HIRO invoker,
//! sans-IO.
//!
//! Two halves, both pure:
//!
//! * **Parsers** of the REST bodies the invoker reads: `GET /api/version`
//!   ([`parse_versions`]), the auth answer ([`parse_auth_response`]), the path
//!   prefix ([`prefix_path`]), and [`capability_defaults`], which reads a
//!   capabilities body the way the invoker does. The capabilities and
//!   applicabilities bodies themselves are read by the existing
//!   [`crate::parse_capabilities`], [`crate::parse_applies_on`] and
//!   [`crate::parse_applicabilities_typed`]; [`Registration`] only hands the raw
//!   bodies to them.
//! * **The fetch state machine** [`FetchState`]: the invoker fetches the
//!   capabilities and then the applicabilities, retries any failure on HIRO's
//!   backoff curve and refetches on `configChanged`. It outputs "fetch this
//!   path" and takes "this path returned status/body/error". There is no HTTP
//!   client, no timer and no clock read here; the caller sends the requests,
//!   keeps the time and applies its own request timeout (a timeout is a
//!   [`FetchOutcome::Transport`] error).
//!
//! The reference is graph_conn. `GR` is `lib/graph_conn/graph_rest_calls.ex`,
//! `RQ` `lib/graph_conn/request.ex`, `I` `lib/action_api/invoker.ex`, `B`
//! `lib/graph_conn/backoff.ex`, `CM` `lib/graph_conn/connection_manager.ex`, `RA`
//! `lib/graph_conn/retry_after.ex`. The oracle's line numbers for `GR` are
//! 15 to 20 lines off the checked-out source; those below are the source's.
//!
//! # `GET /api/version`
//!
//! The answer is a JSON object keyed by API name. An entry needs the strings
//! `endpoint`, `protocols` and `version` (`GR:115-119`); anything else is
//! skipped and its name reported in [`Versions::skipped`] (`GR:132`, the warning
//! of `GR:134-148`). A name must be 1 to 64 characters of `[a-zA-Z0-9]` then
//! `[a-zA-Z0-9_.-]` (`GR:14-15`, `:177-180`); another is skipped as malformed
//! too. No usable entry is [`VersionsError::NoUsableApi`] (`GR:152-153`).
//! The action REST API is the entry `action`, the socket `action-ws`
//! ([`Versions::action`], [`Versions::action_ws`]); the oracle's example entries
//! are `mock_router.ex:199-216`.
//!
//! # Choices where the source is silent or cannot be copied
//!
//! * A capabilities body and an applicabilities body must each be a JSON
//!   object. HIRO accepts any 200 body (an undecodable one is kept as the raw
//!   string, `GR:417-424`) and would fail later, in
//!   `capability_defaults/1`; here such a body is a failed fetch
//!   ([`FetchError::BadBody`]) and is retried.
//! * `capability_defaults` treats a capability without `mandatoryParameters` or
//!   `optionalParameters` as having none; HIRO raises there (`I:351-352`).
//! * The auth `_TOKEN` must be a string. HIRO only matches the key (`GR:292`);
//!   a non-string token cannot be a bearer token.
//! * [`FetchState`] draws its jitter from a seeded generator ([`Draw`]); HIRO uses
//!   `:rand`. [`Draw::Low`] and [`Draw::High`] pin the ends of each range.
//! * A `configChanged` while a refetch is already running, or while the
//!   invoker is not ready, is ignored.

use std::collections::BTreeMap;
use std::fmt;

use serde_json::{Map, Value};

use crate::applicability::{Applicabilities, ApplicabilityError, parse_applicabilities_typed};
use crate::submit_state::Millis;
use crate::wire::MAX_RETRY_AFTER_MS;

// ------------------------------------------------------------- versions --

/// The path of the version request (`GR:66`).
pub const VERSION_PATH: &str = "/api/version";
/// The longest API name that is read (`GR:14`).
pub const API_NAME_MAX: usize = 64;

/// One API of `GET /api/version`: `endpoint`, `protocols`, `version`
/// (`GR:115-125`: `path`, `protocol`, `subprotocol`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiEntry {
    /// `endpoint`: the path prefix of the API, e.g. `/api/0.9/action/`.
    pub path: String,
    /// `protocols`: the WebSocket protocol, e.g. `action-0.9.0`; empty for REST.
    pub protocol: String,
    /// `version`: the subprotocol, e.g. `0.9`.
    pub subprotocol: String,
}

/// The usable APIs of a version answer, and the names that were skipped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Versions {
    /// The usable entries by API name.
    pub apis: BTreeMap<String, ApiEntry>,
    /// The names of the entries that were skipped as malformed, sorted. HIRO
    /// logs these (`GR:134-148`).
    pub skipped: Vec<String>,
}

/// Why a version answer is refused (`GR:76-92`, `:98-153`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VersionsError {
    /// The body is not JSON: `{:bad_versions, :not_json}`.
    NotJson,
    /// The body is JSON but not an object: `{:bad_versions, :not_an_object}`.
    NotAnObject,
    /// No entry is usable: `{:bad_versions, :no_usable_api}`.
    NoUsableApi,
    /// Status 429; the wait the `retry-after` header advertised, 0 if none.
    RateLimited(u64),
    /// Any other status: `{:unexpected_status, status}`.
    UnexpectedStatus(u16),
}

/// A request for an API the answer did not name: `{:unknown_api, known}`
/// (`GR:278`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownApi {
    /// The names that are known.
    pub known: Vec<String>,
}

impl Versions {
    /// The entry of `api`, or the names that are known (`GR:274-280`).
    ///
    /// # Errors
    /// [`UnknownApi`] when the answer did not name `api`.
    pub fn entry(&self, api: &str) -> Result<&ApiEntry, UnknownApi> {
        self.apis.get(api).ok_or_else(|| UnknownApi {
            known: self.apis.keys().cloned().collect(),
        })
    }

    /// The action REST API, `action`.
    #[must_use]
    pub fn action(&self) -> Option<&ApiEntry> {
        self.apis.get("action")
    }

    /// The action WebSocket API, `action-ws`.
    #[must_use]
    pub fn action_ws(&self) -> Option<&ApiEntry> {
        self.apis.get("action-ws")
    }

    /// HIRO asks the graph and the auth service separately and keeps only
    /// `auth` of the second answer (`GR:68-72`).
    #[must_use]
    pub fn with_auth_from(mut self, auth_answer: &Versions) -> Self {
        if let Some(a) = auth_answer.apis.get("auth") {
            self.apis.insert("auth".to_owned(), a.clone());
        }
        self
    }
}

/// `\A[a-zA-Z0-9][a-zA-Z0-9_.-]{0,63}\z` (`GR:14-15`).
fn is_api_name(name: &str) -> bool {
    let b = name.as_bytes();
    !b.is_empty()
        && b.len() <= API_NAME_MAX
        && b[0].is_ascii_alphanumeric()
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'-'))
}

/// Read the body of a `200` answer of `GET /api/version`.
///
/// A malformed entry is skipped, not fatal: it is named in
/// [`Versions::skipped`] and the rest are kept (`GR:105-109`, `:132-133`).
///
/// # Errors
/// [`VersionsError::NotJson`], [`VersionsError::NotAnObject`] or
/// [`VersionsError::NoUsableApi`].
pub fn parse_versions(body: &str) -> Result<Versions, VersionsError> {
    let value: Value = serde_json::from_str(body).map_err(|_| VersionsError::NotJson)?;
    let Value::Object(entries) = value else {
        return Err(VersionsError::NotAnObject);
    };
    let mut versions = Versions::default();
    for (name, entry) in &entries {
        match take_entry(name, entry) {
            Some(e) => {
                versions.apis.insert(name.clone(), e);
            }
            None => versions.skipped.push(name.clone()),
        }
    }
    versions.skipped.sort();
    if versions.apis.is_empty() {
        return Err(VersionsError::NoUsableApi);
    }
    Ok(versions)
}

fn take_entry(name: &str, entry: &Value) -> Option<ApiEntry> {
    if !is_api_name(name) {
        return None;
    }
    let obj = entry.as_object()?;
    let text = |k: &str| obj.get(k)?.as_str().map(str::to_owned);
    Some(ApiEntry {
        path: text("endpoint")?,
        protocol: text("protocols")?,
        subprotocol: text("version")?,
    })
}

/// Read a whole answer of `GET /api/version`: `200` is parsed, `429` is a rate
/// limit with its `retry-after`, any other status is refused (`GR:80-92`).
///
/// # Errors
/// A [`VersionsError`].
pub fn versions_response(
    status: u16,
    headers: &[(String, String)],
    body: &str,
) -> Result<Versions, VersionsError> {
    match status {
        200 => parse_versions(body),
        429 => Err(VersionsError::RateLimited(retry_after_ms(headers))),
        other => Err(VersionsError::UnexpectedStatus(other)),
    }
}

/// The path of a request: the API's endpoint with the path appended, plain
/// concatenation as `Request.set_namespace` does (`RQ:38-39`; `GR:263`). The
/// endpoint ends in `/` and the path does not start with one:
/// `prefix_path("/api/0.9/action/", "capabilities")` is
/// `/api/0.9/action/capabilities`.
#[must_use]
pub fn prefix_path(endpoint: &str, path: &str) -> String {
    format!("{endpoint}{path}")
}

/// The path of the auth request: the auth endpoint then `app` (`GR:205`).
#[must_use]
pub fn auth_path(auth_endpoint: &str) -> String {
    prefix_path(auth_endpoint, "app")
}

/// The wait an HTTP `retry-after` header advertises, in ms (`RA:27-35`,
/// `:93-123`): only delay-seconds is read, the name is case-insensitive, the
/// value is trimmed, a wait past a day is a day, and a missing, non-numeric or
/// non-positive one is `0`.
#[must_use]
pub fn retry_after_ms(headers: &[(String, String)]) -> u64 {
    let Some((_, value)) = headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("retry-after"))
    else {
        return 0;
    };
    let t = value.trim();
    let digits = t.strip_prefix(['+', '-']).unwrap_or(t);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || t.starts_with('-') {
        return 0;
    }
    // Digits only. A value too large for `u128` is far past a day, so a day.
    digits.parse::<u128>().map_or(MAX_RETRY_AFTER_MS, |s| {
        u64::try_from(s.saturating_mul(1_000))
            .unwrap_or(MAX_RETRY_AFTER_MS)
            .min(MAX_RETRY_AFTER_MS)
    })
}

// ----------------------------------------------------------------- auth --

/// The token the auth API answered with (`GR:292-299`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    /// `_TOKEN`.
    pub token: String,
    /// `expires-at`, unix time in milliseconds (`GR:184-186`).
    pub expires_at: i64,
}

/// Why an auth answer is refused (`GR:283-329`, `:208-232`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AuthError {
    /// The body is not JSON: `{:invalid_auth_response, :not_json}`.
    NotJson,
    /// The body is JSON but not an object: `{:invalid_auth_response, :not_an_object}`.
    NotAnObject,
    /// An object without `_TOKEN` and `expires-at`: `{:invalid_auth_response, keys}`,
    /// the keys sorted.
    InvalidAuthResponse(Vec<String>),
    /// `expires-at` is not a number: `{:invalid_expires_at, value}`.
    InvalidExpiresAt(Value),
    /// Status 401: `:wrong_credentials`.
    WrongCredentials,
    /// Status 429, with the advertised wait in ms (0 if none).
    RateLimited(u64),
    /// Any other status.
    Rejected(u16),
}

/// Read the body of a `200` answer of the auth request.
///
/// `expires-at` is an integer, or a float which is truncated (`GR:321-326`).
/// Only the type is checked: a past or wrong-unit timestamp is accepted
/// (`GR:317-320`).
///
/// # Errors
/// An [`AuthError`].
pub fn parse_auth_response(body: &str) -> Result<Auth, AuthError> {
    let value: Value = serde_json::from_str(body).map_err(|_| AuthError::NotJson)?;
    let Value::Object(obj) = value else {
        return Err(AuthError::NotAnObject);
    };
    let (Some(Value::String(token)), Some(expires)) = (obj.get("_TOKEN"), obj.get("expires-at"))
    else {
        let mut keys: Vec<String> = obj.keys().cloned().collect();
        keys.sort();
        return Err(AuthError::InvalidAuthResponse(keys));
    };
    let expires_at = match expires {
        Value::Number(n) => n.as_i64().or_else(|| {
            n.as_f64()
                .filter(|f| f.is_finite() && f.abs() < 9.0e18)
                .map(|f| f.trunc() as i64)
        }),
        _ => None,
    };
    match expires_at {
        Some(expires_at) => Ok(Auth {
            token: token.clone(),
            expires_at,
        }),
        None => Err(AuthError::InvalidExpiresAt(expires.clone())),
    }
}

/// Read a whole auth answer by status (`GR:208-232`).
///
/// # Errors
/// An [`AuthError`].
pub fn auth_response(
    status: u16,
    headers: &[(String, String)],
    body: &str,
) -> Result<Auth, AuthError> {
    match status {
        200 => parse_auth_response(body),
        401 => Err(AuthError::WrongCredentials),
        429 => Err(AuthError::RateLimited(retry_after_ms(headers))),
        other => Err(AuthError::Rejected(other)),
    }
}

// ----------------------------------------------------- capability defaults --

/// The field defaults of one capability, as `capability_defaults/1` reads them
/// (`I:343-360`): the `default` of every parameter in `mandatoryParameters` and
/// `optionalParameters`, the optional one winning when a field is in both. A
/// parameter with no `default` key has none; a `default` of `null` is a default.
/// An unknown capability has none.
///
/// `capabilities` is the capabilities body as parsed JSON, the shape
/// `{"<Name>": {"mandatoryParameters": {"<p>": {...}}, "optionalParameters":
/// {"<p>": {"default": V}}}}` (`GR`-independent; `mock.ex:592-611`, verbatim in
/// `mock_test.exs:73-93`).
#[must_use]
pub fn capability_defaults(capabilities: &Value, capability: &str) -> Map<String, Value> {
    let mut merged: Map<String, Value> = Map::new();
    let Some(cap) = capabilities.get(capability) else {
        return Map::new();
    };
    for group in ["mandatoryParameters", "optionalParameters"] {
        if let Some(Value::Object(params)) = cap.get(group) {
            for (field, p) in params {
                merged.insert(field.clone(), p.clone());
            }
        }
    }
    merged
        .into_iter()
        .filter_map(|(field, p)| match p {
            Value::Object(mut o) => o.remove("default").map(|d| (field, d)),
            _ => None,
        })
        .collect()
}

/// [`capability_defaults`] of every capability of a body that has one.
#[must_use]
pub fn all_capability_defaults(capabilities: &Value) -> BTreeMap<String, Map<String, Value>> {
    capabilities
        .as_object()
        .map(|o| {
            o.keys()
                .map(|name| (name.clone(), capability_defaults(capabilities, name)))
                .filter(|(_, d)| !d.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

// ------------------------------------------------------------ registration --

/// The two bodies the invoker keeps once it is ready (`I:310-320`).
#[derive(Debug, Clone, PartialEq)]
pub struct Registration {
    /// The capabilities body, as parsed JSON.
    pub capabilities: Value,
    /// The applicabilities body, as parsed JSON.
    pub applicabilities: Value,
}

impl Registration {
    /// The capabilities as concrete signatures, by the existing
    /// [`crate::parse_capabilities`].
    ///
    /// # Errors
    /// Its error message when the body is not a `MapOfCapabilities`.
    pub fn concrete_capabilities(
        &self,
    ) -> Result<Vec<ogar_from_schema::registration::ConcreteCapability>, String> {
        crate::parse_capabilities(&self.capabilities.to_string())
    }

    /// The applicabilities, by the existing [`parse_applicabilities_typed`].
    ///
    /// # Errors
    /// Its [`ApplicabilityError`].
    pub fn typed_applicabilities(&self) -> Result<Applicabilities, ApplicabilityError> {
        parse_applicabilities_typed(&self.applicabilities.to_string())
    }

    /// The handler ids: the keys of an applicabilities object (`mock.ex:122-124`;
    /// tests take `available_applicabilities() |> Map.keys() |> hd()` as the
    /// handler id, `invoker_test.exs:281-285`).
    #[must_use]
    pub fn handler_ids(&self) -> Vec<String> {
        self.applicabilities
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------- backoff --

/// HIRO's retry seed, ms (`CM:70`).
pub const RETRY_INITIAL_MS: u64 = 1_000;
/// HIRO's retry ceiling, ms (`CM:71`).
pub const RETRY_MAX_MS: u64 = 10_000;
/// HIRO's spread above an advertised wait, ms (`CM:72`).
pub const RETRY_JITTER_MS: u64 = 1_000;
/// HIRO's ceiling for an advertised wait, ms (`CM:73`).
pub const RETRY_FLOOR_MAX_MS: u64 = 300_000;

/// The four numbers of the backoff curve. [`Default`] is HIRO's default of each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffConfig {
    /// `:retry_initial_ms`.
    pub initial_ms: u64,
    /// `:retry_max_ms`.
    pub max_ms: u64,
    /// `:retry_jitter_ms`.
    pub jitter_ms: u64,
    /// `:retry_floor_max_ms`; `0` is "no ceiling" (`CM:1067`).
    pub floor_max_ms: u64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_ms: RETRY_INITIAL_MS,
            max_ms: RETRY_MAX_MS,
            jitter_ms: RETRY_JITTER_MS,
            floor_max_ms: RETRY_FLOOR_MAX_MS,
        }
    }
}

/// Where the jitter comes from. `uniform(n)` of Elixir is an integer in `1..=n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Draw {
    /// xorshift64* from this state (any value, 0 is mapped to a fixed one).
    Seeded(u64),
    /// Always the lowest value: 1.
    Low,
    /// Always the highest value: `n`.
    High,
}

impl Draw {
    /// A value in `1..=n`; `n` of 0 is treated as 1.
    pub fn uniform(&mut self, n: u64) -> u64 {
        let n = n.max(1);
        match self {
            Self::Low => 1,
            Self::High => n,
            Self::Seeded(state) => {
                let mut x = if *state == 0 {
                    0x9E37_79B9_7F4A_7C15
                } else {
                    *state
                };
                x ^= x >> 12;
                x ^= x << 25;
                x ^= x >> 27;
                *state = x;
                1 + x.wrapping_mul(0x2545_F491_4F6C_DD1D) % n
            }
        }
    }
}

/// `Backoff.jitter/1`: `half + uniform(max(half, 1))` with `half = delay / 2`
/// (`B:16-20`). A wait in `half + 1 ..= half + max(half, 1)`.
pub fn jitter(delay: u64, draw: &mut Draw) -> u64 {
    let half = delay / 2;
    half + draw.uniform(half.max(1))
}

/// The delay to wait and the curve value to carry into the next attempt
/// (`B:40-63`, `CM:465-475`).
///
/// The curve doubles `current` (at least 1) up to the cap, which is at least
/// the seed (`CM:466-469`), and jitters it. An advertised wait `advertised_ms`
/// (0 is none) is clamped to `floor_max_ms` (`CM:1063-1071`) and is a floor:
/// the delay is the larger of the jittered curve and `floor + uniform(spread)`.
/// The carried curve value ignores the floor.
pub fn next_delay(
    cfg: &BackoffConfig,
    current: u64,
    advertised_ms: u64,
    draw: &mut Draw,
) -> (u64, u64) {
    let cap = cfg.max_ms.max(cfg.initial_ms).max(1);
    let next_current = current.max(1).saturating_mul(2).min(cap);
    let curve = jitter(next_current, draw);
    let floor = if cfg.floor_max_ms == 0 {
        advertised_ms
    } else {
        advertised_ms.min(cfg.floor_max_ms)
    };
    let delay = if floor == 0 {
        curve
    } else {
        curve.max(floor + draw.uniform(cfg.jitter_ms.max(1)))
    };
    (delay, next_current)
}

// ------------------------------------------------------------ fetch machine --

/// Which of the two fetches a request is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchKind {
    /// `capabilities`.
    Capabilities,
    /// `applicabilities`.
    Applicabilities,
}

impl FetchKind {
    /// The path relative to the action API: `capabilities` or
    /// `applicabilities` (`I:760-761`).
    #[must_use]
    pub fn path(self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::Applicabilities => "applicabilities",
        }
    }
}

/// What the caller got back for a fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchOutcome {
    /// An HTTP answer.
    Response {
        /// The status code.
        status: u16,
        /// The headers (only `retry-after` is read).
        headers: Vec<(String, String)>,
        /// The body text.
        body: String,
    },
    /// No answer: a transport error, a closed connection, a request timeout.
    Transport(String),
}

/// Why a fetch failed, as HIRO's `{path, reason}` (`I:765-787`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FetchError {
    /// `{:status, code}`: any status but 200 and 429 (`I:776-777`).
    Status(u16),
    /// `{:rate_limited, ms}`: status 429 with the advertised wait, 0 if none
    /// (`I:772-773`, `:784-787`).
    RateLimited(u64),
    /// The transport's own error (`I:779-780`).
    Transport(String),
    /// A 200 whose body is not the expected JSON (see the module docs).
    BadBody,
}

/// A failed fetch: the path (`capabilities` or `applicabilities`) and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchFailure {
    /// The path relative to the action API.
    pub path: &'static str,
    /// Why.
    pub error: FetchError,
}

impl fmt::Display for FetchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.error {
            FetchError::Status(c) => write!(f, "{}: status {c}", self.path),
            FetchError::RateLimited(ms) => write!(f, "{}: rate limited, {ms}ms", self.path),
            FetchError::Transport(e) => write!(f, "{}: {e}", self.path),
            FetchError::BadBody => write!(f, "{}: body is not usable JSON", self.path),
        }
    }
}

/// What the caller feeds the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FetchInput {
    /// The connection is `:ready` and the invoker is still initialising
    /// (`I:160-170`). The first attempt is at once. A second one while a fetch is
    /// pending, or after the invoker is ready, does nothing (`I:790-797`).
    Start,
    /// The connection left `:ready`: a pending fetch or retry is cancelled
    /// (`I:172-184`). The next [`FetchInput::Start`] begins again from the seed.
    Cancel,
    /// The answer to an [`FetchOutput::Fetch`].
    Fetched {
        /// Which request it answers.
        kind: FetchKind,
        /// What came back.
        outcome: FetchOutcome,
    },
    /// `configChanged` arrived (`I:239-240`): refetch once (`reconfigure/0`,
    /// `I:366-381`).
    ConfigChanged,
    /// Time passed.
    Tick,
}

/// Something HIRO only logs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FetchLog {
    /// A fetch failed and will be retried in `retry_in_ms` (`I:882-884`).
    FetchFailed {
        /// The failure.
        failure: FetchFailure,
        /// The delay before the next attempt.
        retry_in_ms: u64,
    },
    /// A refetch after `configChanged` failed; the old registration is kept and
    /// nothing is retried (`I:375-380`).
    ReconfigureFailed(FetchFailure),
    /// A `configChanged` that does nothing here (not ready, or a refetch is
    /// already running).
    ConfigChangedIgnored,
}

/// What the machine asks the caller to do.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FetchOutput {
    /// `GET` this path and report the answer with [`FetchInput::Fetched`].
    Fetch {
        /// Which request.
        kind: FetchKind,
        /// The full path: the action endpoint with the kind's path appended.
        path: String,
    },
    /// Both bodies arrived: the invoker is ready (HIRO opens `action-ws` now,
    /// `I:825`).
    Ready(Registration),
    /// A refetch after `configChanged` succeeded: replace the registration
    /// (`I:373`).
    Reconfigured(Registration),
    /// HIRO logs this and does nothing else.
    Log(FetchLog),
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Idle,
    Fetching {
        kind: FetchKind,
        capabilities: Option<Value>,
    },
    Waiting {
        until: Millis,
    },
    Ready,
    Reconfiguring {
        kind: FetchKind,
        capabilities: Option<Value>,
    },
}

/// The capability fetch of one invoker.
///
/// ```
/// use ogar_action_handler::discovery::{
///     Draw, FetchInput, FetchKind, FetchOutcome, FetchOutput, FetchState,
/// };
///
/// let mut f = FetchState::new("/api/0.9/action/", Draw::Seeded(1));
/// let out = f.step(0, FetchInput::Start);
/// assert!(matches!(&out[0], FetchOutput::Fetch { path, .. } if path == "/api/0.9/action/capabilities"));
/// let fail = FetchOutcome::Response { status: 500, headers: vec![], body: String::new() };
/// let out = f.step(5, FetchInput::Fetched { kind: FetchKind::Capabilities, outcome: fail });
/// assert!(matches!(out[0], FetchOutput::Log(_)));
/// assert!(f.next_wakeup().is_some()); // the retry
/// ```
#[derive(Debug, Clone)]
pub struct FetchState {
    endpoint: String,
    cfg: BackoffConfig,
    draw: Draw,
    retry_in: u64,
    phase: Phase,
    clock: Millis,
}

impl FetchState {
    /// A machine fetching from the action API at `action_endpoint` (the
    /// `endpoint` of the `action` version entry, ending in `/`), with HIRO's
    /// backoff numbers.
    #[must_use]
    pub fn new(action_endpoint: impl Into<String>, draw: Draw) -> Self {
        Self::with_config(action_endpoint, BackoffConfig::default(), draw)
    }

    /// As [`FetchState::new`] with other backoff numbers.
    #[must_use]
    pub fn with_config(action_endpoint: impl Into<String>, cfg: BackoffConfig, draw: Draw) -> Self {
        Self {
            endpoint: action_endpoint.into(),
            cfg,
            draw,
            retry_in: cfg.initial_ms,
            phase: Phase::Idle,
            clock: 0,
        }
    }

    /// True once both bodies have arrived and the invoker is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self.phase, Phase::Ready | Phase::Reconfiguring { .. })
    }

    /// When the next attempt is due, if one is waiting.
    #[must_use]
    pub fn next_wakeup(&self) -> Option<Millis> {
        match self.phase {
            Phase::Waiting { until } => Some(until),
            _ => None,
        }
    }

    /// The curve value the next failure will double (HIRO's `retry_in`).
    #[must_use]
    pub fn retry_in(&self) -> u64 {
        self.retry_in
    }

    /// Apply one input at `now` (milliseconds, non-decreasing).
    pub fn step(&mut self, now: Millis, input: FetchInput) -> Vec<FetchOutput> {
        let now = now.max(self.clock);
        self.clock = now;
        let mut out = Vec::new();
        // A wait that is due ends first, as in the other machines.
        if let Phase::Waiting { until } = self.phase
            && until <= now
        {
            self.begin(FetchKind::Capabilities, None, &mut out, false);
        }
        match input {
            FetchInput::Start => {
                if self.phase == Phase::Idle {
                    self.retry_in = self.cfg.initial_ms;
                    self.begin(FetchKind::Capabilities, None, &mut out, false);
                }
            }
            FetchInput::Cancel => {
                if matches!(self.phase, Phase::Fetching { .. } | Phase::Waiting { .. }) {
                    self.phase = Phase::Idle;
                    self.retry_in = self.cfg.initial_ms;
                }
            }
            FetchInput::ConfigChanged => {
                if self.phase == Phase::Ready {
                    self.begin(FetchKind::Capabilities, None, &mut out, true);
                } else {
                    out.push(FetchOutput::Log(FetchLog::ConfigChangedIgnored));
                }
            }
            FetchInput::Fetched { kind, outcome } => self.fetched(now, kind, outcome, &mut out),
            FetchInput::Tick => {}
        }
        out
    }

    fn begin(
        &mut self,
        kind: FetchKind,
        capabilities: Option<Value>,
        out: &mut Vec<FetchOutput>,
        reconfigure: bool,
    ) {
        self.phase = if reconfigure {
            Phase::Reconfiguring { kind, capabilities }
        } else {
            Phase::Fetching { kind, capabilities }
        };
        out.push(FetchOutput::Fetch {
            kind,
            path: prefix_path(&self.endpoint, kind.path()),
        });
    }

    fn fetched(
        &mut self,
        now: Millis,
        kind: FetchKind,
        outcome: FetchOutcome,
        out: &mut Vec<FetchOutput>,
    ) {
        let (current, caps, reconfigure) = match &self.phase {
            Phase::Fetching { kind, capabilities } => (*kind, capabilities.clone(), false),
            Phase::Reconfiguring { kind, capabilities } => (*kind, capabilities.clone(), true),
            // Cancelled, waiting or ready: a stale answer.
            _ => return,
        };
        if current != kind {
            return;
        }
        match read_body(outcome) {
            Ok(body) => match kind {
                FetchKind::Capabilities => {
                    self.begin(FetchKind::Applicabilities, Some(body), out, reconfigure)
                }
                FetchKind::Applicabilities => {
                    let reg = Registration {
                        capabilities: caps.unwrap_or(Value::Null),
                        applicabilities: body,
                    };
                    self.phase = Phase::Ready;
                    self.retry_in = self.cfg.initial_ms;
                    out.push(if reconfigure {
                        FetchOutput::Reconfigured(reg)
                    } else {
                        FetchOutput::Ready(reg)
                    });
                }
            },
            Err(error) => {
                let failure = FetchFailure {
                    path: kind.path(),
                    error,
                };
                if reconfigure {
                    // Keeps what it had, and does not retry (`I:366-381`).
                    self.phase = Phase::Ready;
                    out.push(FetchOutput::Log(FetchLog::ReconfigureFailed(failure)));
                    return;
                }
                // A 429's wait is the floor of the next one (`I:889-894`).
                let advertised = match failure.error {
                    FetchError::RateLimited(ms) => ms,
                    _ => 0,
                };
                let (delay, next) =
                    next_delay(&self.cfg, self.retry_in, advertised, &mut self.draw);
                self.retry_in = next;
                self.phase = Phase::Waiting {
                    until: now.saturating_add(delay),
                };
                out.push(FetchOutput::Log(FetchLog::FetchFailed {
                    failure,
                    retry_in_ms: delay,
                }));
            }
        }
    }
}

fn read_body(outcome: FetchOutcome) -> Result<Value, FetchError> {
    let (status, headers, body) = match outcome {
        FetchOutcome::Transport(e) => return Err(FetchError::Transport(e)),
        FetchOutcome::Response {
            status,
            headers,
            body,
        } => (status, headers, body),
    };
    match status {
        200 => {
            let v: Value = serde_json::from_str(&body).map_err(|_| FetchError::BadBody)?;
            if v.is_object() {
                Ok(v)
            } else {
                Err(FetchError::BadBody)
            }
        }
        429 => Err(FetchError::RateLimited(retry_after_ms(&headers))),
        other => Err(FetchError::Status(other)),
    }
}
