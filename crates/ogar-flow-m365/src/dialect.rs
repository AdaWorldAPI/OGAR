//! The flow dialect: what each flow call byte does when the `ogar-loco`
//! [`Interpreter`](ogar_loco::Interpreter) reaches it.
//!
//! The interpreter owns the program counter and control flow; this dialect
//! owns every value. It refuses every shared-core byte it is handed (flow
//! bodies use none), and it checks the [`ReportScope`] before each domain
//! call, so an operation the actor may not perform fails before any fact is
//! read.

use std::collections::BTreeMap;

use lance_graph_contract::class_view::FieldMask;
use ogar_loco::{Dialect, FnIndex};

use crate::adjacency::{Capability, operation_for_fn};
use crate::host::{DirectoryObservations, FieldValue, ReportScope, SourceStamp};
use crate::report::{Report, ReportRow};
use crate::vocab::{self, Comparison, ExportFormat, ReportField};

/// One row in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    key: String,
    fields: BTreeMap<ReportField, FieldValue>,
}

/// The dialect's stack value: a population of rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Population {
    rows: Vec<Row>,
}

impl Population {
    /// How many rows the population holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the population is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Every field any row currently holds — what this population has
    /// materialized, whatever is later selected or exported.
    #[must_use]
    pub fn loaded_fields(&self) -> FieldMask {
        self.rows
            .iter()
            .flat_map(|r| r.fields.keys())
            .fold(FieldMask::EMPTY, |m, k| m.with(k.position()))
    }
}

/// Why a flow call failed. Every variant stops the run before export, so a
/// failed run produces no report and no partial output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowError {
    /// The actor has no report scope at all.
    NoScope,
    /// The scope does not grant the capability this operation needs.
    CapabilityDenied {
        /// The operation.
        operation: &'static str,
        /// The capability it needs.
        capability: Capability,
    },
    /// A field the scope does not permit was filtered on or selected.
    FieldDenied {
        /// The field.
        field: ReportField,
    },
    /// A field was filtered on or selected before any stage loaded it.
    FieldNotLoaded {
        /// The field.
        field: ReportField,
    },
    /// A call byte this dialect does not execute (any shared-core byte, or a
    /// domain byte outside the flow vocabulary).
    UnsupportedCall {
        /// The byte.
        call: FnIndex,
    },
    /// A value byte that does not decode under its codebook.
    MalformedCall {
        /// The call.
        call: FnIndex,
    },
    /// The stack did not hold a population where one was required.
    StackShape {
        /// The call.
        call: FnIndex,
    },
    /// The run ended without a report, or with values left on the stack.
    NoReport,
}

impl core::fmt::Display for FlowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for FlowError {}

/// The flow dialect over one host, one scope, one compiled argument table.
pub struct FlowDialect<'a, H: DirectoryObservations> {
    host: &'a H,
    scope: &'a ReportScope,
    args: &'a [String],
    requested: &'a str,
    trace: &'a [crate::frontend::TraceEntry],
    source: SourceStamp,
    report: Option<Report>,
}

impl<'a, H: DirectoryObservations> FlowDialect<'a, H> {
    /// Bind the dialect for one run.
    pub fn new(
        host: &'a H,
        scope: &'a ReportScope,
        flow: &'a crate::frontend::CompiledFlow,
    ) -> Self {
        Self {
            host,
            scope,
            args: &flow.args,
            requested: &flow.requested,
            trace: &flow.trace,
            source: host.source(),
            report: None,
        }
    }

    /// The report, once `export` has run.
    #[must_use]
    pub fn report(&self) -> Option<&Report> {
        self.report.as_ref()
    }

    /// Take the report out of the dialect.
    pub fn into_report(self) -> Option<Report> {
        self.report
    }

    fn require(&self, f: FnIndex) -> Result<(), FlowError> {
        let op = operation_for_fn(f).ok_or(FlowError::UnsupportedCall { call: f })?;
        match op.capability {
            Some(cap) if !self.scope.capabilities.has(cap) => Err(FlowError::CapabilityDenied {
                operation: op.name,
                capability: cap,
            }),
            _ => Ok(()),
        }
    }

    fn permit(&self, field: ReportField) -> Result<(), FlowError> {
        if self.scope.fields.has(field.position()) {
            Ok(())
        } else {
            Err(FlowError::FieldDenied { field })
        }
    }

