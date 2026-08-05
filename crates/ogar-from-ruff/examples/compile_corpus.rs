//! Corpus compiler — ndjson triples → CompiledClass array.
//!
//! This example demonstrates the full transpile pipeline for a harvested
//! corpus:
//!
//! 1. Read ndjson triples from a file (or .gz if the plain path doesn't exist)
//! 2. Parse into Triple objects using ruff_spo_triplet::from_ndjson
//! 3. Reassemble them into a language-agnostic `ModelGraph`
//! 4. Lift and mint (compile) the graph using the appropriate language+port combo
//! 5. When `--features serde` is enabled, serialize to JSON (the fixture
//!    medcare-bridge and other consumers depend on)
//! 6. Write the output (compressing if the path ends with .gz)
//!
//! Usage (without JSON serialization, stdout only):
//! ```sh
//! cargo run -p ogar-from-ruff --example compile_corpus -- \
//!     /path/to/corpus.ndjson <domain> <port>
//! ```
//!
//! Usage (with JSON output, if serde feature is enabled):
//! ```sh
//! cargo run -p ogar-from-ruff --example compile_corpus --features serde -- \
//!     /path/to/corpus.ndjson <domain> <port> /path/to/output.json[.gz]
//! ```
//!
//! Domain examples: "medcare", "odoo", "woa", "q2"
//! Port examples: "healthcare", "odoo", "woa", "q2"

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use ogar_from_ruff::mint::compile_graph_csharp;
use ogar_vocab::ports::HealthcarePort;
use ruff_spo_triplet::{from_ndjson, reassemble_model_graph};

#[cfg(feature = "serde")]
use {hex, serde_json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!(
            "Usage: {} <corpus.ndjson[.gz]> <domain> <port> [output.json[.gz]]",
            args[0]
        );
        eprintln!("  Domain:   medcare | odoo | woa | q2 | ...");
        eprintln!("  Port:     healthcare | odoo | woa | q2 | ...");
        std::process::exit(1);
    }

    let corpus_path = Path::new(&args[1]);
    let domain = &args[2];
    let port_name = &args[3];
    let output_path = args.get(4).map(|s| Path::new(s.as_str()));

    eprintln!("=== Corpus compiler ===");
    eprintln!("corpus:     {}", corpus_path.display());
    eprintln!("domain:     {}", domain);
    eprintln!("port:       {}", port_name);
    if let Some(out) = output_path {
        eprintln!("output:     {}", out.display());
    }

    // Read and parse the corpus (ndjson or gzipped).
    let ndjson_text = read_ndjson_corpus(corpus_path)?;
    eprintln!("loaded ndjson file ({} bytes)", ndjson_text.len());

    // Parse ndjson into Triple objects (validates predicate vocabulary).
    let triples =
        from_ndjson(&ndjson_text).map_err(|e| format!("failed to parse ndjson: {}", e))?;
    eprintln!("parsed {} triples", triples.len());

    // Reassemble into a ModelGraph (arguments: triples first, then namespace/domain).
    let graph = reassemble_model_graph(&triples, domain);
    eprintln!("reassembled {} classes", graph.models.len());

    let total_fields: usize = graph.models.iter().map(|m| m.fields.len()).sum();
    let total_associations: usize = graph.models.iter().flat_map(|m| &m.associations).count();
    eprintln!(
        "        {} total fields, {} total associations",
        total_fields, total_associations
    );

    // Route by port and compile.
    let compiled = match port_name.as_str() {
        "healthcare" => {
            let classes = compile_graph_csharp::<HealthcarePort>(&graph);
            eprintln!("compiled:   {} CompiledClass", classes.len());
            let n_actions: usize = classes.iter().map(|c| c.actions.len()).sum();
            eprintln!("            {} total ActionDefs", n_actions);
            classes
        }
        _ => {
            eprintln!("ERROR: unsupported port '{}'", port_name);
            eprintln!("Supported ports: healthcare");
            std::process::exit(1);
        }
    };

    // Report facet distribution.
    let n_nonzero_classid = compiled
        .iter()
        .filter(|c| c.facet.facet_classid() != 0)
        .count();
    eprintln!(
        "facet:      {} → classid 0x00000000, {} non-zero",
        compiled.len() - n_nonzero_classid,
        n_nonzero_classid
    );

    // Serialize to JSON if output path is provided and serde feature is enabled.
    #[cfg(feature = "serde")]
    {
        if let Some(out) = output_path {
            write_json_output(out, &compiled)?;
            eprintln!("generated:  {}", out.display());
        }
    }

    #[cfg(not(feature = "serde"))]
    {
        if output_path.is_some() {
            eprintln!("ERROR: JSON output requested, but serde feature is not enabled.");
            eprintln!("Rebuild with: cargo run --features serde -- ...");
            std::process::exit(1);
        }
    }

    eprintln!("done.");
    Ok(())
}

