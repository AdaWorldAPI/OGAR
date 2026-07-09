# OGAR Crypto-Kernel SIMD Integration Plan (v1)

> **Goal:** OGAR becomes the crypto-kernel beast — SIMD-accelerated crypto in
> **one codebase** that runs native (AVX-512 / AVX2 / NEON) **and** in a stock
> wasm32 browser, **audited-reuse-first** (never hand-roll AEAD or curve math).
> Not a today-sprint; a step-at-a-time roadmap with the fork decisions made
> explicit.

Status: v1 authored from direct source recon. Per-kernel backend matrices are
being deepened by a research fan-out; refinements land as v1.1. The `ogar-auth`
council (crypto/security review of the shipped crate) runs in parallel.

---

## 0. TL;DR

- The crypto lives in `ndarray/crates/encryption` (XChaCha20-Poly1305, Ed25519,
  SHA-384, Argon2id), re-exported by OGAR's `crates/ogar-auth`.
- The acceleration substrate is **`ndarray::simd`** — a compile-time polyfill
  that dispatches one agnostic logical type down to whatever the silicon has.
- **The whole plan hinges on ONE keystone gap** (§2): the *wide integer* logical
  types (`U32x16`, `U64x8`, `I32x16`) are AVX-512-native on x86 but **fall back
  to scalar on wasm**. Back them with `[v128;4]` (mirroring the float wides that
  already work) and agnostic ChaCha/BLAKE2b light up in the browser.
- **Patch the CIPHER, not the AEAD.** `chacha20poly1305` is a thin composition
  of `chacha20` + `poly1305`; a SIMD backend added to `chacha20` is inherited by
  the AEAD and by `encryption` **with zero changes to audited AEAD code**.

---

## 1. The agnostic-SIMD principle

You write crypto **once** against a wide logical vector and the polyfill maps it:

| Logical type | AVX-512 | AVX2 | NEON | wasm128 | scalar |
|---|---|---|---|---|---|
| `F32x16` / `F64x8` | 1× zmm | 2× ymm | 4×128 | **`[v128;4]`** ✓ | array |
| `U32x16` / `U64x8` / `I32x16` | 1× zmm ✓ | (verify) | (verify) | **scalar ✗** | array |

`U32x16` is **literally one ChaCha20 block** (16 u32 words). `U64x8` is
**BLAKE2b's state** (8 u64). So the crypto kernels are natural fits for the wide
types — the only thing missing is the wasm (and possibly NEON/AVX2) *integer*
backing.

---

## 2. THE KEYSTONE (Step 0, gates everything else)

**Finding (verified):** `ndarray/src/simd_wasm.rs:75`
```rust
pub use crate::simd::scalar::{I32x16, U32x16, U64x8};
```
The wide *integer* types are re-exported from the **scalar** fallback on wasm.
The `[v128;4]` treatment exists only for `F32x16`, `F64x8`, and `I8x16`. So
`ndarray::simd::U32x16` in a browser is plain `[u32;16]` — unaccelerated.

**Keystone task:** implement `U32x16` and `U64x8` as `[v128;4]` in
`simd_wasm.rs` (mirror the existing `F32x16([v128;4])`), with the crypto op-set:
`wrapping_add`, `xor`, `shl::<N>`, `shr::<N>` (logical), and `rotl::<N>` composed
as `(x << n) | (x >>u (32-n))` (wasm has no rotate instruction). Honor the **W1a
consumer contract** (`.claude/knowledge/vertical-simd-consumer-contract.md` in
ndarray): struct-method wrappers, all backends implemented, mandatory
scalar-parity test.

**Sub-task:** verify whether NEON/AVX2 also route the wide integers to scalar
(the backend inventory shows `U32x16`/`U64x8` defined only in `simd_avx512`).
Mobile (NEON) matters; if it's scalar there too, the keystone covers NEON as
well.

**Verifiable HERE (host):** the avx512 ↔ scalar parity test runs in CI now.
**Not verifiable here (wasm):** no wasm runtime in the container (only `node`).
On-target wasm parity needs a `node`/`wasmtime` CI gate (§7).

---

