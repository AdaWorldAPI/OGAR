//! DO-arm adapter codegen — deterministic Rust scaffolds emitted from
//! harvested `ActionDef` signatures, gated on Klickwege structure-parity
//! witnesses (see [`emit_do_adapters`]).
//!
//! Corpus-agnostic by construction: this module only consumes typed
//! [`AdapterClass`] / [`AdapterAction`] structs the caller builds from
//! the real `ogar_vocab::ActionDef` harvest — no corpus data lives here.
//! Output is fully deterministic (stable sorts on every input `Vec`,
//! `BTreeSet`-tracked name disambiguation) so it is diffable as a golden
//! artifact across regenerations of the same harvest.

use std::collections::BTreeSet;

/// One class to emit an adapter for, built from the ActionDef harvest.
#[derive(Debug, Clone)]
pub struct AdapterClass {
    /// PascalCase class name (the ActionDef `object_class`).
    pub class_name: String,
    /// Resolved canonical concept snake_case name, or empty if unresolved.
    pub concept: String,
    /// The class's DO-arm actions, in harvest order.
    pub actions: Vec<AdapterAction>,
}

/// One action = one ActionDef signature (writes/reads only; kausal/body
/// are out of scope — signature-only harvest).
#[derive(Debug, Clone)]
pub struct AdapterAction {
    /// The predicate (method name).
    pub predicate: String,
    /// Write-target strings (e.g. "Ns:Class.Field"), harvest order.
    pub writes: Vec<String>,
    /// Read-source strings, harvest order.
    pub reads: Vec<String>,
}

/// Emit a deterministic Rust module of classid-keyed DO-arm adapter
/// scaffolds. `nav_witnessed` is the set of canonical concept names the
/// Klickwege navigation plane surfaces (structure-parity witnesses): a
/// class whose `concept` is present is STRUCTURE-WITNESSED; one whose
/// concept is absent (or empty) is emitted with a
/// `// STRUCTURE-UNWITNESSED` marker so the parity gap is visible in the
/// generated code, never silent. Fully deterministic (BTree ordering,
/// stable across runs) so the output is diffable as a golden artifact.
#[must_use]
pub fn emit_do_adapters(classes: &[AdapterClass], nav_witnessed: &BTreeSet<String>) -> String {
    let mut sorted: Vec<&AdapterClass> = classes.iter().collect();
    sorted.sort_by(|a, b| a.class_name.cmp(&b.class_name));

    let mut out = String::new();
    out.push_str("//! Generated DO-arm adapters — DO NOT EDIT.\n");
    out.push_str("//! Emitted from the ActionDef harvest; gated on Klickwege structure parity.\n");
    out.push('\n');

    let mut witnessed_count = 0usize;
    let mut unwitnessed_count = 0usize;

    for class in &sorted {
        let is_witnessed = !class.concept.is_empty() && nav_witnessed.contains(&class.concept);
        if is_witnessed {
            witnessed_count += 1;
            out.push_str("// [structure: witnessed]\n");
        } else {
            unwitnessed_count += 1;
            let concept_display = if class.concept.is_empty() {
                "?"
            } else {
                class.concept.as_str()
            };
            out.push_str(&format!(
                "// [structure: UNWITNESSED — no Klickwege screen surfaces \"{concept_display}\"]\n"
            ));
        }

        out.push_str(&format!("pub struct {}Adapter;\n", class.class_name));
        out.push('\n');
        out.push_str(&format!("impl {}Adapter {{\n", class.class_name));

        let mut actions: Vec<&AdapterAction> = class.actions.iter().collect();
        actions.sort_by(|a, b| a.predicate.cmp(&b.predicate));

        let mut used_fn_names: BTreeSet<String> = BTreeSet::new();
        let mut action_blocks: Vec<String> = Vec::with_capacity(actions.len());
        for action in &actions {
            let base = to_snake_case(&action.predicate);
            let fn_name = unique_fn_name(&base, &mut used_fn_names);

            let mut writes: Vec<&str> = action.writes.iter().map(String::as_str).collect();
            writes.sort_unstable();
            let mut reads: Vec<&str> = action.reads.iter().map(String::as_str).collect();
            reads.sort_unstable();

            let mut block = String::new();
            block.push_str(&format!(
                "    /// `{}` — generated from ActionDef signature.\n",
                action.predicate
            ));
            block.push_str(&format!(
                "    /// writes ({}): {}\n",
                writes.len(),
                name_preview(&writes)
            ));
            block.push_str(&format!(
                "    /// reads ({}): {}\n",
                reads.len(),
                name_preview(&reads)
            ));
            block.push_str(&format!("    pub fn {fn_name}(&self) {{\n"));
            block.push_str(
                "        /* generated from ActionDef signature; body is upstream (thinking lives in lance-graph) */\n",
            );
            block.push_str("        unimplemented!()\n");
            block.push_str("    }\n");
            action_blocks.push(block);
        }
        out.push_str(&action_blocks.join("\n"));
        out.push_str("}\n");
        out.push('\n');
    }

    let total = sorted.len();
    out.push_str(&format!(
        "// summary: total={total} witnessed={witnessed_count} unwitnessed={unwitnessed_count}\n"
    ));

    out
}

