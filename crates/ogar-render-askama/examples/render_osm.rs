//! Render the OSM Rails harvest → Rust structs (ClassView × FieldMask → struct
//! plus `ActionDef` methods), one file per class.
//!
//! `cargo run -p ogar-render-askama --example render_osm -- <osm-root> <out-dir>`
use lance_graph_contract::class_view::FieldMask;
use ogar_render_askama::render_class_with_methods;
use ogar_vocab::canonical_concept_id;
use std::fs;
use std::path::Path;

/// Resolve an association target (a class_name like `"User"` or a relation
/// name like `"changeset"`) to its OGAR canonical concept, if grounded.
fn ground_target(target: &str) -> Option<&'static str> {
    // Try the raw name, then a PascalCase-ish normalisation of a snake name.
    ground(target).or_else(|| {
        let pascal: String = target
            .split('_')
            .map(|w| {
                let mut ch = w.chars();
                ch.next()
                    .map(|c| c.to_ascii_uppercase().to_string() + ch.as_str())
                    .unwrap_or_default()
            })
            .collect();
        ground(&pascal)
    })
}

/// Rails class name → OGAR canonical concept (the `ogar-vocab` codebook, Geo
/// domain `0x0FXX`). `None` for classes that aren't canonical geodata concepts
/// (diary / message / issue / … app-social entities) — they render without a
/// `CLASS_ID`.
fn ground(rails: &str) -> Option<&'static str> {
    Some(match rails {
        "Node" | "OldNode" => "osm_node",
        "Way" | "OldWay" => "osm_way",
        "Relation" | "OldRelation" => "osm_relation",
        "Changeset" => "osm_changeset",
        "NodeTag" | "WayTag" | "RelationTag" | "OldNodeTag" | "OldWayTag"
        | "OldRelationTag" => "osm_element_tag",
        "RelationMember" | "OldRelationMember" => "osm_relation_member",
        "WayNode" | "OldWayNode" => "osm_way_node",
        "Note" => "osm_note",
        "Trace" => "osm_gpx_trace",
        "User" => "osm_user",
        _ => return None,
    })
}

/// A safe snake_case Rust identifier (module or fn): sanitises `?`/punctuation,
/// avoids a leading digit, and escapes the handful of Rust keywords a container
/// or verb might hit (`type`, `move`, `ref`, `match`, …) with a trailing `_`.
fn rustify(name: &str) -> String {
    let s = snake(name);
    const KW: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
        "where", "while", "async", "await", "dyn", "box", "do", "macro", "try", "yield", "gen",
    ];
    if s.is_empty() {
        "action".to_string()
    } else if KW.contains(&s.as_str()) {
        format!("{s}_")
    } else if s.as_bytes()[0].is_ascii_digit() {
        format!("_{s}")
    } else {
        s
    }
}

