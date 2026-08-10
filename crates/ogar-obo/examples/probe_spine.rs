//! Measure what the RDF front-end actually harvests from the meta-study spine.
//!
//! This is the falsifier for [`ogar_obo::rdf`]: it runs the *shipped* code path
//! over real ontology documents and reports numbers, rather than asserting the
//! path works. It also measures the numeric-`0` pad collision that gates
//! [`ogar_obo::registry::META_STUDY_SPINE`] — so the gate is a measurement, not
//! a claim.
//!
//! ```text
//! cargo run -p ogar-obo --features rdf --example probe_spine -- <dir>
//! ```
//!
//! `<dir>` holds one document per namespace, named `<PREFIX>.<ext>` (the
//! extension picks the serialization via `rdf::detect_format`). A missing file
//! is reported, never silently skipped — a spine that quietly measured 4 of 8
//! namespaces would read as a complete result.

use std::collections::BTreeMap;
use std::path::Path;

use ogar_obo::rdf::{self, OBO_RESTRICTION_PREDICATES, RdfFormat};
use ogar_obo::registry::{META_STUDY_SPINE, NsRegistry};
use ogar_obo::{OboNode, Predicate, TermId};

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: probe_spine <dir-of-ontology-documents>");
        std::process::exit(2);
    });
    let dir = Path::new(&dir);
    let reg = &META_STUDY_SPINE;

    // Harvest every namespace into ONE map: the spine is a single graph whose
    // members import each other, and the cross-ontology edge count below is
    // only meaningful across the union.
    let mut union: std::collections::HashMap<TermId, OboNode> = std::collections::HashMap::new();
    let mut missing: Vec<&str> = Vec::new();
    let mut per_file: Vec<(String, usize)> = Vec::new();

    for spec in reg.specs() {
        let Some((path, bytes)) = read_named(dir, spec.prefix) else {
            missing.push(spec.prefix);
            continue;
        };
        let fmt = rdf::detect_format(
            path.extension().and_then(|e| e.to_str()),
            &bytes[..bytes.len().min(256)],
        );
        match rdf::parse_rdf(&bytes, fmt, reg, OBO_RESTRICTION_PREDICATES) {
            Ok(nodes) => {
                per_file.push((format!("{} ({:?})", path.display(), fmt), nodes.len()));
                for (id, node) in nodes {
                    let slot = union.entry(id).or_default();
                    slot.is_a.extend(node.is_a);
                    slot.rel.extend(node.rel);
                    slot.xref.extend(node.xref);
                    slot.obsolete |= node.obsolete;
                }
            }
            Err(e) => {
                eprintln!("!! {}: {e}", path.display());
                missing.push(spec.prefix);
            }
        }
    }

    for (f, n) in &per_file {
        println!("read  {f}: {n} classes");
    }
    if !missing.is_empty() {
        println!("\n!! NOT MEASURED: {missing:?} — the totals below exclude them");
    }

    // Per-namespace census, keyed by ordinal so the report follows table order.
    let mut own: BTreeMap<u8, Census> = BTreeMap::new();
    for (id, node) in &union {
        let c = own.entry(id.ns).or_default();
        c.classes += 1;
        c.obsolete += usize::from(node.obsolete);
        c.is_a += node.is_a.len();
        c.rel += node.rel.len();
        c.max_num = c.max_num.max(id.num);
        if id.num == 0 {
            c.zero_id += 1;
        }
        c.max_is_a = c.max_is_a.max(node.is_a.len());
        for parent in &node.is_a {
            if parent.ns != id.ns {
                *c.cross.entry(parent.ns).or_default() += 1;
            }
        }
        for (p, o) in &node.rel {
            if *p == Predicate::Other {
                c.rel_other += 1;
            }
            if o.ns != id.ns {
                *c.cross.entry(o.ns).or_default() += 1;
            }
        }
    }

    println!(
        "\n{:<7} {:>4} {:>8} {:>7} {:>7} {:>6} {:>9} {:>7} {:>6}",
        "ns", "ord", "classes", "is_a", "maxIsA", "rel", "max num", "id==0", "obs"
    );
    let (mut t_c, mut t_i, mut t_r, mut t_z) = (0, 0, 0, 0);
    for (ord, c) in &own {
        println!(
            "{:<7} {:>4} {:>8} {:>7} {:>7} {:>6} {:>9} {:>7} {:>6}",
            name(reg, *ord),
            ord,
            c.classes,
            c.is_a,
            c.max_is_a,
            c.rel,
            c.max_num,
            c.zero_id,
            c.obsolete
        );
        t_c += c.classes;
        t_i += c.is_a;
        t_r += c.rel;
        t_z += c.zero_id;
    }
    println!(
        "{:<7} {:>4} {:>8} {:>7} {:>7} {:>6}",
        "TOTAL", "", t_c, t_i, "", t_r
    );

    println!("\ncross-namespace edges (subject ns -> object ns : count)");
    for (ord, c) in &own {
        for (other, n) in &c.cross {
            println!("  {} -> {} : {n}", name(reg, *ord), name(reg, *other));
        }
    }

    let rel_other: usize = own.values().map(|c| c.rel_other).sum();
    println!(
        "\ntyped edges landing on Predicate::Other: {rel_other} of {t_r} \
         (no CURIE row in OBO_RESTRICTION_PREDICATES — reported, never guessed)"
    );

    // The gate, measured.
    println!("\n── numeric-0 pad collision (gates META_STUDY_SPINE) ──");
    if t_z == 0 {
        println!("  none — no term in this spine carries numeric 0");
    } else {
        for (ord, c) in &own {
            if c.zero_id > 0 {
                println!(
                    "  {} has a real term at numeric 0 — an edge into it is \
                     indistinguishable from the lane pad",
                    name(reg, *ord)
                );
            }
        }
        println!(
            "  => {t_z} affected namespace(s). Baking them is unsound until the \
             edges.rs +1 bias lands (operator call: it moves every shipped digest)."
        );
    }
}

#[derive(Default)]
struct Census {
    classes: usize,
    obsolete: usize,
    is_a: usize,
    max_is_a: usize,
    rel: usize,
    rel_other: usize,
    max_num: u32,
    zero_id: usize,
    cross: BTreeMap<u8, usize>,
}

fn name(reg: &NsRegistry, ord: u8) -> &'static str {
    reg.spec(ord).map_or("?", |s| s.prefix)
}

/// Find `<dir>/<PREFIX>.<any-ext>` — the serialization is decided by
/// `detect_format`, not by the caller guessing the extension.
fn read_named(dir: &Path, prefix: &str) -> Option<(std::path::PathBuf, Vec<u8>)> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.file_stem().and_then(|s| s.to_str()) == Some(prefix) {
            let bytes = std::fs::read(&p).ok()?;
            return Some((p, bytes));
        }
    }
    None
}

// Keep the unused-import lint honest when the format enum is only used via
// `detect_format`'s return value.
const _: fn() -> RdfFormat = || RdfFormat::Turtle;
