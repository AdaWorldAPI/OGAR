# OGAR Agent Instructions

> Rules for AI coding agents working in this repository. Adopting the
> conventions used by AdaWorldAPI's surrealdb fork.

## Repository purpose

OGAR (Open Graph of Active Record) is a **vocabulary repository first,
code crate second**. The canonical artifacts are the TTL and SurrealQL
files in `vocab/`; the Rust crates in `crates/` are producer/consumer
convenience built on top.

When in doubt: **the vocabulary is the contract; the code conforms**.

## Repository layout

```
OGAR/
├── crates/
│   ├── ogar-vocab/       — canonical Rust IR types
│   ├── ogar-ontology/    — prefix conventions + identity helpers
│   ├── ogar-emitter/     — Emitter trait + Triple type (Sprint 1)
│   ├── ogar-from-ruff/   — Ruby AR adapter (Sprint 1)
│   ├── ogar-to-surrealql/— SurrealQL DDL emitter (Sprint 2)
│   ├── ogar-to-postgres/ — PostgreSQL DDL emitter (Sprint 3)
│   ├── ogar-python/      — Python AR producer (Sprint 4)
│   └── ogar-ext-odoo/    — Odoo-specific extensions (Sprint 5)
├── vocab/
│   ├── ogar.ttl          — Turtle/RDF canonical vocabulary
│   ├── ogar.json-ld      — JSON-LD projection (planned)
│   └── ogar.surql        — SurrealQL DDL projection
├── docs/
│   └── ARCHITECTURE.md   — full layer-stack writeup
└── .claude/
    ├── VISION.md         — the one-page vision
    ├── PLAN.md           — sprint-by-sprint roadmap
    ├── AGENTS.md         — this file
    └── board/
        └── EPIPHANIES.md — append-only findings log
```

## Extension rules

### Adding to the canonical vocabulary

The base `ogar/` vocabulary covers Active-Record-pattern essentials.
Adding a new type to `crates/ogar-vocab/` requires:

1. **Naming**: PascalCase struct, language-neutral name.
2. **Field discipline**: every field maps to a vocabulary term in
   `vocab/ogar.ttl`. Add the term first; add the field second.
3. **Comment per field**: explain *why* the field exists, not what the
   type is. Refer to the gap-probe taxonomy if applicable.
4. **Test**: at minimum a `Default::default()` test and a round-trip
   test through a representative producer.
5. **No language-specific fields on base types.** If a field only
   makes sense for one ORM, it goes in `ogar-extensions/<lang>/`,
   not on the base struct.

### Adding a per-language extension

```
crates/ogar-ext-<lang>/    — Rust types
vocab/ogar-ext-<lang>.ttl  — RDF terms
```

Extensions extend, never modify. They register additional fields keyed
to an existing canonical `ogar/Class`. Producers populate the
extension struct alongside the canonical class.

### Adding a producer (extracts source → OGAR IR)

A producer crate (`ogar-<source>` or `ogar-from-<source>`) implements:

```rust
pub fn extract(source: &Path) -> Vec<ogar_vocab::Class>;
```

Producers are **single-responsibility**: they parse source and emit
canonical IR. They do not interpret semantics beyond the vocabulary.
Cross-ORM differences live in extensions, not in producers.

### Adding a consumer (OGAR IR → target form)

A consumer crate (`ogar-to-<target>`) implements:

```rust
impl ogar_emitter::OgarEmitter for ToTarget {
    fn emit_class(class: &ogar_vocab::Class, prefix: &str) -> Vec<Triple>;
    // ... one method per top-level vocab type
}
```

OR for non-triple outputs (DDL, OpenAPI):

```rust
pub fn emit(classes: &[ogar_vocab::Class], prefix: &str) -> String;
```

## Prefix conventions

| Prefix              | Use                                                    |
|---------------------|--------------------------------------------------------|
| `ogar/`             | core vocabulary terms (Class, Association, Field …)    |
| `ogar-extensions/<lang>/` | language-specific extension vocabulary           |
| `ogit/`             | OGIT baseline (IT-ops semantics)                       |
| `ogit-erp/`         | shared ERP business semantics                          |
| `ogit-<app>/`       | application-specific extensions (op, gitlab, redmine…) |

Adding a new top-level prefix is an **ontology-level decision**: open
a discussion before introducing one. Most needs are met by a deeper
segment under an existing prefix.

## EPIPHANIES log

Append-only findings log at `.claude/board/EPIPHANIES.md`. Conventions:

- **Newest at top.** Always.
- **Each entry**: `## YYYY-MM-DD — short headline` + `**Status:**` line
  (FINDING / CONJECTURE / FRAMING / SUPERSEDED) + `**Scope:**` line +
  body + `**Cross-ref:**` line.
- **Body and date immutable.** Corrections append as new entries citing
  the original.
- **Only the Status line is mutable** (e.g. CONJECTURE → SUPERSEDED-BY
  with a date pointer).

This is for cross-cutting findings (architectural insights, bugs that
recur across crates, naming drifts). One-shot file-local notes go in
code comments, not here.

## Commit messages

- First line: < 72 characters.
- Then blank line.
- Then detailed body explaining *why* the change, with refs to the
  sprint number in `PLAN.md`.
- Reference cross-cutting EPIPHANIES entries by date when applicable.

## When making changes

1. **Read `VISION.md` first.** All architectural decisions trace back
   to it.
2. **Find the sprint in `PLAN.md`.** If your change isn't in a planned
   sprint, propose a new sprint before coding.
3. **Update the vocab files** if you're adding vocabulary. Vocab edits
   are the contract change; code edits are the conformance.
4. **Run `cargo check --workspace && cargo test --workspace`** before
   commit.
5. **Log cross-cutting findings** to EPIPHANIES if they would help a
   future agent or human avoid a wrong path.

## Brutal-review discipline

Before opening a PR, run a brutal-review pass on your own work. Three
angles:
- **Correctness**: what edge cases break? what assumption could be
  wrong?
- **API ergonomics**: is the trait/type shape composable? would a
  consumer write awkward code?
- **Scope creep**: are you doing more than the sprint asks? cut it
  back.

If you can't honestly answer all three, the PR isn't ready.

## Forbidden

- Adding language-specific fields to canonical `ogar-vocab` types.
- Reading or writing repositories outside `AdaWorldAPI/OGAR` (this
  repo) without explicit instruction.
- Cluttering `.claude/` with planning docs that don't survive the
  sprint they planned for. PLAN.md is the live roadmap; sprint-local
  notes belong in PR descriptions.
- Bypassing the vocabulary contract. If something needs a new field,
  add it to `vocab/ogar.ttl` first.