## 3. Architecture — the layering (patch the cipher, not the AEAD)

```
ndarray::simd          bricks: U32x16 / U64x8 (+ rotate), one W1a contract
     ▲ consumes
chacha20 / argon2      VENDOR+PATCH: a wasm+simd128 backend that uses the bricks
  (RustCrypto crates)  — the ONLY code we add crypto SIMD to
     ▲ inherited by
chacha20poly1305 /     UNCHANGED, audited. Thin composition (chacha20+poly1305).
XChaCha20Poly1305      Inherits the backend for free.
     ▲ used by
encryption (ndarray)   UNCHANGED. seal/open, kdf, hash, sign, envelope.
     ▲ re-exported by
ogar-auth (OGAR)       the consumer SDK: forward re-export + password/totp/legacy
```

**The injection point (verified):** `chacha20/src/backends.rs` is a `cfg_if`
ladder — `soft` / `x86(avx2,sse2)` / `aarch64(neon)` / `else soft`. Adding a
`wasm32 + simd128` arm → `backends/simd128.rs` is the single, clean seam.
`XChaCha20Poly1305` (24-byte nonce, what `encryption` uses) inherits it.

---

## 3.5 Fork coverage — the obligatory forks (P0: never crates.io)

Per the workspace P0 rule (*always depend on the AdaWorldAPI fork; never the
crates.io version of a forked crate*), the SIMD backends are patched into the
**AdaWorldAPI forks**, and `encryption` must depend on them.

**Fork coverage (verified via API 2026-07-09):**

| RustCrypto repo | Holds | AdaWorldAPI fork | Golden-path role |
|---|---|---|---|
| `AEADs` | chacha20poly1305, aes-gcm | ✅ **forked** | AEAD wrapper (no cipher SIMD here) |
| `hashes` | sha2, **blake2**, sha1 | ✅ **forked** | BLAKE2b SIMD patchable here |
| `stream-ciphers` | **chacha20** | ❌ **not forked** | **ChaCha20 SIMD backend lives here** |
| `password-hashes` | **argon2** | ❌ **not forked** | **Argon2 block-mix SIMD lives here** |
| `universal-hashes` | poly1305 | ❌ not forked | Poly1305 stays scalar |
| `block-ciphers` | aes | ❌ not forked | AES = native AES-NI / WebCrypto |

**The gap:** the two P0 kernels (ChaCha20, Argon2) live in `stream-ciphers` /
`password-hashes`, which are **not forked yet**; and `AdaWorldAPI/AEADs`'s
`chacha20poly1305` deps **crates.io** `chacha20 = "0.10"`, so even the AEAD fork
pulls the cipher unforked. **The obligatory forks still to create are
`AdaWorldAPI/stream-ciphers` (chacha20) and `AdaWorldAPI/password-hashes`
(argon2)** — you literally cannot patch a SIMD backend into a crate you don't
own a fork of.

