//! Reading a HIRO `GET /applicabilities` body as **ON** entries ([`AppliesOn`]).
//!
//! The readable wire shape is graph_conn's (the Elixir client). In its mock,
//! `lib/graph_conn/mock.ex` (`convert_applicabilities_from_json`, stored by
//! `put_applicabilities`, read back by `get_applicabilities`), the body is keyed by
//! handler id, then by capability, then by an `on <expr>` key whose value maps a
//! parameter name to a template:
//!
//! ```json
//! {"<handler>": {"<capability>": {"on ogit/_id": {"LocalNodeID": "${ogit/_id}"}}}}
//! ```
//!
//! Two input forms are read:
//!
//! - [`parse_applies_on`]: that response shape (handler, capability, `on` key).
//! - [`parse_applies_on_definitions`]: the definition list the mock converts from,
//!   an array of objects with a `capability` and an `applicability` that is either
//!   `{"on <expr>": {param: template}}` or the list form `["on <expr>"]` (no
//!   parameters). The handler id is not in that list, so the caller supplies it.
//!
//! [`parse_applicabilities_typed`] reads either the `on` shape or the legacy
//! `ModelFilter` shape (UNCONFIRMED, see
//! [`ogar_from_schema::registration::ModelFilter`]) and says which it found.
//!
//! The `on <expr>` text is kept, not resolved: mapping it to a class id happens at
//! intake and is not built here. `${...}` templates are kept verbatim.

use serde_json::{Map, Value};

use ogar_from_schema::registration::{AppliesOn, RegisteredApplicabilities};

/// Why an applicability body was refused. Every variant names what was wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ApplicabilityError {
    /// The text is not valid JSON; carries the parser's message.
    NotJson(String),
    /// The body is not a JSON object.
    NotAnObject,
    /// A handler's value is not an object.
    HandlerNotAnObject {
        /// The handler id.
        handler: String,
    },
    /// A capability's value is not an object of `on <expr>` entries.
    CapabilityNotAnObject {
        /// The handler id.
        handler: String,
        /// The capability name.
        capability: String,
    },
    /// A key under a capability does not start with `on `.
    NotAnOnKey {
        /// The handler id.
        handler: String,
        /// The capability name.
        capability: String,
        /// The offending key.
        key: String,
    },
    /// An `on` key with nothing after the prefix.
    EmptyOnExpression {
        /// The handler id.
        handler: String,
        /// The capability name.
        capability: String,
    },
    /// The value of an `on <expr>` key is not an object of parameters.
    ParametersNotAnObject {
        /// The handler id.
        handler: String,
        /// The capability name.
        capability: String,
        /// The `on` expression.
        on: String,
    },
    /// A parameter template is not a string.
    TemplateNotAString {
        /// The handler id.
        handler: String,
        /// The capability name.
        capability: String,
        /// The `on` expression.
        on: String,
        /// The parameter name.
        parameter: String,
    },
    /// The body mixes `on` handlers and `ModelFilter` handlers.
    MixedShapes,
    /// The body looks like the legacy `ModelFilter` shape but does not read as it;
    /// carries the parser's message.
    LegacyShape(String),
    /// A definition list is not a JSON array.
    DefinitionsNotAnArray,
    /// A definition at `index` is not an object.
    DefinitionNotAnObject {
        /// Position in the array.
        index: usize,
    },
    /// A definition at `index` lacks a field.
    DefinitionMissingField {
        /// Position in the array.
        index: usize,
        /// The field name.
        field: &'static str,
    },
    /// A definition's `capability` is not a string.
    DefinitionCapabilityNotAString {
        /// Position in the array.
        index: usize,
    },
    /// A definition's `applicability` is neither `{"on <expr>": {...}}` with one
    /// entry nor `["on <expr>"]` with one string.
    DefinitionApplicabilityShape {
        /// Position in the array.
        index: usize,
    },
}

