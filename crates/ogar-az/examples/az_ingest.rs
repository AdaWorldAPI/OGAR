//! Read-only lab ingest of a Microsoft Graph `users` page.
//!
//! This example performs no HTTP. Fetch with any client holding a token that
//! has `User.Read.All` (application or delegated), e.g.:
//!
//! ```sh
//! URL=$(cargo run -q -p ogar-az --example az_ingest -- --url)
//! curl -s -H "Authorization: Bearer $TOKEN" "$URL" > page1.json
//! cargo run -p ogar-az --example az_ingest -- page1.json <tenant-id>
//! ```
//!
//! Follow `next_link` for further pages with the same dictionary and pool.

use ogar_dir_core::{OuDictionary, ValuePool};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--url") {
        println!("{}", ogar_az::users_url(ogar_az::SCHEMA_VERSION, 100));
        return;
    }
    let [path, tenant] = args.as_slice() else {
        eprintln!("usage: az_ingest <page.json> <tenant-guid> | --url");
        std::process::exit(2);
    };
    let tenant = ogar_dir_core::Guid128::parse(tenant).expect("tenant must be a GUID");
    let body = std::fs::read_to_string(path).expect("read page");
    let (mut dict, mut pool) = (OuDictionary::new(), ValuePool::new());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let page = ogar_az::ingest_page(&body, tenant, &mut dict, &mut pool, now).expect("ingest");
    for r in &page.records {
        let ou = r.ou_hhtl().and_then(|h| dict.explain(&h).map(|n| (h, n)));
        println!(
            "{} upn={:?} immutableId={:?} syncEnabled={:?} ou={:?}",
            r.node_guid(),
            ogar_az::attr_str(r, &pool, "userPrincipalName"),
            ogar_az::attr_str(r, &pool, "onPremisesImmutableId"),
            r.num(1),
            ou,
        );
    }
    println!(
        "records={} pool_bytes={} ignored={:?} next_link={:?}",
        page.records.len(),
        pool.as_bytes().len(),
        page.ignored,
        page.next_link
    );
}
