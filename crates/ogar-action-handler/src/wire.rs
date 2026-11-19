//! The HIRO `action-ws` **wire codec**: JSON text frame to typed message and back.
//!
//! The reference is graph_conn, the Elixir client (handler and invoker), with the
//! Python and Java clients for the field names they share. Every frame is a JSON
//! text frame. A frame has a `type` discriminator, except the two gateway
//! refusals, which arrive typeless as `{"error": {"code": 429 | 503, ...}, "id":
//! ...}` (the in-band 429 and 503).
//!
//! | frame | wire keys |
//! |---|---|
//! | `submitAction` | `type`, `id`, `handler`, `capability`, `parameters` (object), `timeout` (integer, milliseconds) |
//! | `acknowledged` | `type`, `id`, optional `code`, optional `message` |
//! | `negativeAcknowledged` | `type`, `id`, `code`, `message` |
//! | `sendActionResult` | `type`, `id`, `result` (a string: the JSON text of the result) |
//! | `configChanged` | `type` only |
//! | `error` | `type`, `code`, `message` |
//! | `hello` | `type`, optional `max_frame_bytes` |
//! | `clientHello` | `type`, optional `client {app, version}`, optional `settings {redelivery}` |
//! | in-band 429 | no `type`; `error {code: 429, message?, retryAfterMs?}`, `id?` |
//! | in-band 503 | no `type`; `error {code: 503, message?, upstream?}`, `id?` |
//!
//! Decoding never panics. A frame that is not JSON, not an object, has an unknown
//! or non-string `type`, no `type` and no gateway refusal, or a missing or
//! ill-typed field is a [`WireError`] that names it. Nothing is dropped silently.
//!
//! # Mapping to the existing types
//!
//! [`SubmitAction`], [`SendActionResult`], [`NegativeAcknowledged`] and
//! [`InboundError`] are the frame payloads. Differences the wire forces:
//!
//! - `parameters` is a JSON object. The OGAR type holds `(String, String)` pairs, so
//!   a string value is kept verbatim and any other JSON value is kept as its
//!   compact JSON text (`60000` becomes `"60000"`). Encoding writes every value as
//!   a string. A key repeated in the pair list is written once, last value wins.
//!   Decoded pairs come out in key order (the JSON object has no order).
//! - `timeout` on the wire is milliseconds and maps to `timeout_millis`. Absent is
//!   `None`. A seconds value is never converted here (the invoker side converts
//!   seconds to milliseconds before it sends; see graph_conn `invoker.ex`).
//! - `scope` is not a HIRO wire field. It is not written and decodes to `None`.
//! - `handler` is not read by graph_conn's handler; a frame without it decodes with
//!   an empty `handler`.
//! - An `acknowledged` frame carries `code` and `message` only sometimes (the
//!   gateway's ack of a submit carries neither), so it has its own type, [`AckFrame`].
//! - `sendActionResult.result` must be present. A string is kept verbatim. The
//!   invoker side of graph_conn also takes an object or a number there; such a value
//!   is kept as its compact JSON text. A missing or `null` result is refused
//!   ([`WireError::MissingField`]) rather than invented.

use serde_json::{Map, Value};

use ogar_from_schema::action_ws::{
    Acknowledged, InboundError, NegativeAcknowledged, SendActionResult, SubmitAction,
};

/// Upper bound of a usable `retryAfterMs`, one day (graph_conn `retry_after.ex`).
pub const MAX_RETRY_AFTER_MS: u64 = 86_400_000;

/// The frame limit a server is assumed to accept when its `hello` advertised
/// none: `1_000_000` bytes (graph_conn `connection_manager.ex:61`, `:201-221`;
/// `README.md:174-182`).
pub const DEFAULT_MAX_FRAME_BYTES: u64 = 1_000_000;

/// The `action_status` of a result replaced because its frame was too big
/// (graph_conn `result_frame.ex:7`).
pub const TOO_BIG_STATUS: u32 = 59;

/// The size in bytes of `frame` as [`encode`] writes it, which is what a server's
/// frame limit counts.
#[must_use]
pub fn frame_bytes(frame: &ActionWsFrame) -> u64 {
    encode(frame).len() as u64
}