impl core::fmt::Display for ApplicabilityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotJson(e) => write!(f, "applicabilities body is not JSON: {e}"),
            Self::NotAnObject => write!(f, "applicabilities body is not a JSON object"),
            Self::HandlerNotAnObject { handler } => {
                write!(f, "handler `{handler}` is not an object")
            }
            Self::CapabilityNotAnObject {
                handler,
                capability,
            } => write!(
                f,
                "capability `{capability}` of handler `{handler}` is not an object"
            ),
            Self::NotAnOnKey {
                handler,
                capability,
                key,
            } => write!(
                f,
                "key `{key}` under `{handler}`/`{capability}` does not start with `on `"
            ),
            Self::EmptyOnExpression {
                handler,
                capability,
            } => write!(f, "empty `on` expression under `{handler}`/`{capability}`"),
            Self::ParametersNotAnObject {
                handler,
                capability,
                on,
            } => write!(
                f,
                "parameters of `on {on}` under `{handler}`/`{capability}` are not an object"
            ),
            Self::TemplateNotAString {
                handler,
                capability,
                on,
                parameter,
            } => write!(
                f,
                "parameter `{parameter}` of `on {on}` under `{handler}`/`{capability}` is not a string"
            ),
            Self::MixedShapes => write!(
                f,
                "applicabilities body mixes the `on` shape and the legacy ModelFilter shape"
            ),
            Self::LegacyShape(e) => write!(f, "legacy ModelFilter shape does not parse: {e}"),
            Self::DefinitionsNotAnArray => write!(f, "definitions are not a JSON array"),
            Self::DefinitionNotAnObject { index } => {
                write!(f, "definition {index} is not an object")
            }
            Self::DefinitionMissingField { index, field } => {
                write!(f, "definition {index} has no `{field}`")
            }
            Self::DefinitionCapabilityNotAString { index } => {
                write!(f, "definition {index}: `capability` is not a string")
            }
            Self::DefinitionApplicabilityShape { index } => write!(
                f,
                "definition {index}: `applicability` is not one `on <expr>` object entry or one `on <expr>` string in a list"
            ),
        }
    }
}

impl std::error::Error for ApplicabilityError {}

/// Which shape a `GET /applicabilities` body had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applicabilities {
    /// graph_conn's shape: the ON entries, sorted by handler, capability, `on`.
    AppliesOn(Vec<AppliesOn>),
    /// The legacy `ModelFilter` shape (UNCONFIRMED), keyed by handler id.
    ModelFilter(RegisteredApplicabilities),
}

/// Keys that mark a handler value as the legacy `ModelFilter` shape.
const LEGACY_KEYS: [&str; 7] = [
    "modelFilters",
    "model",
    "filters",
    "priority",
    "Priority",
    "parameters",
    "parameter",
];

/// True when every value is an object whose keys are all `on <expr>` keys: the
/// ON shape, whatever the capabilities are called.
fn is_on_shaped(obj: &Map<String, Value>) -> bool {
    obj.values().all(|v| match v {
        Value::Object(ons) => ons.keys().all(|k| k.starts_with("on ")),
        // The list form of a parameterless capability.
        Value::Array(list) => list
            .iter()
            .all(|i| matches!(i, Value::String(k) if k.starts_with("on "))),
        _ => false,
    })
}

/// The shape is decided by structure first: a capability may be named `model`,
/// `priority` or any other legacy key, so a key name alone never selects the
/// legacy reader.
fn is_legacy_handler(obj: &Map<String, Value>) -> bool {
    !is_on_shaped(obj)
        && obj
            .keys()
            .any(|k| LEGACY_KEYS.contains(&k.as_str()) || k == "Parameter")
}

fn parse_object(json: &str) -> Result<Map<String, Value>, ApplicabilityError> {
    match serde_json::from_str::<Value>(json) {
        Err(e) => Err(ApplicabilityError::NotJson(e.to_string())),
        Ok(Value::Object(o)) => Ok(o),
        Ok(_) => Err(ApplicabilityError::NotAnObject),
    }
}

/// Read a `GET /applicabilities` body in graph_conn's shape (handler id,
/// capability, `on <expr>` key, parameter templates) into [`AppliesOn`] entries,
/// sorted by handler, capability, `on` expression.
///
/// An empty handler (`{"action_handler": {}}`, graph_conn's default) yields no
/// entries. The legacy `ModelFilter` shape is refused here, not read as empty;
/// use [`parse_applicabilities_typed`] to accept either.
///
/// # Errors
///
/// An [`ApplicabilityError`] naming the first malformed part.
pub fn parse_applies_on(json: &str) -> Result<Vec<AppliesOn>, ApplicabilityError> {
    applies_on_from_object(&parse_object(json)?)
}

