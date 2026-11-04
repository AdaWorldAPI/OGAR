//! The report a flow produces, and its deterministic JSON rendering.
//!
//! JSON exists only here, at the export boundary. Nothing upstream of
//! `export` serializes anything.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use lance_graph_contract::sla::TenantId;

use crate::frontend::TraceEntry;
use crate::host::{FieldValue, SourceStamp};
use crate::vocab::ReportField;

/// The format tag every rendered report carries.
pub const REPORT_FORMAT: &str = "ogar-flow-m365/report.v1";

/// One exported row: only the selected, permitted fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportRow {
    /// The fields, in mask-position order.
    pub fields: BTreeMap<ReportField, FieldValue>,
}

/// A finished report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The user reference the pipeline asked for.
    pub requested: String,
    /// The tenant the run was scoped to.
    pub tenant: TenantId,
    /// The policy version the scope came from.
    pub policy_version: u32,
    /// The fields the scope permitted.
    pub permitted_fields: Vec<ReportField>,
    /// Where the facts came from.
    pub source: SourceStamp,
    /// The rows.
    pub rows: Vec<ReportRow>,
    /// One entry per pipeline stage.
    pub trace: Vec<TraceEntry>,
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn json_value(out: &mut String, v: &FieldValue) {
    match v {
        FieldValue::Text(s) => json_str(out, s),
        FieldValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        FieldValue::List(xs) => {
            out.push('[');
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json_str(out, x);
            }
            out.push(']');
        }
    }
}

impl Report {
    /// Render as compact JSON. Key order is fixed, so equal reports render to
    /// equal bytes.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut o = String::new();
        o.push_str("{\"format\":");
        json_str(&mut o, REPORT_FORMAT);
        o.push_str(",\"requested\":");
        json_str(&mut o, &self.requested);
        let _ = write!(
            o,
            ",\"scope\":{{\"tenant\":{},\"policy_version\":{},\"fields\":[",
            self.tenant, self.policy_version
        );
        for (i, f) in self.permitted_fields.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            json_str(&mut o, f.name());
        }
        o.push_str("]},\"source\":{\"id\":");
        json_str(&mut o, &self.source.source_id);
        let _ = write!(
            o,
            ",\"observed_version\":{}}},\"rows\":[",
            self.source.observed_version
        );
        for (i, row) in self.rows.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            o.push('{');
            for (j, (k, v)) in row.fields.iter().enumerate() {
                if j > 0 {
                    o.push(',');
                }
                json_str(&mut o, k.name());
                o.push(':');
                json_value(&mut o, v);
            }
            o.push('}');
        }
        o.push_str("],\"trace\":[");
        for (i, t) in self.trace.iter().enumerate() {
            if i > 0 {
                o.push(',');
            }
            o.push_str("{\"label\":");
            json_str(&mut o, &t.label);
            o.push_str(",\"operation\":");
            json_str(&mut o, t.operation);
            let _ = write!(o, ",\"fn\":\"{:#04x}\"}}", t.fn_index.0);
        }
        o.push_str("]}");
        o
    }
}