/// PascalCase / dotted name → snake_case module/file stem.
fn snake(name: &str) -> String {
    let mut out = String::new();
    let mut prev_alnum_lower = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && prev_alnum_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_alnum_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            prev_alnum_lower = false;
        }
    }
    let s = out.trim_matches('_').to_string();
    if s.is_empty() {
        "anon".into()
    } else {
        s
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let src = args
        .next()
        .unwrap_or_else(|| "/home/user/_src-osm-website".into());
    let out_arg = args.next().unwrap_or_else(|| {
        "/home/user/OGAR/.claude/harvest/osm-website-rs/crates/osm-domain/src/generated".into()
    });
    let out = Path::new(&out_arg);
    fs::create_dir_all(out).unwrap();

    let src_path = Path::new(&src);
    let mut classes = ogar_from_rails::extract_with(src_path, "osm");
    if classes.is_empty() {
        classes = ogar_from_rails::extract_with(&src_path.join("app/models"), "osm");
    }
    classes.sort_by(|a, b| a.name.cmp(&b.name));

    // Grounding: map each Rails class onto its OGAR canonical concept (the 15%
    // ontological step the mechanical lift can't infer) so the render emits
    // `CLASS_ID` from the codebook. Versioned `Old*` classes ground to the same
    // concept as their base (temporal axis, not a new id).
    for c in classes.iter_mut() {
        if let Some(concept) = ground(&c.name) {
            c.canonical_concept = Some(concept.to_string());
        }
    }

    let mut mods: Vec<String> = Vec::new();
    for c in &classes {
        let stem = snake(&c.name);
        // THINK arm only — data struct + CLASS_ID. The DO arm is NOT methods on
        // the record; it is the standalone `osm::<part_of>::<is_a>(input)`
        // module tree emitted below (OGAR: "ActionDef is standalone, not on
        // Class").
        let code = render_class_with_methods(c, FieldMask(u64::MAX), &[]).expect("render");
        let header = format!(
            "// @generated by ogar-render-askama — ruff → OGAR transpile\n\
             // source class: {} (openstreetmap-website app/models)\n\n",
            c.name
        );
        fs::write(out.join(format!("{stem}.rs")), format!("{header}{code}")).unwrap();
        mods.push(stem);
    }
    mods.sort();
    mods.dedup();
    let mod_body: String = mods.iter().map(|m| format!("pub mod {m};\n")).collect();
    fs::write(
        out.join("mod.rs"),
        format!(
            "// @generated module index — {} OSM domain classes (ruff → OGAR render)\n{mod_body}",
            mods.len()
        ),
    )
    .unwrap();

    // ── OSM association graph as classid-keyed SPO edges (lance-graph feed) ──
    // node/way/relation as a graph: each grounded class is a subject carrying a
    // CLASS_ID; each association is an edge to a target concept (classid where
    // the target grounds).
    let mut spo = String::from(
        "# OSM association graph — SPO edges (ruff → OGAR), classid-keyed\n\
         # subject_concept[classid] --assoc_kind:name--> object[classid]\n\n",
    );
    let mut edges = 0usize;
    for c in &classes {
        let s_concept = c.canonical_concept.as_deref().unwrap_or("-");
        let s_id = c
            .canonical_concept
            .as_deref()
            .and_then(canonical_concept_id)
            .map(|i| format!("0x{i:04X}"))
            .unwrap_or_else(|| "----".into());
        for a in &c.associations {
            let target = a.class_name.clone().unwrap_or_else(|| a.name.clone());
            let o_id = ground_target(&target)
                .and_then(canonical_concept_id)
                .map(|i| format!("0x{i:04X}"))
                .unwrap_or_else(|| "----".into());
            spo.push_str(&format!(
                "{s_concept}[{s_id}] --{:?}:{}--> {target}[{o_id}]\n",
                a.kind, a.name
            ));
            edges += 1;
        }
    }
    if let Some(root) = out.ancestors().nth(4) {
        let hp = root.join("harvest");
        fs::create_dir_all(&hp).ok();
        fs::write(hp.join("osm_graph.spo"), &spo).unwrap();
    } else {
        eprintln!(
            "warning: out-dir {} is not ≥5 levels under a harvest tree — \
             skipping osm_graph.spo (codex #152: no longer silent)",
            out.display()
        );
    }

    // ── Action rail: (part_of : is_a) — the castable V3 action shape ──
    // `extract_action_rail` joins `app/controllers` internally, so it needs the
    // REPO ROOT — not the `app/models` dir the struct-extraction fallback may use.
    // Resolve the root so controllers resolve even when `src` was the models dir
    // (Bugbot/codex #152: the rail was silently empty for a models-only `src`).
    let rail_root: &Path = if src_path.join("app/controllers").is_dir() {
        src_path
    } else if src_path.ends_with("app/models") {
        src_path.parent().and_then(|p| p.parent()).unwrap_or(src_path)
    } else {
        src_path
    };
    let rail = ogar_from_rails::extract_action_rail(rail_root, "osm");
    let mut rt = String::from(
        "# OSM action rail — (part_of : is_a), the V3 castable action shape\n\
         # a u8:u8 HHTL tile per action; cast = fix one axis, walk the other\n\
         # part_of[id]:is_a[id]\\tcontroller#action\n\n",
    );
    let mut parts: std::collections::BTreeSet<(u8, String)> = std::collections::BTreeSet::new();
    let mut isas: std::collections::BTreeSet<(u8, String)> = std::collections::BTreeSet::new();
    for r in &rail {
        rt.push_str(&format!(
            "{}[0x{:02X}]:{}[0x{:02X}]\t{}#{}\n",
            r.part_of, r.part_of_id, r.is_a, r.is_a_id, r.controller, r.action
        ));
        parts.insert((r.part_of_id, r.part_of.clone()));
        isas.insert((r.is_a_id, r.is_a.clone()));
    }
    rt.push_str(&format!("\n# is_a axis ({} archetypes): ", isas.len()));
    rt.push_str(
        &isas
            .iter()
            .map(|(i, n)| format!("{n}=0x{i:02X}"))
            .collect::<Vec<_>>()
            .join(" "),
    );
    rt.push_str(&format!("\n# part_of axis ({} containers): ", parts.len()));
    rt.push_str(
        &parts
            .iter()
            .map(|(i, n)| format!("{n}=0x{i:02X}"))
            .collect::<Vec<_>>()
            .join(" "),
    );
    rt.push('\n');
    if let Some(root) = out.ancestors().nth(4) {
        fs::write(root.join("harvest/osm_actions.rail"), &rt).unwrap();
    } else {
        eprintln!(
            "warning: out-dir {} is not ≥5 levels under a harvest tree — \
             skipping osm_actions.rail (codex #152: no longer silent)",
            out.display()
        );
    }

    // ── DO arm: a faithful `controllers` module tree → generated/controllers.rs.
    // Each module MIRRORS a source controller 1:1 by its own (snake, verbatim,
    // NOT singularised) name — `NodesController → nodes`, `MapController → map`.
    // Functions are the `is_a` actions; standalone, never methods on the record.
    // osm-domain re-exports (`pub use controllers::*`) for the ergonomic surface,
    // so `osm_domain::controllers::nodes::show` and re-exported `nodes::show`
    // both resolve. The (part_of : is_a) tile stays the CANONICAL rail address,
    // so multiple source controllers on the same tile (`Api::NodesController#show`
    // + `NodesController#show` → `nodes:show`) are ONE castable action by design —
    // the api-vs-web surface is the classid custom-low half, not a new tile. All
    // sources are cited so provenance is never lost (Bugbot/codex #152).
    let mut tree: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, Vec<(String, String)>>,
    > = std::collections::BTreeMap::new();
    for r in &rail {
        tree.entry(r.part_of.clone())
            .or_default()
            .entry(r.is_a.clone())
            .or_default()
            .push((r.controller.clone(), r.action.clone()));
    }
    let mut acts = String::from(
        "//! @generated DO arm — a faithful `controllers` mirror. Each module is a\n\
         //! source controller by its own verbatim (snake) name; each fn is an `is_a`\n\
         //! action (standalone, not methods on the record). osm-domain re-exports this\n\
         //! module (`pub use controllers::*`), so `controllers::nodes::show(input)` and\n\
         //! the re-exported `nodes::show(input)` both resolve. No singularisation.\n\n\
         #![allow(clippy::all, dead_code, unused_variables)]\n\n\
         /// DO-arm action input — the Rails `params` / request. Typed-field harvest is a\n\
         /// follow-up (ruff `Function` carries reads/writes, not param types yet).\n\
         #[derive(Debug, Default)]\npub struct Input;\n\n\
         /// DO-arm action output — the Rails response.\n\
         #[derive(Debug, Default)]\npub struct Output;\n\n",
    );
    let mut fn_count = 0usize;
    for (part_of, isas) in &tree {
        acts.push_str(&format!(
            "pub mod {} {{\n    use super::{{Input, Output}};\n",
            rustify(part_of)
        ));
        // Distinct is_a rail keys can normalise to the SAME Rust fn name (`foo?`
        // and `foo` both → `foo`). Group by the emitted name and MERGE their
        // sources + labels into one fn, so no rail tile silently vanishes
        // (a `continue` here dropped the collider — codex/Bugbot on the Python
        // sibling #154; the Rust emitter had the same latent drop).
        let mut by_fname: std::collections::BTreeMap<String, (Vec<String>, Vec<String>)> =
            std::collections::BTreeMap::new();
        for (is_a, sources) in isas {
            let entry = by_fname.entry(rustify(is_a)).or_default();
            entry.0.push(is_a.clone());
            entry
                .1
                .extend(sources.iter().map(|(c, a)| format!("{c}#{a}")));
        }
        for (fname, (mut labels, mut srcs)) in by_fname {
            labels.sort();
            labels.dedup();
            srcs.sort();
            srcs.dedup();
            let is_a_disp = labels.join(", ");
            let primary = srcs[0].clone();
            let source_line = if srcs.len() == 1 {
                format!("Source: `{primary}`.")
            } else {
                format!("Sources (canonical tile): `{}`.", srcs.join("`, `"))
            };
            acts.push_str(&format!(
                "    /// `{part_of}:{is_a_disp}` — DO arm. {source_line}\n    \
                 pub fn {fname}(input: Input) -> Output {{\n        \
                 let _ = input;\n        todo!(\"port {primary}\")\n    }}\n"
            ));
            fn_count += 1;
        }
        acts.push_str("}\n\n");
    }
    fs::write(out.join("controllers.rs"), acts).unwrap();

    println!(
        "rendered {} data structs (THINK) + DO arm controllers::<name>::<is_a> = {fn_count} fns \
         across {} controllers [{} part_of x {} is_a rail, {} tiles]; {edges} SPO edges -> {}",
        classes.len(),
        tree.len(),
        parts.len(),
        isas.len(),
        rail.len(),
        out.display()
    );
}