fn applies_on_from_object(body: &Map<String, Value>) -> Result<Vec<AppliesOn>, ApplicabilityError> {
    let mut rows = Vec::new();
    for (handler, hv) in body {
        let Value::Object(caps) = hv else {
            return Err(ApplicabilityError::HandlerNotAnObject {
                handler: handler.clone(),
            });
        };
        for (capability, cv) in caps {
            let on_of = |key: &str| {
                on_expression(key).map_err(|e| match e {
                    OnKeyError::NoPrefix => ApplicabilityError::NotAnOnKey {
                        handler: handler.clone(),
                        capability: capability.clone(),
                        key: key.to_owned(),
                    },
                    OnKeyError::Empty => ApplicabilityError::EmptyOnExpression {
                        handler: handler.clone(),
                        capability: capability.clone(),
                    },
                })
            };
            // The list form: a capability with no parameters is a list of its
            // `on <expr>` strings (HIRO's public Action API page, the
            // `ParameterlessExecute` entry of the applicabilities example).
            if let Value::Array(list) = cv {
                for item in list {
                    let Value::String(key) = item else {
                        return Err(ApplicabilityError::CapabilityNotAnObject {
                            handler: handler.clone(),
                            capability: capability.clone(),
                        });
                    };
                    rows.push(AppliesOn {
                        handler: handler.clone(),
                        capability: capability.clone(),
                        on: on_of(key)?,
                        parameters: Vec::new(),
                    });
                }
                continue;
            }
            let Value::Object(ons) = cv else {
                return Err(ApplicabilityError::CapabilityNotAnObject {
                    handler: handler.clone(),
                    capability: capability.clone(),
                });
            };
            for (key, pv) in ons {
                let on = on_of(key)?;
                let Value::Object(params) = pv else {
                    return Err(ApplicabilityError::ParametersNotAnObject {
                        handler: handler.clone(),
                        capability: capability.clone(),
                        on,
                    });
                };
                let mut parameters = Vec::with_capacity(params.len());
                for (name, tv) in params {
                    let Value::String(template) = tv else {
                        return Err(ApplicabilityError::TemplateNotAString {
                            handler: handler.clone(),
                            capability: capability.clone(),
                            on,
                            parameter: name.clone(),
                        });
                    };
                    parameters.push((name.clone(), template.clone()));
                }
                rows.push(AppliesOn {
                    handler: handler.clone(),
                    capability: capability.clone(),
                    on,
                    parameters,
                });
            }
        }
    }
    Ok(rows)
}

enum OnKeyError {
    NoPrefix,
    Empty,
}

/// `on ogit/_id` to `ogit/_id`.
fn on_expression(key: &str) -> Result<String, OnKeyError> {
    let rest = key.strip_prefix("on ").ok_or(OnKeyError::NoPrefix)?;
    let rest = rest.trim();
    if rest.is_empty() {
        Err(OnKeyError::Empty)
    } else {
        Ok(rest.to_owned())
    }
}

