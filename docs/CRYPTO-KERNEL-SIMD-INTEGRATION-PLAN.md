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
- **Acceleration ordering `ndarray-native ≥ wasm > WebGPU` — with one flip.**
  ndarray-native (AVX-512/NEON) and wasm-SIMD are **the same polyfill code**, not
  a priority choice — one substrate, two compile targets. WebGPU/WebGL (§5 Axis 3
  Tier 3) is a *separate* build. For the **latency-bound** auth/envelope path
  (OGAR's actual workload) the order holds: CPU-SIMD > GPU. It **flips only for
  bulk throughput** (encrypting/hashing large volumes), where a GPU can beat both
  CPUs — so GPU is reserved for that case, never the default. (On a *server*, the
  bulk winner is a native GPU/CUDA, not WebGL — out of the browser scope here.)
- **Effort ≈ a weekend, because the palette already paid the hard 90%.** The
  polyfill, the 5-backend dispatch, the wide `U32x16`/`U64x8` types, the W1a
  parity harness, and the `[v128;4]` wasm pattern all already exist (proven on
  the float wides + the palette distance kernels). Crypto adds only a `rotl` op +
  two small kernels (ChaCha double-round, BLAKE `G`) riding that substrate.

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

| RustCrypto repo | Holds | AdaWorldAPI fork | Fork ver | Golden-path role |
|---|---|---|---|---|
| `AEADs` | chacha20poly1305, aes-gcm | ✅ forked | 0.11.0 | AEAD wrapper (no cipher SIMD here) |
| `hashes` | sha2, **blake2**, sha1 | ✅ forked | 0.11.x | BLAKE2b SIMD patchable here |
| `stream-ciphers` | **chacha20** | ✅ **forked** | 0.10.1 | **ChaCha20 SIMD backend lives here** |
| `universal-hashes` | poly1305, ghash | ✅ **forked** | 0.9.1 | Poly1305 (stays scalar) |
| `password-hashes` | **argon2** | ❌ not forked | — | **Argon2 block-mix SIMD lives here** |
| `block-ciphers` | aes | ❌ not forked | — | AES = native AES-NI / WebCrypto |

**Still to fork:** `password-hashes` (argon2 — the Argon2 golden-path kernel has
no fork home yet) and `block-ciphers` (aes).

**⚠ Version skew — the real integration cost.** The forks track the
**bleeding-edge RustCrypto `0.11.x` / `0.9.x`** line; `encryption` is pinned to
stable **`0.10.x`** (`chacha20poly1305 0.10`, `sha2 0.10`, `argon2 0.5`,
`poly1305 0.8`). This reshapes the wiring:
- **`[patch.crates-io]` cannot bridge a major gap** — patching `0.10` with the
  `0.11` fork just emits *"patch … was not used in the crate graph"* (the
  workspace's own policy-alert). `[patch]` is the wrong tool here.
- **A direct git dep does work** (`chacha20poly1305 = { git = ".../AEADs" }`) —
  cargo takes whatever version the fork ships, sidestepping the semver-match
  wall. But then `encryption` must compile against the **`0.11` API** (RustCrypto
  reshuffled the `aead`/`digest` trait surfaces at 0.11), so Step 0.0 carries a
  **small API migration** in `aead.rs`/`hash.rs`, not a no-op.
- The forks must be **internally version-consistent** (AEADs 0.11's
  `chacha20poly1305` must accept the `chacha20 0.10.1` that stream-ciphers
  ships) — verify during the migration.

**Step 0.0 (revised):** migrate `encryption` to the `0.11` crypto line via **git
deps onto the forks** (not `[patch]`); fork `password-hashes`/`block-ciphers` at
matching versions (argon2 0.6, aes 0.9). *This is "the obligatory fork to be able
to patch": once `encryption` consumes the fork, a single SIMD patch in the fork
propagates to every consumer.*

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
- **Hash role-split (not either/or):** for a *fast* hash, **BLAKE3** is the pick
  — tree-parallel and SIMD-native (the one hash that pays back SIMD, incl. GPU
  Tier 3), so **`chacha20poly1305` + BLAKE3** is the fast modern pairing.
  **SHA-384** stays as the *conservative / NIST-audit* envelope-identity digest
  where boringness > speed. Use BLAKE3 for bulk/throughput hashing, SHA-384 for
  the audit-facing identity — **both, different roles**.
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

**Axis 3 — browser hardware path (three tiers).**
- **Tier 1 — wasm-CPU-SIMD** (our ChaCha/BLAKE kernels over `ndarray::simd`).
  The one-codebase golden path; right for the *latency-bound* case (one login,
  one envelope).
- **Tier 2 — WebCrypto SubtleCrypto** (AES-GCM/SHA/HKDF are hardware + audited
  in-browser) — async/JS-boundary, doesn't cover ChaCha; a back-pocket AES path.
- **Tier 3 — borrowed browser GPU (WebGPU / WebGL compute).** A *distinct* path
  (WGSL shaders, not the ndarray Rust SIMD) — and it's "not nothing": the browser
  sandbox makes borrowing the client GPU a **bounded, consented** compute on the
  user's *own* data, unlike unsandboxed cryptojacking.
  - **Fits:** *bulk/throughput* client-side work — batch AEAD over many
    independent ChaCha blocks, bulk BLAKE3 hashing of large files (GPU loves
    embarrassingly-parallel keystreams).
  - **Does NOT fit:** single-op latency (host↔GPU transfer dominates), and
    **Argon2 by design** (memory-hard specifically to resist GPU parallelism —
    the defender gets no win; GPU-Argon2 is the *attacker's* tool).
  - **Cost:** separate WGSL kernels + `wgpu`/WebGPU plumbing + async dispatch +
    feature-detection (WebGPU still uneven, Safari lagging); added attack surface
    (shared-GPU timing side-channels).
- **Recommendation:** **Tier 1** for the golden path (latency-bound envelope /
  login — the whole point). Keep **Tier 2** for AES. Reserve **Tier 3** for a
  *proven bulk* client-side encrypt/hash workload (e.g. sealing large records
  in-browser) — real capability, separate build, and **never for Argon2**.

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