/// Keeps a frame within `max_frame_bytes`: graph_conn `ResultFrame.fit/2`
/// (`result_frame.ex:15-38`; Elixir also logs a warning, which this crate does not: it does no logging).
///
/// A [`ActionWsFrame::SendActionResult`] whose encoded frame is larger than the
/// limit comes back with its `result` replaced by
/// `{"error":{"req_id":ID,"action_status":59,"action_error":"Response was too big
/// (X.XX MB, limit Y.YY MB)"}}`, where the sizes are bytes over 1,000,000 with two
/// decimals. A frame exactly at the limit is left alone
/// (`result_frame_test.exs:25-30`). **Any other frame is returned unchanged,
/// whatever its size** (`result_frame.ex:24-25`, `result_frame_test.exs:58-62`).
///
/// The replacement is not fitted again. Elixir has the same shape: the
/// replacement fits whenever the limit leaves room for it (about 150 bytes and
/// the id; the test at `result_frame_test.exs:47-56` shows a limit of 1000), and
/// below that the server's limit is smaller than any error this can send.
#[must_use]
pub fn fit(frame: ActionWsFrame, max_frame_bytes: u64) -> ActionWsFrame {
    fit_checked(frame, max_frame_bytes).0
}

/// [`fit`], and whether the result was replaced.
pub(crate) fn fit_checked(frame: ActionWsFrame, max_frame_bytes: u64) -> (ActionWsFrame, bool) {
    let ActionWsFrame::SendActionResult(result) = &frame else {
        return (frame, false);
    };
    let bytes = frame_bytes(&frame);
    if bytes <= max_frame_bytes {
        return (frame, false);
    }
    let message = format!(
        "Response was too big ({} MB, limit {} MB)",
        megabytes(bytes),
        megabytes(max_frame_bytes)
    );
    let replaced = SendActionResult {
        id: result.id.clone(),
        result: crate::submit_state::error_result(&result.id, TOO_BIG_STATUS, &message),
    };
    (replaced.into(), true)
}

/// `bytes / 1_000_000` with two decimals (`result_frame.ex:41-45`).
#[allow(clippy::cast_precision_loss)]
fn megabytes(bytes: u64) -> String {
    format!("{:.2}", bytes as f64 / 1_000_000.0)
}

/// Why a frame could not be decoded. Every variant names what was wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WireError {
    /// The text is not valid JSON; carries the parser's message.
    NotJson(String),
    /// The JSON value is not an object.
    NotAnObject,
    /// An object with no `type` and no typeless gateway refusal in it.
    MissingType,
    /// `type` is present but not a string.
    TypeNotAString,
    /// `type` is a string no HIRO action-ws frame uses; carries it.
    UnknownType(String),
    /// A typeless frame whose `error` object is neither a 429 nor a 503; carries
    /// the `code` when it was an integer.
    UnknownTypelessError(Option<i64>),
    /// A required field is absent (or `null` where a value is required).
    MissingField {
        /// The frame it was missing from.
        frame: &'static str,
        /// The field name.
        field: &'static str,
    },
    /// A field is present with the wrong JSON type or out of range.
    WrongType {
        /// The frame it was found in.
        frame: &'static str,
        /// The field name.
        field: &'static str,
        /// What was expected.
        expected: &'static str,
    },
}

impl core::fmt::Display for WireError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotJson(e) => write!(f, "frame is not JSON: {e}"),
            Self::NotAnObject => write!(f, "frame is not a JSON object"),
            Self::MissingType => write!(f, "frame has no `type` and is not a gateway refusal"),
            Self::TypeNotAString => write!(f, "frame `type` is not a string"),
            Self::UnknownType(t) => write!(f, "unknown frame type `{t}`"),
            Self::UnknownTypelessError(Some(c)) => {
                write!(f, "typeless error frame with unsupported code {c}")
            }
            Self::UnknownTypelessError(None) => {
                write!(f, "typeless error frame with no integer code")
            }
            Self::MissingField { frame, field } => {
                write!(f, "`{frame}` frame is missing `{field}`")
            }
            Self::WrongType {
                frame,
                field,
                expected,
            } => write!(f, "`{frame}` frame field `{field}` is not {expected}"),
        }
    }
}

impl std::error::Error for WireError {}