/// First-few-names preview of a sorted name slice: up to 3 names, with a
/// trailing `", ..."` when more were dropped. `"(none)"` when empty.
fn name_preview(names: &[&str]) -> String {
    if names.is_empty() {
        return "(none)".to_string();
    }
    let take = names.len().min(3);
    let mut s = names[..take].join(", ");
    if names.len() > take {
        s.push_str(", ...");
    }
    s
}

/// Convert a PascalCase / mixedCase / already-snake_case predicate name
/// into a Rust-identifier-safe snake_case fragment. Idempotent on input
/// that is already snake_case. Non-identifier characters are dropped
/// rather than transcribed, since predicate names are expected to be
/// identifier-shaped already.
fn to_snake_case(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' || c == '-' || c == ' ' {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            continue;
        }
        if c.is_ascii_uppercase() {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = chars.get(i + 1).copied();
            let boundary = match prev {
                Some(p) if p.is_ascii_lowercase() || p.is_ascii_digit() => true,
                Some(p) if p.is_ascii_uppercase() => next.is_some_and(|n| n.is_ascii_lowercase()),
                _ => false,
            };
            if boundary && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        }
    }
    let collapsed: String = out
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if collapsed.is_empty() {
        "unnamed".to_string()
    } else {
        collapsed
    }
}