    fn arg(&self, f: FnIndex, b: u8) -> Result<&'a str, FlowError> {
        self.args
            .get(usize::from(b))
            .map(String::as_str)
            .ok_or(FlowError::MalformedCall { call: f })
    }

    fn get_user(&self, f: FnIndex, v: [u8; 3]) -> Result<Population, FlowError> {
        let user_ref = self.arg(f, v[0])?;
        // Ask the host only for permitted fields; the read is projected at
        // the source.
        let fields = ReportField::USER_FIELDS.intersect(self.scope.fields);
        let rows = self
            .host
            .find_user(self.scope.tenant, user_ref, fields)
            .map(|obs| Row {
                key: obs.object_id,
                // Defence in depth: a host that returns more than it was
                // asked for does not get the extra fields into the run.
                fields: obs
                    .fields
                    .into_iter()
                    .filter(|(k, _)| fields.has(k.position()))
                    .collect(),
            })
            .into_iter()
            .collect();
        Ok(Population { rows })
    }

    fn attach(
        &self,
        mut pop: Population,
        field: ReportField,
        read: impl Fn(&str) -> Vec<String>,
    ) -> Result<Population, FlowError> {
        self.permit(field)?;
        for row in &mut pop.rows {
            let mut facts = read(&row.key);
            facts.sort();
            row.fields.insert(field, FieldValue::List(facts));
        }
        Ok(pop)
    }

    fn filter(&self, f: FnIndex, v: [u8; 3], mut pop: Population) -> Result<Population, FlowError> {
        let field = ReportField::from_position(v[0]).ok_or(FlowError::MalformedCall { call: f })?;
        let cmp = Comparison::from_byte(v[1]).ok_or(FlowError::MalformedCall { call: f })?;
        let want = self.arg(f, v[2])?;
        self.permit(field)?;
        let mut kept = Vec::with_capacity(pop.rows.len());
        for row in pop.rows {
            let value = row
                .fields
                .get(&field)
                .ok_or(FlowError::FieldNotLoaded { field })?;
            let hit = match (cmp, value) {
                (Comparison::Contains, FieldValue::List(xs)) => {
                    xs.iter().any(|x| x.eq_ignore_ascii_case(want))
                }
                (Comparison::Eq | Comparison::Ne, FieldValue::Text(s)) => {
                    s.eq_ignore_ascii_case(want) == (cmp == Comparison::Eq)
                }
                (Comparison::Eq | Comparison::Ne, FieldValue::Bool(b)) => {
                    let wb = want.eq_ignore_ascii_case("true");
                    (*b == wb) == (cmp == Comparison::Eq)
                }
                _ => return Err(FlowError::MalformedCall { call: f }),
            };
            if hit {
                kept.push(row);
            }
        }
        pop.rows = kept;
        Ok(pop)
    }

    fn project(&self, v: [u8; 3], mut pop: Population) -> Result<Population, FlowError> {
        let mask = vocab::unpack_mask(v);
        for p in 0..24u8 {
            if mask.has(p)
                && let Some(field) = ReportField::from_position(p)
            {
                self.permit(field)?;
            }
        }
        for row in &mut pop.rows {
            for p in 0..24u8 {
                if mask.has(p)
                    && let Some(field) = ReportField::from_position(p)
                    && field != ReportField::ObjectId
                    && !row.fields.contains_key(&field)
                {
                    return Err(FlowError::FieldNotLoaded { field });
                }
            }
            row.fields.retain(|k, _| mask.has(k.position()));
            if mask.has(ReportField::ObjectId.position()) {
                row.fields
                    .insert(ReportField::ObjectId, FieldValue::Text(row.key.clone()));
            }
        }
        Ok(pop)
    }

    fn export(&mut self, f: FnIndex, v: [u8; 3], pop: Population) -> Result<(), FlowError> {
        ExportFormat::from_byte(v[0]).ok_or(FlowError::MalformedCall { call: f })?;
        // No clipping here: every field enters a row through `get_user`'s
        // projection or `attach`, and both check the scope, so a population
        // never holds an unpermitted field to clip.
        let permitted: FieldMask = self.scope.fields;
        let rows = pop
            .rows
            .into_iter()
            .map(|r| ReportRow { fields: r.fields })
            .collect();
        self.report = Some(Report {
            requested: self.requested.to_owned(),
            tenant: self.scope.tenant,
            policy_version: self.scope.policy_version,
            permitted_fields: ReportField::ALL
                .into_iter()
                .filter(|k| permitted.has(k.position()))
                .collect(),
            source: self.source.clone(),
            rows,
            trace: self.trace.to_vec(),
        });
        Ok(())
    }
}

fn pop_population(f: FnIndex, stack: &mut Vec<Population>) -> Result<Population, FlowError> {
    stack.pop().ok_or(FlowError::StackShape { call: f })
}

impl<H: DirectoryObservations> Dialect for FlowDialect<'_, H> {
    type Value = Population;
    type Error = FlowError;

    fn truthy(&self, value: &Population) -> bool {
        !value.is_empty()
    }

    fn repeat_count(&self, value: &Population) -> u32 {
        u32::try_from(value.len()).unwrap_or(u32::MAX)
    }

    fn call(
        &mut self,
        f: FnIndex,
        values: [u8; 3],
        stack: &mut Vec<Population>,
    ) -> Result<(), FlowError> {
        self.require(f)?;
        match f {
            vocab::GET_USER => {
                let pop = self.get_user(f, values)?;
                stack.push(pop);
            }
            vocab::GET_LICENSE_EVIDENCE => {
                let pop = pop_population(f, stack)?;
                let (host, tenant) = (self.host, self.scope.tenant);
                stack.push(self.attach(pop, ReportField::Licenses, |k| host.licenses(tenant, k))?);
            }
            vocab::GET_MEMBERSHIP => {
                let pop = pop_population(f, stack)?;
                let (host, tenant) = (self.host, self.scope.tenant);
                stack.push(self.attach(pop, ReportField::Memberships, |k| {
                    host.memberships(tenant, k)
                })?);
            }
            vocab::WHERE => {
                let pop = pop_population(f, stack)?;
                stack.push(self.filter(f, values, pop)?);
            }
            vocab::SELECT => {
                let pop = pop_population(f, stack)?;
                stack.push(self.project(values, pop)?);
            }
            vocab::EXPORT => {
                let pop = pop_population(f, stack)?;
                self.export(f, values, pop)?;
            }
            _ => return Err(FlowError::UnsupportedCall { call: f }),
        }
        Ok(())
    }
}
