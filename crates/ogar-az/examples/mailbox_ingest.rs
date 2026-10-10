//! Read-only lab ingest of one user's Exchange Online mailbox via Graph.
//!
//! This example performs no HTTP. Print the reads and the permission each
//! needs, fetch them with any client holding a token, then encode:
//!
//! ```sh
//! cargo run -q -p ogar-az --example mailbox_ingest -- --pulls <user-id>
//! curl -s -H "Authorization: Bearer $TOKEN" "<mailboxSettings url>" > settings.json
//! curl -s -H "Authorization: Bearer $TOKEN" "<settings/exchange url>" > exchange.json
//! cargo run -q -p ogar-az --example mailbox_ingest -- <user-id> <tenant-id> settings.json exchange.json
//! ```

use ogar_az::mailbox::{self, MailboxBodies, Pull};
use ogar_dir_core::{Guid128, ValuePool};

fn guid(s: &str) -> Guid128 {
    Guid128::parse(s).expect("expected a GUID")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, user] if flag == "--pulls" => {
            let user = guid(user);
            for p in [
                Pull::MailboxSettings { user },
                Pull::ExchangeSettings { user },
            ] {
                println!("GET {}  ({})", p.url(), p.permission().name());
            }
        }
        [user, tenant, settings, exchange] => {
            let (settings, exchange) = (
                std::fs::read_to_string(settings).expect("read settings"),
                std::fs::read_to_string(exchange).expect("read exchange"),
            );
            let mut pool = ValuePool::new();
            let rec = mailbox::encode_mailbox(
                &MailboxBodies {
                    user: guid(user),
                    mailbox_settings: Some(&settings),
                    exchange_settings: Some(&exchange),
                },
                guid(tenant),
                &mut pool,
                0,
            )
            .expect("encode");
            println!(
                "{} purpose={:?} primaryMailboxId={:?} mailboxGuid={:?}",
                rec.node_guid(),
                mailbox::user_purpose(&rec, &pool),
                mailbox::primary_mailbox_id(&rec, &pool),
                mailbox::mailbox_guid(&rec),
            );
            if let Some(id) = mailbox::primary_mailbox_id(&rec, &pool) {
                let f = Pull::MailboxFolders { mailbox: id };
                println!("next: GET {}  ({})", f.url(), f.permission().name());
            }
        }
        _ => {
            eprintln!(
                "usage: mailbox_ingest --pulls <user-id> | <user-id> <tenant-id> <settings.json> <exchange.json>"
            );
            std::process::exit(2);
        }
    }
}
