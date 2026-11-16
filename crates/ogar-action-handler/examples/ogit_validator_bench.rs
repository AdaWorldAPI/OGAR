//! Times `Validator::validate` over 100,000 synthetic writes.
//!
//! `OGIT_FORK_PATH=<checkout> cargo run --release -p ogar-action-handler \
//!   --features quack --example ogit_validator_bench`
//!
//! A write is a type, the JanusGraph keys of its mandatory attributes, a few
//! of its optional ones and three system keys. One write in ten drops a
//! mandatory key and one in ten adds an unknown key, so the reject paths run.

use std::path::PathBuf;
use std::time::Instant;

use ogar_action_handler::ogit::{AttrId, Flag, Lane, OgitTables, TypeId, Validator};

const WRITES: usize = 100_000;

fn main() {
    let root = std::env::var_os("OGIT_FORK_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../OGIT"));
    let tables = OgitTables::load(&root).expect("the OGIT checkout loads");
    let mut validator = Validator::new(&tables).expect("the validator is prepared");

    let (tys, attrs, flags) = (
        tables.lane(Lane::TypeAttrType),
        tables.lane(Lane::TypeAttrAttr),
        tables.lane(Lane::TypeAttrFlag),
    );
    let key = |r: usize| {
        validator
            .key(AttrId(u16::try_from(attrs[r]).unwrap()))
            .unwrap()
            .to_owned()
    };
    // Per type: its name and its keys, mandatory first.
    let mut shapes: Vec<(String, Vec<String>, usize)> = Vec::new();
    let mut r = 0;
    while r < tys.len() {
        let t = tys[r];
        let mut keys = Vec::new();
        let mut mandatory = 0;
        while r < tys.len() && tys[r] == t {
            if flags[r] == Flag::Mandatory as u32 {
                mandatory += 1;
                keys.push(key(r));
            } else if keys.len() < mandatory + 3 {
                keys.push(key(r));
            }
            r += 1;
        }
        let name = tables
            .type_name(TypeId(u16::try_from(t).unwrap()))
            .unwrap()
            .to_owned();
        shapes.push((name, keys, mandatory));
    }

    let system = ["ogit/_id", "ogit/_type", "ogit/_creator"];
    let writes: Vec<(&str, Vec<&str>)> = (0..WRITES)
        .map(|i| {
            let (name, keys, mandatory) = &shapes[i % shapes.len()];
            let mut ks: Vec<&str> = keys.iter().map(String::as_str).collect();
            if i % 10 == 3 && *mandatory > 0 {
                ks.remove(0);
            }
            if i % 10 == 7 {
                ks.push("ogit/notDeclaredAnywhere");
            }
            ks.extend(system);
            (name.as_str(), ks)
        })
        .collect();

    let mut violations = 0usize;
    let start = Instant::now();
    for (ty, keys) in &writes {
        violations += validator.validate(&tables, ty, keys).unwrap().len();
    }
    let elapsed = start.elapsed();
    println!(
        "{WRITES} writes over {} types: {:.0} ns/write, {violations} violations",
        shapes.len(),
        elapsed.as_nanos() as f64 / WRITES as f64
    );
}
