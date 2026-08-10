//! Split a baked union into per-namespace slabs, exactly as the S3 layout holds them.
use ogar_obo::{
    Row512, as_le_bytes, bake_with, merge_logical_defs, parse_hp_logical_defs, parse_obo,
    registry::OBO_CORE,
};
use std::collections::HashMap;
fn main() {
    let dir = std::env::args().nth(1).unwrap_or("/tmp/obo".into());
    let out = std::env::args().nth(2).unwrap_or("/tmp/slabs".into());
    std::fs::create_dir_all(&out).unwrap();
    let mut nodes = HashMap::new();
    for f in ["mondo", "hp", "uberon", "pato", "ro"] {
        for (id, n) in parse_obo(&std::fs::read_to_string(format!("{dir}/{f}.obo")).unwrap()) {
            let e: &mut ogar_obo::OboNode = nodes.entry(id).or_default();
            e.is_a.extend(n.is_a);
            e.rel.extend(n.rel);
            e.xref.extend(n.xref);
            e.obsolete |= n.obsolete;
        }
    }
    let owl = std::fs::read_to_string(format!("{dir}/hp-base.owl")).unwrap_or_default();
    merge_logical_defs(&mut nodes, &parse_hp_logical_defs(&owl));
    let baked = bake_with(&nodes, 0x0000, &OBO_CORE);
    assert_eq!(baked.stats.links_dropped, 0);
    assert_eq!(baked.stats.rows_overflowed, 0);
    let mut per: HashMap<&str, Vec<Row512>> = HashMap::new();
    for (i, id) in baked.ids.iter().enumerate() {
        let name = match id.ns {
            0 => "mondo",
            1 => "hpo",
            2 => "uberon",
            3 => "pato",
            _ => "ro",
        };
        per.entry(name).or_default().push(baked.rows[i]);
    }
    let mut names: Vec<_> = per.keys().copied().collect();
    names.sort_unstable();
    for n in names {
        let rows = &per[n];
        let p = format!("{out}/obo.{n}.soa");
        std::fs::write(&p, as_le_bytes(rows)).unwrap();
        println!("{p}  {} rows  {} bytes", rows.len(), rows.len() * 512);
    }
}