/// An `acknowledged` frame as it travels: `code` and `message` are optional on the
/// wire. The handler's own ack is `{type, id, code: 200, message: ""}` (graph_conn
/// `handler.ex`); the invoker's result ack is `{type, id, code: 200}`; the
/// gateway's ack of a submit may carry only `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckFrame {
    /// The id of the message being acknowledged.
    pub id: String,
    /// Status code, when the sender gave one.
    pub code: Option<u16>,
    /// Note, when the sender gave one.
    pub message: Option<String>,
}

impl From<Acknowledged> for AckFrame {
    fn from(a: Acknowledged) -> Self {
        Self {
            id: a.id,
            code: Some(a.code),
            message: Some(a.message),
        }
    }
}

impl AckFrame {
    /// The typed [`Acknowledged`], when the frame carried a `code`. A missing
    /// `message` becomes the empty string; a missing `code` yields `None` (no
    /// default is invented).
    #[must_use]
    pub fn into_acknowledged(self) -> Option<Acknowledged> {
        self.code.map(|code| Acknowledged {
            id: self.id,
            code,
            message: self.message.unwrap_or_default(),
        })
    }
}

/// The gateway's `hello`, sent once on connect.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hello {
    /// The largest frame the gateway accepts. Only a positive integer counts;
    /// anything else decodes to `None` (graph_conn `ws_connection.ex`).
    pub max_frame_bytes: Option<u64>,
}

/// The client's optional `clientHello`, sent once after each `hello`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClientHello {
    /// `client.app`.
    pub app: Option<String>,
    /// `client.version`.
    pub version: Option<String>,
    /// `settings.redelivery`.
    pub redelivery: Option<bool>,
}

/// The gateway's typeless in-band 429: a frame was rate-limited.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RateLimited {
    /// The id of the denied frame; absent or `null` on the wire is `None`.
    pub id: Option<String>,
    /// `error.message`, when present.
    pub message: Option<String>,
    /// `error.retryAfterMs`: an integer as sent, or a float of at least 1
    /// truncated. Any other shape is `None`. Use [`RateLimited::retry_after_ms`]
    /// for the usable wait.
    pub retry_after_ms: Option<i64>,
}

impl RateLimited {
    /// The wait a caller can act on, in `0..=`[`MAX_RETRY_AFTER_MS`]: a positive
    /// advertised wait capped at a day, otherwise `0` (graph_conn
    /// `RetryAfter.from_ms`).
    #[must_use]
    pub fn retry_after_ms(&self) -> u64 {
        match self.retry_after_ms {
            Some(n) if n > 0 => u64::try_from(n)
                .unwrap_or(MAX_RETRY_AFTER_MS)
                .min(MAX_RETRY_AFTER_MS),
            _ => 0,
        }
    }
}

/// The gateway's typeless in-band 503: the upstream service is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpstreamUnavailable {
    /// The id of the refused frame; absent or `null` on the wire is `None`.
    pub id: Option<String>,
    /// `error.message`, when present.
    pub message: Option<String>,
    /// `error.upstream`, when present.
    pub upstream: Option<String>,
}

/// One decoded `action-ws` frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionWsFrame {
    /// `hello` (gateway to client).
    Hello(Hello),
    /// `clientHello` (client to gateway).
    ClientHello(ClientHello),
    /// `submitAction`.
    SubmitAction(SubmitAction),
    /// `acknowledged`.
    Acknowledged(AckFrame),
    /// `negativeAcknowledged`.
    NegativeAcknowledged(NegativeAcknowledged),
    /// `sendActionResult`.
    SendActionResult(SendActionResult),
    /// `configChanged`.
    ConfigChanged,
    /// A typed `error`.
    Error(InboundError),
    /// The typeless in-band 429.
    RateLimited(RateLimited),
    /// The typeless in-band 503.
    UpstreamUnavailable(UpstreamUnavailable),
}

