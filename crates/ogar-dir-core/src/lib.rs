//! # ogar-dir-core — observed directory reality, as bytes
//!
//! Shared substrate for `ogar-ad` (Active Directory / LDAP) and `ogar-az`
//! (Entra ID / Microsoft Graph). It answers exactly one question: *how is one
//! observation of one directory object laid out?* It is not an IAM, does not
//! provision, reconcile or apply business rules.
//!
//! The four axes are kept orthogonal:
//!
//! | axis  | carrier                                   | module      |
//! |-------|-------------------------------------------|-------------|
//! | WHO   | [`Guid128`] source object id              | [`guid`]    |
//! | WHERE | scope [`Guid128`] + [`OuHhtl`] (ingress); [`DirectoryScope`] × [`Dn128`] (execution) | [`hhtl`], [`dn128`] |
//! | WHAT  | schema-defined attribute slots + [`ValuePool`] | [`record`], [`pool`], [`schema`] |
//! | LINKS | [`DirEdge`] records, never inline lists   | [`edge`]    |
//! | WHEN  | `observed_at_ms` only (provenance hook)   | [`record`]  |
//!
//! ## Relation to the OGAR canonical node
//!
//! [`DirRecord`] is exactly 512 bytes and keeps the canonical
//! `key(16) | edges(16) | value(480)` split of lance-graph-contract `NodeRow`.
//! The 16-byte canonical key is a MINTED address (`classid` + 12-byte facet)
//! and is therefore **not** the source GUID: a 128-bit external GUID cannot be
//! placed in the key without truncation. In this PoC the key and edge facet
//! are left zero (the documented zero-fallback "default class, dormant"), and
//! the authoritative source GUID lives in the value slab. See `docs/` in the
//! workspace for the conflict note.

pub mod base64;
pub mod dn;
pub mod dn128;
pub mod edge;
pub mod guid;
pub mod hhtl;
pub mod label;
pub mod pool;
pub mod record;
pub mod schema;

pub use dn::{Dn, DnError, Rdn};
pub use dn128::{DN_CHILDREN, DN_LEVELS, DirectoryScope, Dn128, Dn128Error};
pub use edge::{DirEdge, EdgeEvidence, EdgeKind};
pub use guid::{Guid128, GuidParseError};
pub use hhtl::{HhtlError, OU_LEVELS, OuDictionary, OuHhtl};
pub use label::CloudLabel;
pub use pool::{PoolError, StrRef, ValuePool};
pub use record::{ABI_MAJOR, ABI_MINOR, DirRecord, GUID_SLOTS, RECORD_BYTES, RecordError};
pub use schema::{AttrDef, AttrKind, SchemaFamily, SchemaId, SlotSpace};
