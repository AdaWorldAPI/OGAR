//! Harvest driver: OpenStreetMap-website (Rails) → OGAR `Vec<Class>` IR dump.
//! `cargo run -p ogar-from-rails --example harvest_osm -- <repo-root> [--ir]`
use std::path::Path;

fn main() {
    let arg = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/user/_src-osm-website".into());
    let ir = std::env::args().any(|a| a == "--ir");
    let root = Path::new(&arg);

    let mut classes = ogar_from_rails::extract_with(root, "osm");
    if classes.is_empty() {
        classes = ogar_from_rails::extract_with(&root.join("app/models"), "osm");
    }
    classes.sort_by(|a, b| a.name.cmp(&b.name));

    if !ir {
        println!("harvested {} classes from {}", classes.len(), arg);
        for c in &classes {
            println!(
                "  {:<28} parent={:<16} assoc={:<2} mixins={:<2} enums={}",
                c.name,
                c.parent.clone().unwrap_or_else(|| "-".into()),
                c.associations.len(),
                c.mixins.len(),
                c.enums.len(),
            );
        }
        return;
    }

    // --ir: full structured IR (relational graph), stable text form.
    println!(
        "# OSM Rails → OGAR IR (namespace=osm) — {} classes",
        classes.len()
    );
    for c in &classes {
        println!("\n## {}", c.name);
        if let Some(p) = &c.parent {
            println!("parent: {p}");
        }
        println!("language: {:?}", c.language);
        for m in &c.mixins {
            println!("mixin: {m}");
        }
        for a in &c.associations {
            let target = a.class_name.clone().unwrap_or_else(|| a.name.clone());
            let fk = a
                .foreign_key
                .as_ref()
                .map(|f| format!(" fk={f}"))
                .unwrap_or_default();
            println!("assoc: {:?} {} -> {}{}", a.kind, a.name, target, fk);
        }
        for e in &c.enums {
            println!("enum: {} ({:?})", e.column, e.source);
        }
        for s in &c.store_accessors {
            println!("store_accessor: {} [{}]", s.column, s.fields.join(", "));
        }
    }
}