impl From<SubmitAction> for ActionWsFrame {
    fn from(m: SubmitAction) -> Self {
        Self::SubmitAction(m)
    }
}
impl From<Acknowledged> for ActionWsFrame {
    fn from(m: Acknowledged) -> Self {
        Self::Acknowledged(m.into())
    }
}
impl From<AckFrame> for ActionWsFrame {
    fn from(m: AckFrame) -> Self {
        Self::Acknowledged(m)
    }
}
impl From<NegativeAcknowledged> for ActionWsFrame {
    fn from(m: NegativeAcknowledged) -> Self {
        Self::NegativeAcknowledged(m)
    }
}
impl From<SendActionResult> for ActionWsFrame {
    fn from(m: SendActionResult) -> Self {
        Self::SendActionResult(m)
    }
}
impl From<InboundError> for ActionWsFrame {
    fn from(m: InboundError) -> Self {
        Self::Error(m)
    }
}

// ───────────────────────────────────────────────────────────── encode ──

/// Encode a frame as the JSON text HIRO's clients send.
#[must_use]
pub fn encode(frame: &ActionWsFrame) -> String {
    frame_to_value(frame).to_string()
}

fn typed(kind: &str) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("type".to_owned(), Value::from(kind));
    m
}

fn frame_to_value(frame: &ActionWsFrame) -> Value {
    match frame {
        ActionWsFrame::Hello(h) => {
            let mut m = typed("hello");
            if let Some(n) = h.max_frame_bytes {
                m.insert("max_frame_bytes".to_owned(), Value::from(n));
            }
            Value::Object(m)
        }
        ActionWsFrame::ClientHello(c) => {
            let mut m = typed("clientHello");
            let mut client = Map::new();
            if let Some(app) = &c.app {
                client.insert("app".to_owned(), Value::from(app.as_str()));
            }
            if let Some(v) = &c.version {
                client.insert("version".to_owned(), Value::from(v.as_str()));
            }
            if !client.is_empty() {
                m.insert("client".to_owned(), Value::Object(client));
            }
            if let Some(r) = c.redelivery {
                let mut settings = Map::new();
                settings.insert("redelivery".to_owned(), Value::from(r));
                m.insert("settings".to_owned(), Value::Object(settings));
            }
            Value::Object(m)
        }
        ActionWsFrame::SubmitAction(s) => {
            let mut m = typed("submitAction");
            m.insert("id".to_owned(), Value::from(s.id.as_str()));
            m.insert("handler".to_owned(), Value::from(s.handler.as_str()));
            m.insert("capability".to_owned(), Value::from(s.capability.as_str()));
            let mut params = Map::new();
            for (k, v) in &s.parameters {
                params.insert(k.clone(), Value::from(v.as_str()));
            }
            m.insert("parameters".to_owned(), Value::Object(params));
            if let Some(t) = s.timeout_millis {
                m.insert("timeout".to_owned(), Value::from(t));
            }
            Value::Object(m)
        }
        ActionWsFrame::Acknowledged(a) => {
            let mut m = typed("acknowledged");
            m.insert("id".to_owned(), Value::from(a.id.as_str()));
            if let Some(c) = a.code {
                m.insert("code".to_owned(), Value::from(c));
            }
            if let Some(msg) = &a.message {
                m.insert("message".to_owned(), Value::from(msg.as_str()));
            }
            Value::Object(m)
        }
        ActionWsFrame::NegativeAcknowledged(n) => {
            let mut m = typed("negativeAcknowledged");
            m.insert("id".to_owned(), Value::from(n.id.as_str()));
            m.insert("code".to_owned(), Value::from(n.code));
            m.insert("message".to_owned(), Value::from(n.message.as_str()));
            Value::Object(m)
        }
        ActionWsFrame::SendActionResult(r) => {
            let mut m = typed("sendActionResult");
            m.insert("id".to_owned(), Value::from(r.id.as_str()));
            m.insert("result".to_owned(), Value::from(r.result.as_str()));
            Value::Object(m)
        }
        ActionWsFrame::ConfigChanged => Value::Object(typed("configChanged")),
        ActionWsFrame::Error(e) => {
            let mut m = typed("error");
            m.insert("code".to_owned(), Value::from(e.code));
            m.insert("message".to_owned(), Value::from(e.message.as_str()));
            Value::Object(m)
        }
        ActionWsFrame::RateLimited(r) => {
            let mut err = Map::new();
            err.insert("code".to_owned(), Value::from(429));
            if let Some(msg) = &r.message {
                err.insert("message".to_owned(), Value::from(msg.as_str()));
            }
            if let Some(ms) = r.retry_after_ms {
                err.insert("retryAfterMs".to_owned(), Value::from(ms));
            }
            typeless(err, r.id.as_deref())
        }
        ActionWsFrame::UpstreamUnavailable(u) => {
            let mut err = Map::new();
            err.insert("code".to_owned(), Value::from(503));
            if let Some(msg) = &u.message {
                err.insert("message".to_owned(), Value::from(msg.as_str()));
            }
            if let Some(up) = &u.upstream {
                err.insert("upstream".to_owned(), Value::from(up.as_str()));
            }
            typeless(err, u.id.as_deref())
        }
    }
}

