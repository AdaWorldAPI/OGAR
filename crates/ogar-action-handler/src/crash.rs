//! The text of a crashed action, without the values it crashed on.
//!
//! The reference is graph_conn `lib/action_api/crash.ex` (`Crash.banner/2`), with
//! its test `test/action_api/crash_test.exs`, and `lib/action_api/handler.ex`,
//! which wraps the banner as `"Execution failed: #{banner}"` into an
//! `action_status` 54 result (`handler.ex:309`, `:327`, `:356-357`). The banner is
//! written for text that travels to the Graph and the invoker, so it never carries
//! a value the capability crashed on, because that can be a secret
//! (`crash.ex:1`, `crash.ex:5`).
//!
//! Which part of a crash is kept (`crash.ex:7-25`):
//!
//! | crash | banner | HIRO source |
//! |---|---|---|
//! | a `RuntimeError` | `** (RuntimeError) <message>`: the message is kept, because the capability wrote it itself | `crash.ex:7-8` |
//! | any other exception | `** (<Module>)`: the module only | `crash.ex:10-11` |
//! | an exit or throw whose reason is a string or an atom | `** (<kind>) <inspect(reason)>` | `crash.ex:21-22` |
//! | an exit or throw with any other reason | `** (<kind>)` | `crash.ex:24-25` |
//!
//! # What a Rust execution maps to
//!
//! HIRO's cases are Elixir's. The two failures of a Rust executor map as follows;
//! both are choices, because the Elixir source has no Rust.
//!
//! * An executor that returns `Err(message)` is a [`Crash::RuntimeError`]: a
//!   failure whose message the capability wrote itself, and which HIRO therefore
//!   keeps. The result text is `Execution failed: ** (RuntimeError) <message>`.
//! * An executor that panics is [`Crash::panicked`], an [`Crash::Exit`] with
//!   [`Reason::Other`]: the text is `Execution failed: ** (exit)`. The panic
//!   message is dropped, because it is not known to be one the capability wrote
//!   itself: a panic raised by the standard library (`unwrap` on an `Err`, an
//!   index out of bounds) quotes the value it failed on. This is the banner-only
//!   case of `crash.ex:24-25`, the one `crash_test.exs:28-30` pins for an exit
//!   reason that is neither a string nor an atom.
//!
//! `Jason.EncodeError` (`handler.ex:325-327`) has no Rust counterpart: a result is
//! a list of string pairs and always encodes.

/// The `action_status` of a result whose execution crashed (`handler.ex:309`,
/// `:327`, `:357`).
pub const CRASH_STATUS: u32 = 54;

/// What the `action_error` of a crash starts with (`handler.ex:309`, `:327`,
/// `:357`).
pub const FAILURE_PREFIX: &str = "Execution failed: ";

/// The reason of an exit or throw.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reason {
    /// An atom, such as `killed`: given as its name, without the colon.
    Atom(String),
    /// A string, such as `"capability exited"`.
    Text(String),
    /// Any other term, such as a tuple or a map. It is left out of the banner.
    Other,
}

/// A crashed execution, as far as its banner needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Crash {
    /// A `RuntimeError` with the message the capability gave it.
    RuntimeError(String),
    /// Any other exception, named by its module, for instance `KeyError`.
    Exception(String),
    /// An exit.
    Exit(Reason),
    /// A throw.
    Throw(Reason),
}

impl Crash {
    /// The crash of an executor that panicked: [`Crash::Exit`] with no reason
    /// kept. See the module documentation for why the panic message is dropped.
    #[must_use]
    pub fn panicked() -> Self {
        Self::Exit(Reason::Other)
    }

    /// The banner of this crash (`Crash.banner/2`, `crash.ex:7-25`).
    #[must_use]
    pub fn banner(&self) -> String {
        match self {
            Self::RuntimeError(message) => format!("** (RuntimeError) {message}"),
            Self::Exception(module) => format!("** ({module})"),
            Self::Exit(reason) => kind_banner("exit", reason),
            Self::Throw(reason) => kind_banner("throw", reason),
        }
    }

    /// The `action_error` of the result: [`FAILURE_PREFIX`] and the banner.
    #[must_use]
    pub fn failure_message(&self) -> String {
        format!("{FAILURE_PREFIX}{}", self.banner())
    }
}

fn kind_banner(kind: &str, reason: &Reason) -> String {
    match reason {
        Reason::Atom(name) => format!("** ({kind}) {}", inspect_atom(name)),
        Reason::Text(text) => format!("** ({kind}) {}", inspect_text(text)),
        Reason::Other => format!("** ({kind})"),
    }
}

/// `inspect/1` of an atom: `nil`, `true` and `false` bare, a plain identifier as
/// `:name`, anything else as `:"name"`. Atoms that Elixir writes bare with
/// non-ASCII letters, or as operators (`:+`), come out quoted here.
fn inspect_atom(name: &str) -> String {
    if matches!(name, "nil" | "true" | "false") {
        return name.to_owned();
    }
    let mut chars = name.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    let rest = name.strip_suffix(['?', '!']).unwrap_or(name);
    let plain = starts
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '@');
    if plain {
        format!(":{name}")
    } else {
        format!(":{}", inspect_text(name))
    }
}

/// `inspect/1` of a string: double quotes, with `"`, `\`, newline, return, tab
/// and `#{` escaped. Other control characters are written as `\xNN`; that last
/// rule is not pinned by a HIRO test.
fn inspect_text(text: &str) -> String {
    let mut out = String::from("\"");
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '#' if chars.peek() == Some(&'{') => out.push_str("\\#"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