**The obligatory re-wire (Step 0.0):** `encryption/Cargo.toml` currently deps
crates.io (`chacha20poly1305="0.10"`, `sha2`, `argon2`, `ed25519-dalek`) — a P0
violation. Before any SIMD can be *consumed*, point `encryption` at the forks
and add `[patch.crates-io]` at the ndarray workspace root so the WHOLE tree
(including `chacha20poly1305`'s transitive `chacha20`) resolves to forks:
`chacha20poly1305`→AEADs, `sha2`/`blake2`→hashes, `chacha20`→stream-ciphers,
`argon2`→password-hashes (the last two once forked). *This is the "obligatory
fork to be able to patch": you cannot patch-and-consume a backend the dep graph
doesn't point at.*

---

## 4. Per-kernel plan

| Kernel | Role | SIMD fit | ndarray brick | Priority | Upstream home |
|---|---|---|---|---|---|
| **XChaCha20** | AEAD encrypt | **excellent** (u32 add/xor/rotl) | `U32x16` | **P0** | RustCrypto/stream-ciphers (`chacha20`) |
| **Poly1305** | AEAD MAC | poor (130-bit carries) | — | scalar | RustCrypto/universal-hashes |
| **Argon2id / BLAKE2b** | password KDF | good compute (`G`=u64), memory-bound total | `U64x8` + raw `v128` | **P1** | RustCrypto/password-hashes + hashes |
| **Ed25519** | signatures | poor (255-bit field) — **never hand-roll** | native dalek-simd only | P1 (native), skip (wasm) | dalek-cryptography |
| **AES-256-GCM** | native-fast AEAD | needs AES-NI/CLMUL (absent in wasm) | — | P1 (native), delegate (browser) | RustCrypto/block-ciphers + AEADs |
| **SHA-384/512** | digest / fingerprint | modest (`u64x2`, 2 lanes) | — | **skip** unless bulk | RustCrypto/hashes |
| **HMAC-SHA384 (HS384)** | symmetric MAC (tokens/claims) | n/a (1 MAC/token) | — | **skip** | reuses `sha2` |
| **SHA-256 / HMAC** | compat MAC | `u32x4`, only bulk | — | skip | RustCrypto/hashes |
| **TOTP / HMAC-SHA1** | authenticator compat | none | — | **skip** (ceremonial) | in `ogar-auth` |
| **Legacy 3DES / PBKDF1-MD5** | migration read-path | none | — | **skip** | in `ogar-auth` — correctness first (§8) |

Notes:
- **BLAKE2b is already in-tree** (via `argon2`) — a fast, `u64x2`-friendly,
  non-NIST hash one `use` away if a hash hotpath ever appears.
- **BLAKE3** is the only hash that would *justify* wasm SIMD (tree-parallel,
  SIMD-native). Reach for it only if a **bulk fingerprint/identity** hotpath
  emerges; otherwise SHA-384's audit-boringness wins and hash-SIMD is skipped.
- **SHA-384 = SHA-512 machinery + distinct IV, truncated to 48 B.** It is a
  digest-SIZE / length-extension-safety choice, **not** a throughput lever.

---

## 5. Decision axes — the "multiple options" (operator picks)

**Axis 1 — acceleration strategy. RESOLVED by §3.5.**
The AdaWorldAPI-fork-first P0 rule already picks the mechanism: **patch the
AdaWorldAPI fork** (create it if missing), consume via `[patch.crates-io]`. It's
not "upstream vs fork" — the fork *is* the home. Per kernel:
- **ChaCha20** → fork `AdaWorldAPI/stream-ciphers`, add `backends/simd128.rs`
  consuming `ndarray::simd::U32x16`.
- **BLAKE2b** → patch `AdaWorldAPI/hashes/blake2` (already forked).
- **Argon2** → fork `AdaWorldAPI/password-hashes`, SIMD the block-mix over `U64x8`.
- **AES-GCM** → `AdaWorldAPI/AEADs/aes-gcm` (already forked) — native AES-NI path.
- Optionally upstream the same backends to RustCrypto later; the fork ships it
  either way. (Poly1305 / AES-cipher forks only if their kernels ever matter.)

**Axis 2 — AEAD posture.**
- ChaCha-only everywhere (current): one envelope format, portable, consistent.
- **Dual-AEAD with an algorithm tag:** AES-256-GCM on AES-NI/PMULL hardware
  (brutally fast on servers), XChaCha20-Poly1305 as the portable/wasm path.
- **Recommendation:** stay **ChaCha-only** for v1 (consistency + the wasm win is
  the whole point). Revisit dual-AEAD only if a *native* bulk-throughput
  workload proves AES-NI worth the envelope-format complexity.

**Axis 3 — browser hardware path.**
- Pure-wasm-SIMD (our ChaCha/BLAKE2b kernels).
- **Delegate to WebCrypto SubtleCrypto** (AES-GCM/SHA/HKDF are hardware +
  audited in-browser) — but it's async/JS-boundary and doesn't cover ChaCha.
- **Recommendation:** pure-wasm-SIMD for the ChaCha/Argon2 golden path (keeps the
  one-codebase invariant); keep WebCrypto in the back pocket for any AES path.

---

## 6. Phasing — one step at a time

| Step | Deliverable | Verifiable here? |
|---|---|---|
| **0.0** | **Obligatory forks** — re-wire `encryption` onto AdaWorldAPI forks + `[patch.crates-io]`; create `AdaWorldAPI/stream-ciphers` + `password-hashes`. No behavior change, just fork-correct deps. | `cargo tree` shows forks; gates green ✓ |
| **0** | ndarray `U32x16`/`U64x8` as `[v128;4]` on wasm + `rotl` + W1a parity test | host avx512↔scalar ✓; wasm via §7 CI |
| **1** | vendor+patch `chacha20` → `backends/simd128.rs` consuming `U32x16`; RFC 8439 byte-parity. AEAD + `encryption` untouched | scalar twin vs RFC vector ✓; wasm via §7 |
| **2** | confirm/patch NEON + AVX2 wide-integer backing (mobile) | host ✓ |
| **3** | Argon2/BLAKE2b block-mix over `U64x8` + raw `v128` (measure vs memory-bound ceiling) | host + KAT vectors ✓; wasm via §7 |
| **4** | enable native Ed25519 dalek-simd (free server win); optional dual-AEAD AES-NI (Axis 2) | host ✓ |
| **5** | measure; only then decide SHA/BLAKE3 SIMD (Axis: bulk-hash hotpath?) | — |

Each step is independently shippable and leaves the tree green. **Step 0 is the
keystone** — steps 1/3 depend on it.

---

## 7. Verification strategy

- **Host (CI now):** the W1a contract's mandatory backend-parity tests
  (avx512 ↔ avx2 ↔ neon ↔ scalar) already prove the SIMD types lane-for-lane.
- **wasm (new gate):** the container has **no wasm runtime** (only `node`).
  Add a CI job: build the kernel to `wasm32-unknown-unknown` with
  `-C target-feature=+simd128`, run it under `node` (or `wasmtime`), and assert
  byte-parity against the published vectors:
  - ChaCha20 → **RFC 8439 §2.3.2**
  - BLAKE2b → **RFC 7693**
  - (Ed25519 native → RFC 8032; Argon2 → the reference KATs)
- **Iron rule:** no wasm SIMD kernel ships without its `node` parity gate green.
  (Noise/uniform fixtures hide decode bugs — use real published vectors.)

---

## 8. Non-goals / guardrails

- **Never hand-roll** Ed25519 field arithmetic or AES bitslicing — audited crates
  only (dalek / RustCrypto). SIMD for these = native backend flags, not new math.
- **Do not touch the AEAD composition.** Patch `chacha20` (the cipher); leave
  `chacha20poly1305` / `XChaCha20Poly1305` / `encryption` unchanged.
- **Legacy 3DES:** correctness before speed. `ogar-auth`'s `legacy.rs` uses the
  standards-correct **`TdesEde3`** (EDE) — note medcare-rs's reference uses
  `TdesEee3` (EEE), which is a latent bug; both are **unverified against
  production ciphertext pending the canonical `Crypt.cs` + a real
  (plaintext,ciphertext) vector.** No SIMD until byte-parity is proven.
- **SHA / HMAC / TOTP / HS384:** perf-noncritical (one op per token/30 s) —
  SIMD-skip on purpose.

---

## 9. Open decisions for the operator

1. **Axis 1** — confirm "vendor+patch `chacha20` now, upstream PR in parallel."
2. **Axis 2** — confirm "ChaCha-only for v1" (defer dual-AEAD/AES-NI).
3. **Axis 3** — confirm "pure-wasm-SIMD golden path" (WebCrypto only as AES fallback).
4. Green-light **Step 0.0** (create `AdaWorldAPI/stream-ciphers` + `password-hashes`
   forks, re-wire `encryption` onto all forks) — the true first patch — then
   **Step 0** (the `U32x16`/`U64x8` wasm backing).

---

_Cross-refs: `ndarray/src/simd.rs` (dispatch), `ndarray/src/simd_wasm.rs:75`
(the keystone gap), `chacha20/src/backends.rs` (injection point),
`ndarray/crates/encryption/src/aead.rs` (XChaCha20Poly1305 consumer),
`crates/ogar-auth` (the OGAR surface), ndarray
`.claude/knowledge/vertical-simd-consumer-contract.md` (W1a)._