fn typeless(error: Map<String, Value>, id: Option<&str>) -> Value {
    let mut m = Map::new();
    m.insert("error".to_owned(), Value::Object(error));
    if let Some(id) = id {
        m.insert("id".to_owned(), Value::from(id));
    }
    Value::Object(m)
}

// ───────────────────────────────────────────────────────────── decode ──

/// Decode one JSON text frame.
///
/// # Errors
///
/// A [`WireError`] naming why the frame is refused. See the module docs.
pub fn decode(text: &str) -> Result<ActionWsFrame, WireError> {
    let value: Value = serde_json::from_str(text).map_err(|e| WireError::NotJson(e.to_string()))?;
    let Value::Object(obj) = value else {
        return Err(WireError::NotAnObject);
    };
    match obj.get("type") {
        None => decode_typeless(&obj),
        Some(Value::String(t)) => decode_typed(t, &obj),
        Some(_) => Err(WireError::TypeNotAString),
    }
}

fn decode_typed(kind: &str, obj: &Map<String, Value>) -> Result<ActionWsFrame, WireError> {
    match kind {
        "hello" => Ok(ActionWsFrame::Hello(Hello {
            max_frame_bytes: obj
                .get("max_frame_bytes")
                .and_then(Value::as_u64)
                .filter(|n| *n > 0),
        })),
        "clientHello" => decode_client_hello(obj),
        "submitAction" => decode_submit(obj),
        "acknowledged" => Ok(ActionWsFrame::Acknowledged(AckFrame {
            id: req_string(obj, "acknowledged", "id")?,
            code: opt_u16(obj, "acknowledged", "code")?,
            message: opt_string(obj, "acknowledged", "message")?,
        })),
        "negativeAcknowledged" => Ok(ActionWsFrame::NegativeAcknowledged(NegativeAcknowledged {
            id: req_string(obj, "negativeAcknowledged", "id")?,
            code: req_u16(obj, "negativeAcknowledged", "code")?,
            message: req_string(obj, "negativeAcknowledged", "message")?,
        })),
        "sendActionResult" => decode_result(obj),
        "configChanged" => Ok(ActionWsFrame::ConfigChanged),
        "error" => Ok(ActionWsFrame::Error(InboundError {
            code: req_u16(obj, "error", "code")?,
            message: req_string(obj, "error", "message")?,
        })),
        other => Err(WireError::UnknownType(other.to_owned())),
    }
}

fn decode_client_hello(obj: &Map<String, Value>) -> Result<ActionWsFrame, WireError> {
    const F: &str = "clientHello";
    let mut hello = ClientHello::default();
    match obj.get("client") {
        None | Some(Value::Null) => {}
        Some(Value::Object(c)) => {
            hello.app = opt_string(c, F, "app")?;
            hello.version = opt_string(c, F, "version")?;
        }
        Some(_) => {
            return Err(WireError::WrongType {
                frame: F,
                field: "client",
                expected: "an object",
            });
        }
    }
    match obj.get("settings") {
        None | Some(Value::Null) => {}
        Some(Value::Object(s)) => match s.get("redelivery") {
            None | Some(Value::Null) => {}
            Some(Value::Bool(b)) => hello.redelivery = Some(*b),
            Some(_) => {
                return Err(WireError::WrongType {
                    frame: F,
                    field: "settings.redelivery",
                    expected: "a boolean",
                });
            }
        },
        Some(_) => {
            return Err(WireError::WrongType {
                frame: F,
                field: "settings",
                expected: "an object",
            });
        }
    }
    Ok(ActionWsFrame::ClientHello(hello))
}