/// Read the **definition list** form (the input graph_conn's mock converts): a JSON
/// array of objects, each with a `capability` and an `applicability`, which is
/// either `{"on <expr>": {param: template}}` (exactly one entry) or the list form
/// `["on <expr>"]` (exactly one string, no parameters). Other keys of a definition
/// (`name`, `implementation`, `exec`, `tempfiles`) are not applicability data and
/// are ignored.
///
/// `handler` is the handler id to register the entries under (the list itself does
/// not carry it; graph_conn's `put_applicabilities` takes it as an argument).
/// Entries come out in input order; a later entry for the same capability and `on`
/// expression replaces the earlier one in place, as the mock's map does.
///
/// # Errors
///
/// An [`ApplicabilityError`] naming the first malformed definition.
pub fn parse_applies_on_definitions(
    handler: &str,
    json: &str,
) -> Result<Vec<AppliesOn>, ApplicabilityError> {
    let value: Value =
        serde_json::from_str(json).map_err(|e| ApplicabilityError::NotJson(e.to_string()))?;
    let Value::Array(defs) = value else {
        return Err(ApplicabilityError::DefinitionsNotAnArray);
    };
    let mut rows: Vec<AppliesOn> = Vec::new();
    for (index, def) in defs.iter().enumerate() {
        let Value::Object(d) = def else {
            return Err(ApplicabilityError::DefinitionNotAnObject { index });
        };
        let capability = match d.get("capability") {
            None => {
                return Err(ApplicabilityError::DefinitionMissingField {
                    index,
                    field: "capability",
                });
            }
            Some(Value::String(s)) => s.clone(),
            Some(_) => return Err(ApplicabilityError::DefinitionCapabilityNotAString { index }),
        };
        let applicability =
            d.get("applicability")
                .ok_or(ApplicabilityError::DefinitionMissingField {
                    index,
                    field: "applicability",
                })?;
        let shape_err = || ApplicabilityError::DefinitionApplicabilityShape { index };
        let (key, params): (String, Option<&Map<String, Value>>) = match applicability {
            Value::Object(o) if o.len() == 1 => {
                let (k, v) = o.iter().next().ok_or_else(shape_err)?;
                match v {
                    Value::Object(p) => (k.clone(), Some(p)),
                    _ => {
                        return Err(ApplicabilityError::ParametersNotAnObject {
                            handler: handler.to_owned(),
                            capability,
                            on: k.strip_prefix("on ").unwrap_or(k).trim().to_owned(),
                        });
                    }
                }
            }
            Value::Array(a) if a.len() == 1 => match &a[0] {
                Value::String(k) => (k.clone(), None),
                _ => return Err(shape_err()),
            },
            _ => return Err(shape_err()),
        };
        let on = on_expression(&key).map_err(|e| match e {
            OnKeyError::NoPrefix => ApplicabilityError::NotAnOnKey {
                handler: handler.to_owned(),
                capability: capability.clone(),
                key: key.clone(),
            },
            OnKeyError::Empty => ApplicabilityError::EmptyOnExpression {
                handler: handler.to_owned(),
                capability: capability.clone(),
            },
        })?;
        let mut parameters = Vec::new();
        if let Some(p) = params {
            for (name, tv) in p {
                let Value::String(template) = tv else {
                    return Err(ApplicabilityError::TemplateNotAString {
                        handler: handler.to_owned(),
                        capability,
                        on,
                        parameter: name.clone(),
                    });
                };
                parameters.push((name.clone(), template.clone()));
            }
        }
        let row = AppliesOn {
            handler: handler.to_owned(),
            capability,
            on,
            parameters,
        };
        match rows
            .iter_mut()
            .find(|r| r.capability == row.capability && r.on == row.on)
        {
            Some(existing) => *existing = row,
            None => rows.push(row),
        }
    }
    Ok(rows)
}

/// Read a `GET /applicabilities` body in either shape and say which it was.
///
/// A handler value with any of `modelFilters`/`model`/`filters`/`priority`/
/// `Priority`/`parameters`/`parameter`/`Parameter` is the legacy `ModelFilter`
/// shape; a handler value that is empty reads as either. A body whose handlers are
/// all `on`-shaped or empty is [`Applicabilities::AppliesOn`] (an all-empty body is
/// an empty `AppliesOn`).
///
/// # Errors
///
/// An [`ApplicabilityError`]: [`ApplicabilityError::MixedShapes`] when both shapes
/// appear, otherwise the error of the shape found.
pub fn parse_applicabilities_typed(json: &str) -> Result<Applicabilities, ApplicabilityError> {
    let body = parse_object(json)?;
    let mut legacy = 0usize;
    let mut other = 0usize;
    for (handler, hv) in &body {
        let Value::Object(o) = hv else {
            return Err(ApplicabilityError::HandlerNotAnObject {
                handler: handler.clone(),
            });
        };
        if o.is_empty() {
            continue;
        }
        if is_legacy_handler(o) {
            legacy += 1;
        } else {
            other += 1;
        }
    }
    if legacy > 0 && other > 0 {
        return Err(ApplicabilityError::MixedShapes);
    }
    if legacy > 0 {
        let apps: RegisteredApplicabilities = serde_json::from_value(Value::Object(body))
            .map_err(|e| ApplicabilityError::LegacyShape(e.to_string()))?;
        return Ok(Applicabilities::ModelFilter(apps));
    }
    Ok(Applicabilities::AppliesOn(applies_on_from_object(&body)?))
}
