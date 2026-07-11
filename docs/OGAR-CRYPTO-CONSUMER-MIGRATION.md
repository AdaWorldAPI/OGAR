# OGAR crypto — consumer migration plan (ogar-encryption / ogar-auth / ogar-rbac)

> **Goal (operator):** make `ogar-auth` / `ogar-encryption` / `ogar-rbac`
> globally reusable for all consumers, backed by `AdaWorldAPI/crypto` (= the
> ndarray `encryption` crate) wired through the `ndarray::simd::*` hardware-
> acceleration layer (server AVX-512 / browser wasm128). Argon2id + XChaCha20-
> Poly1305 etc. run on that one accelerated spine, native and wasm.
>
> **Status:** the generic surface exists — `ogar-encryption` (this repo, thin
> re-export of the `encryption` crate), `ogar-auth` (password/totp/legacy on top),
> `lance-graph-rbac` (the `0x0B` auth membrane / `ClassRbac`). The
> `ndarray::simd::chacha20_block` ARX primitive (ndarray, RFC 8439 KAT-green) is
> the acceleration foundation the `encryption` AEAD adopts next.

## The one crypto+SIMD spine

```
ndarray::simd::chacha20_block   (ARX keystream, AVX-512 / wasm128 / NEON / scalar)
        ▲ hot path
ndarray crates/encryption       (Argon2id · XChaCha20-Poly1305 · Ed25519 · SHA-384 · envelope · wasm)
        ▲ re-export (never re-implement)
ogar-encryption                 (generic, classid-agnostic raw-crypto surface)   ← ALL consumers pull this
        ▲
ogar-auth                       (+ password PHC · RFC-6238 TOTP · legacy 3DES bridge)
lance-graph-rbac                (authorize() · ClassRbac · 0x0B membrane)         ← the "ogar-rbac" role
```

Consumers pull `ogar-encryption` for raw crypto, `ogar-auth` for login/2FA,
`lance-graph-rbac` (directly, or via `lance-graph-callcenter`'s re-export where a
BBB allowlist forbids the direct dep) for authorization. **Never hand-roll
argon2/chacha/ed25519; never copy the codebook.**

## Migration tiers (from the 2026-07-11 cross-repo reusability audit)

### Tier 1 — byte-compatible, drop-in (same `Argon2::default()` → identical PHC)
| Consumer | Site | → pull |
|---|---|---|
| MedCare-rs | `medcare-core::crypto::{hash,verify}_password` | `ogar-auth::password` |
| MedCare-rs | `medcare-core::totp` (near-verbatim duplicate) | `ogar-auth::totp` |
| woa-rs | `src/auth/mod.rs::{hash_password,verify_argon2id}` | `ogar-auth::password` |
| openproject-nexgen-rs | `op-auth::api_key` Argon2 arm | `ogar-auth::password` |

### Tier 2 — data-compat (different algorithm/format → coordinated rekey, not drop-in)
| Consumer | Site | Note |
|---|---|---|
| MedCare-rs | `medcare-core::crypto` AES-256-GCM (DMS blobs) | XChaCha20-Poly1305 ≠ AES-GCM; decrypt-old/encrypt-new pass over `pma_dokument` + key rotation |
| MedCare-rs | `legacy_crypt.rs` (3DES-EDE2/PBKDF1-MD5) | `ogar-auth::legacy` exists but BOTH sides UNVERIFIED vs prod ciphertext — pair with the byte-parity vector work, don't swap blind |
| woa-rs | RFC-005 Fernet→chacha vault (unimplemented) | build straight onto `ogar-encryption::{aead,kdf}`, amend RFC-005 first |
| Sharepoint-rs | `smb-policy-encryption` (`aes_gcm_kv` stub, no Cargo.toml) | greenfield — scaffold on `ogar-encryption` (XChaCha), drop the AES-GCM name |

### Tier 3 — keep-local (justified)
- Content-integrity **SHA-256** (MedCare hardware/licence/dms; woa GoBD audit-chain; op-attachments) — hashing records, not secrets; the envelope's SHA-**384** is a different width for its own use.
- woa-rs legacy salted-SHA256 portal format — pinned to Python `hashlib.sha256(salt+pw)` for writer-parity.
- **JWT session mint** (MedCare-rs, openproject) — a genuine *surface gap*: no generic JWT/session type in ogar-auth yet (candidate `ogar-auth::session`).
- spider PKCE SHA-256 — RFC 7636, non-secret.

## RBAC ("ogar-rbac") status
- **MedCare-rs** `medcare-rbac` and **woa-rs** `unified_bridge.rs` already route `Policy` through `lance_graph_rbac` — the reference pattern (woa via callcenter's re-export to honor its BBB allowlist).
- **openproject-nexgen-rs** `op-auth::permissions.rs` hand-rolls a bespoke permission engine → should map OpenProject permission *names* over `lance_graph_rbac::policy::Policy`, not a parallel engine.
- **Sharepoint-rs** `smb-policy-tenant-rbac` — declared workspace member, no source; scaffold on `lance-graph-rbac` from the first commit.

## Blockers to confirm before wiring
- **woa-rs BBB-Barriere** (`.claude/CLAUDE.md` §3 Iron Rule 1): allowlist = `lance-graph-{contract,ontology,callcenter}`; `lance-graph-rbac` is VERBOTEN direct. `ogar-auth`/`ogar-encryption` are a *different namespace*, named in neither list → **needs an explicit allowlist ruling** before woa-rs deps them. (MedCare-rs & openproject have no such barrier — same-pattern add.)
- **Sharepoint-rs** won't `cargo build` today (3 members missing `Cargo.toml`, 1 missing directory) — nothing to migrate yet; fix by scaffolding onto the generic surface.

## Sequencing
1. Adopt `ndarray::simd::chacha20_block` inside the `encryption` AEAD hot path (behind a feature; parity vs RustCrypto on RFC 8439 vectors) — the acceleration the goal names. **(next)**
2. Land the AVX-512 / wasm128 backends of `chacha20_block` (parity vs the scalar KAT).
3. Tier-1 drop-in migrations (password/totp) — start with MedCare-rs (no barrier).
4. Confirm the woa-rs allowlist ruling; then woa-rs Tier-1.
5. Tier-2 data-compat migrations behind their rekey plans.
6. Fill the JWT-session gap (`ogar-auth::session`) if the consumers want to converge it.