fn decode_submit(obj: &Map<String, Value>) -> Result<ActionWsFrame, WireError> {
    const F: &str = "submitAction";
    let id = req_string(obj, F, "id")?;
    let capability = req_string(obj, F, "capability")?;
    let handler = opt_string(obj, F, "handler")?.unwrap_or_default();
    let parameters = match obj.get("parameters") {
        None | Some(Value::Null) => {
            return Err(WireError::MissingField {
                frame: F,
                field: "parameters",
            });
        }
        Some(Value::Object(p)) => p
            .iter()
            .map(|(k, v)| (k.clone(), value_text(v)))
            .collect::<Vec<_>>(),
        Some(_) => {
            return Err(WireError::WrongType {
                frame: F,
                field: "parameters",
                expected: "an object",
            });
        }
    };
    let timeout_millis = match obj.get("timeout") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_i64().ok_or(WireError::WrongType {
            frame: F,
            field: "timeout",
            expected: "an integer (milliseconds)",
        })?),
    };
    Ok(ActionWsFrame::SubmitAction(SubmitAction {
        id,
        capability,
        handler,
        scope: None,
        parameters,
        timeout_millis,
    }))
}

fn decode_result(obj: &Map<String, Value>) -> Result<ActionWsFrame, WireError> {
    const F: &str = "sendActionResult";
    let id = req_string(obj, F, "id")?;
    let result = match obj.get("result") {
        None | Some(Value::Null) => {
            return Err(WireError::MissingField {
                frame: F,
                field: "result",
            });
        }
        Some(v) => value_text(v),
    };
    Ok(ActionWsFrame::SendActionResult(SendActionResult {
        id,
        result,
    }))
}

fn decode_typeless(obj: &Map<String, Value>) -> Result<ActionWsFrame, WireError> {
    let Some(Value::Object(err)) = obj.get("error") else {
        return Err(WireError::MissingType);
    };
    // Only a string id belongs to a caller (graph_conn matches `is_binary`); an
    // absent, null or non-string id is `None`.
    let id = obj.get("id").and_then(Value::as_str).map(str::to_owned);
    let message = err
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_owned);
    match err.get("code").and_then(Value::as_i64) {
        Some(429) => {
            let retry_after_ms = match err.get("retryAfterMs") {
                Some(Value::Number(n)) => n.as_i64().or_else(|| {
                    n.as_f64()
                        .filter(|f| *f >= 1.0 && *f <= i64::MAX as f64)
                        .map(|f| f as i64)
                }),
                _ => None,
            };
            Ok(ActionWsFrame::RateLimited(RateLimited {
                id,
                message,
                retry_after_ms,
            }))
        }
        Some(503) => Ok(ActionWsFrame::UpstreamUnavailable(UpstreamUnavailable {
            id,
            message,
            upstream: err
                .get("upstream")
                .and_then(Value::as_str)
                .map(str::to_owned),
        })),
        other => Err(WireError::UnknownTypelessError(other)),
    }
}

/// A JSON value as text: a string verbatim, anything else as compact JSON.
fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn req_string(
    obj: &Map<String, Value>,
    frame: &'static str,
    field: &'static str,
) -> Result<String, WireError> {
    match obj.get(field) {
        None | Some(Value::Null) => Err(WireError::MissingField { frame, field }),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(WireError::WrongType {
            frame,
            field,
            expected: "a string",
        }),
    }
}

fn opt_string(
    obj: &Map<String, Value>,
    frame: &'static str,
    field: &'static str,
) -> Result<Option<String>, WireError> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(WireError::WrongType {
            frame,
            field,
            expected: "a string",
        }),
    }
}

fn opt_u16(
    obj: &Map<String, Value>,
    frame: &'static str,
    field: &'static str,
) -> Result<Option<u16>, WireError> {
    match obj.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .map(Some)
            .ok_or(WireError::WrongType {
                frame,
                field,
                expected: "an integer in 0..=65535",
            }),
    }
}

fn req_u16(
    obj: &Map<String, Value>,
    frame: &'static str,
    field: &'static str,
) -> Result<u16, WireError> {
    opt_u16(obj, frame, field)?.ok_or(WireError::MissingField { frame, field })
}