/// Deterministically disambiguate a snake_case function name against the
/// names already used within the same `impl` block: the first occurrence
/// wins the bare name; every subsequent collision gets `_2`, `_3`, ….
fn unique_fn_name(base: &str, used: &mut BTreeSet<String>) -> String {
    if used.insert(base.to_string()) {
        return base.to_string();
    }
    let mut n = 2usize;
    loop {
        let candidate = format!("{base}_{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invoice_class() -> AdapterClass {
        AdapterClass {
            class_name: "Invoice".to_string(),
            concept: "invoice".to_string(),
            actions: vec![
                AdapterAction {
                    predicate: "PostInvoice".to_string(),
                    writes: vec![
                        "Invoice.state".to_string(),
                        "Invoice.posted_at".to_string(),
                    ],
                    reads: vec!["Invoice.lines".to_string()],
                },
                AdapterAction {
                    predicate: "cancel_invoice".to_string(),
                    writes: vec![],
                    reads: vec!["Invoice.state".to_string()],
                },
            ],
        }
    }

    fn cipher_panel_class() -> AdapterClass {
        AdapterClass {
            class_name: "CipherPanel".to_string(),
            concept: String::new(),
            actions: vec![AdapterAction {
                predicate: "Reset".to_string(),
                writes: vec!["CipherPanel.key".to_string()],
                reads: vec![],
            }],
        }
    }

    fn witness_set() -> BTreeSet<String> {
        let mut s = BTreeSet::new();
        s.insert("invoice".to_string());
        s
    }

    #[test]
    fn deterministic_output_is_pinned() {
        let classes = vec![invoice_class(), cipher_panel_class()];
        let out = emit_do_adapters(&classes, &witness_set());
        let expected = r#"//! Generated DO-arm adapters — DO NOT EDIT.
//! Emitted from the ActionDef harvest; gated on Klickwege structure parity.

// [structure: UNWITNESSED — no Klickwege screen surfaces "?"]
pub struct CipherPanelAdapter;

impl CipherPanelAdapter {
    /// `Reset` — generated from ActionDef signature.
    /// writes (1): CipherPanel.key
    /// reads (0): (none)
    pub fn reset(&self) {
        /* generated from ActionDef signature; body is upstream (thinking lives in lance-graph) */
        unimplemented!()
    }
}

// [structure: witnessed]
pub struct InvoiceAdapter;

impl InvoiceAdapter {
    /// `PostInvoice` — generated from ActionDef signature.
    /// writes (2): Invoice.posted_at, Invoice.state
    /// reads (1): Invoice.lines
    pub fn post_invoice(&self) {
        /* generated from ActionDef signature; body is upstream (thinking lives in lance-graph) */
        unimplemented!()
    }

    /// `cancel_invoice` — generated from ActionDef signature.
    /// writes (0): (none)
    /// reads (1): Invoice.state
    pub fn cancel_invoice(&self) {
        /* generated from ActionDef signature; body is upstream (thinking lives in lance-graph) */
        unimplemented!()
    }
}

// summary: total=2 witnessed=1 unwitnessed=1
"#;
        assert_eq!(out, expected);
    }

    #[test]
    fn shuffled_input_is_still_deterministic() {
        let classes_forward = vec![invoice_class(), cipher_panel_class()];
        let classes_reversed = vec![cipher_panel_class(), invoice_class()];
        let witness = witness_set();
        let out_forward = emit_do_adapters(&classes_forward, &witness);
        let out_reversed = emit_do_adapters(&classes_reversed, &witness);
        assert_eq!(out_forward, out_reversed);
    }

    #[test]
    fn witness_gate_marks_witnessed_and_unwitnessed_classes() {
        let classes = vec![invoice_class(), cipher_panel_class()];
        let out = emit_do_adapters(&classes, &witness_set());
        assert!(out.contains("// [structure: witnessed]\npub struct InvoiceAdapter;"));
        assert!(out.contains(
            "// [structure: UNWITNESSED — no Klickwege screen surfaces \"?\"]\npub struct CipherPanelAdapter;"
        ));
    }

    #[test]
    fn snake_case_collisions_get_distinct_suffixes() {
        let class = AdapterClass {
            class_name: "Cipher".to_string(),
            concept: "cipher".to_string(),
            actions: vec![
                AdapterAction {
                    predicate: "GetX".to_string(),
                    writes: vec![],
                    reads: vec![],
                },
                AdapterAction {
                    predicate: "get_x".to_string(),
                    writes: vec![],
                    reads: vec![],
                },
            ],
        };
        let mut witness = BTreeSet::new();
        witness.insert("cipher".to_string());
        let classes = vec![class];
        let out = emit_do_adapters(&classes, &witness);
        assert!(out.contains("pub fn get_x(&self)"));
        assert!(out.contains("pub fn get_x_2(&self)"));
        assert!(!out.contains("pub fn get_x_2_2(&self)"));
    }

    #[test]
    fn generated_body_is_unimplemented_with_upstream_note() {
        let classes = vec![invoice_class()];
        let out = emit_do_adapters(&classes, &witness_set());
        assert!(out.contains("unimplemented!()"));
        assert!(out.contains("thinking lives in lance-graph"));
    }
}
