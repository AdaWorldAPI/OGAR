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
> `ndarray::simd` ChaCha20 ARX keystream is **complete and trusted**: scalar
> reference + **AVX-512 (server)** + **wasm128 (browser)** backends, all
> byte-parity-proven against both the RFC 8439 KAT and the vetted RustCrypto
> `chacha20` implementation across the parameter space (`chacha20_keystream`,
> ndarray branch `claude/medcare-ruff-codebook-handover-5ulx0i`).

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

#### MedCare-rs Tier-1 — ✅ SHIPPED (2026-07-11, medcare-rs `6d2c372`)

`medcare-core::crypto::{hash,verify}_password` and the whole `medcare-core::totp`
module now **delegate to `ogar-auth`** (body-only, public signatures preserved).
Empirically byte-compatible: medcare-core's own crypto+totp tests — including the
RFC 6238 Appendix-B vectors and the Argon2 hash/verify round-trip — pass **through**
the delegated ogar-auth path (56/56). medcare-core / medcare-db / medcare-server all
compile. The dep-wiring decision below was resolved as **option (b)**: a root
`[patch."…/ndarray"] encryption = { path = "vendor/ndarray/crates/encryption" }`
folds `ogar-encryption`'s git `encryption` onto the locally-vendored fork copy — one
source, no git fetch, no OGAR change. (The `encryption` crate has no ndarray dep, so
the patch touches only that leaf.) The recipe + rationale below are retained as the
template for the woa-rs / openproject Tier-1 migrations.

Byte-compatibility was **confirmed by source read** (2026-07-11) before wiring: `ogar-auth::password`
is `Argon2::default()` (identical PHC to `medcare-core::crypto`), and `ogar-auth::totp`
is a near-verbatim copy of `medcare-core::totp` — same `STEP_SECONDS=30`/`DIGITS=6`/
`SKEW_STEPS=1`, same RFC 4648 base32, same RFC 4226 HOTP, same RFC 6238 Appendix-B
vectors, same `otpauth://` URI. So the migration is a **body-only delegation** that
preserves every `medcare-core` public signature (zero caller changes in medcare-db /
medcare-server):

1. `medcare-core/Cargo.toml`: add `ogar-auth = { path = "../../vendor/OGAR/crates/ogar-auth" }`.
2. `medcare-core/src/crypto.rs`: replace the `hash_password`/`verify_password` **bodies**
   with `ogar_auth::password::*`, mapping `AuthError → DomainError::Crypto`. Keep AES-GCM +
   SHA-256 local (Tier 2/3). 
3. `medcare-core/src/totp.rs`: replace `base32_{encode,decode}` / `hotp` / `code_at` /
   `verify_code` / `provisioning_uri` / `generate_secret_base32` **bodies** with
   `ogar_auth::totp::*`, mapping errors; keep the existing medcare tests (they now
   exercise the delegated path and must stay green). One signature mismatch to bridge:
   ogar-auth `generate_secret_base32()` is `AuthResult<String>` (getrandom) vs medcare's
   infallible `-> String` — either propagate the Result (touches 1-2 callers) or map a
   CSPRNG failure explicitly; do NOT paper over it.

**⚠ Dep-wiring decision the operator must make first** (why this is staged, not shipped):
`ogar-auth` → `ogar-encryption` → `encryption` is a **git** dep (`ndarray` `branch="master"`),
so depping `ogar-auth` into `medcare-core` git-fetches an `encryption`/`ndarray` tree into a
workspace that otherwise **vendors ndarray locally** (`vendor/ndarray -> ../../ndarray`). That
is the established `ogar-encryption` wiring, not a new inconsistency — but it means a second,
git-sourced ndarray-encryption alongside the locally-vendored one, plus a heavier first build.
Resolve before wiring: either (a) accept the git `encryption` pull as-is, or (b) repoint
`ogar-encryption`'s `encryption` dep at the local path so all consumers share one ndarray
source (the fork-policy-consistent option). Only `password`+`totp` are used here, but depping
`ogar-auth` pulls the whole forward suite (incl. the git `encryption`) regardless.

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
1. ~~Land the AVX-512 / wasm128 backends of `chacha20_block` (parity vs the scalar KAT).~~ **DONE.** Both backends shipped, byte-parity-proven vs the scalar KAT *and* vs the vetted RustCrypto `chacha20` across the parameter space. The keystream primitive is trusted.
2. **The `encryption` AEAD is NOT re-wired to the primitive — by security ruling, not omission.** XChaCha20-Poly1305 = HChaCha20 subkey + Poly1305 one-time-key + MAC/AAD/length framing, not just a keystream. Swapping only the keystream means hand-composing that authenticated construction — the "roll your own AEAD" footgun the stack forbids ("never roll your own crypto — wrap vetted RustCrypto"). The RustCrypto `XChaCha20Poly1305` stays. The accelerated primitive is for **raw-stream** use sites whose caller already owns a vetted MAC/framing (e.g. a future woa-rs RFC-005 vault built on `ogar-encryption::{aead,kdf}`), never for reimplementing an AEAD. See the doc-comment in `encryption/src/aead.rs`. **OPEN DECISION for the operator:** whether to add `ndarray` as an *optional* dep of the lean `encryption` crate to expose a raw-stream `apply_keystream` at all — it pulls a heavy dep and the crate's wasm build currently has an unrelated `getrandom` `wasm_js` config gap. Deferred pending that call.
3. Tier-1 drop-in migrations (password/totp → `ogar-auth`). **MedCare-rs DONE** (`6d2c372`, delegated + 56/56 green + all crates compile; dep-wiring resolved via the local-`encryption` patch). openproject-nexgen-rs next (no barrier, same pattern).
4. Confirm the woa-rs allowlist ruling (the `ogar-*` namespace is in neither BBB list); then woa-rs Tier-1.
5. Tier-2 data-compat migrations behind their rekey plans.
6. Fill the JWT-session gap (`ogar-auth::session`) if the consumers want to converge it.