/// Read an ndjson corpus file (or its .gz variant) into a single string.
fn read_ndjson_corpus(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let reader = match File::open(path) {
        Ok(file) => Box::new(file) as Box<dyn std::io::Read>,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Try .gz sibling.
            let mut gz_path = path.as_os_str().to_owned();
            gz_path.push(".gz");
            let gz_path = PathBuf::from(gz_path);
            match File::open(&gz_path) {
                Ok(file) => Box::new(flate2::read::GzDecoder::new(file)),
                Err(_) => {
                    return Err(format!(
                        "corpus file not found: {} or {}",
                        path.display(),
                        gz_path.display()
                    )
                    .into());
                }
            }
        }
        Err(e) => return Err(e.into()),
    };

    let reader = BufReader::new(reader);
    let mut text = String::new();
    for line in reader.lines() {
        text.push_str(&line?);
        text.push('\n');
    }
    Ok(text)
}

/// Serialize compiled classes to JSON (gzip if the path ends with .gz).
/// Since ogar_from_ruff::CompiledClass doesn't derive Serialize by default,
/// we build a custom JSON representation from public accessors.
#[cfg(feature = "serde")]
fn write_json_output(
    output_path: &Path,
    compiled: &[ogar_from_ruff::mint::CompiledClass],
) -> Result<(), Box<dyn std::error::Error>> {
    // Build a JSON-serializable representation of each CompiledClass.
    // This mirrors the shape that medcare-rs' actiondef_exec::CompiledClassFixture expects.
    let json_classes: Vec<serde_json::Value> = compiled
        .iter()
        .map(|c| {
            serde_json::json!({
                "class": serde_json::json!({
                    "name": &c.class.name,
                    "associations": c.class.associations.iter().map(|a| serde_json::json!({
                        "name": &a.name,
                        "kind": format!("{:?}", a.kind),
                        "class_name": &a.class_name,
                        "foreign_key": &a.foreign_key,
                        "polymorphic": a.polymorphic,
                        "through": &a.through,
                        "source": &a.source,
                    })).collect::<Vec<_>>(),
                }),
                "facet": serde_json::json!({
                    "classid": c.facet.facet_classid(),
                    "part_of_chain": c.facet.part_of_chain().to_vec(),
                    "is_a_chain": c.facet.is_a_chain().to_vec(),
                    "bytes_le": hex::encode(c.facet.to_bytes()),
                }),
                "actions": c.actions.iter().map(|a| serde_json::json!({
                    "predicate": &a.predicate,
                    "writes": &a.writes,
                    "reads": &a.reads,
                    "calls": &a.calls,
                    "raises": &a.raises,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    let json = serde_json::to_vec_pretty(&json_classes)?;

    // Write output (with gzip if path ends with .gz).
    if output_path.to_string_lossy().ends_with(".gz") {
        let file = File::create(output_path)?;
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        encoder.write_all(&json)?;
        encoder.finish()?;
    } else {
        let mut file = File::create(output_path)?;
        file.write_all(&json)?;
    }

    Ok(())
}
