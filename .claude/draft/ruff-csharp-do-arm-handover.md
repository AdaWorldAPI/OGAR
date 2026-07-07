# DRAFT / HANDOVER — close out the C# (Roslyn) DO-arm harvest for MedCare

> **Why this lives in OGAR, not ruff:** ruff writes are currently
> **403-blocked** for this session (org-level — the Claude GitHub App needs
> (re)connecting for `AdaWorldAPI`, and/or `ruff` re-added to its scope;
> read works, every write path 403s: git-proxy, token-userinfo push, and a
> fresh token clone+push all return `Permission to AdaWorldAPI/ruff.git
> denied to AdaWorldAPI`). This handover is parked here so a session WITH
> ruff write access (and ideally `dotnet`) can finish the job.
>
> **Author's correction (read first):** an earlier message this session
> claimed "the C# DO-arm is the sole remaining *missing* arm." That was
> **wrong** — it was inferred from the loader crate's stale TEST FIXTURE,
> not from the harvester source. Reading `harvester/Program.cs` shows the
> DO-arm walk **already exists** (drafted, unverified). The real gap is
> narrower: verify it + cover it with a loader test + de-draft it.

---

## 1. Current state (ruff @ `844b42a` = main after #57)

| Piece | State | Evidence |
|---|---|---|
| C# DO-arm walk (`EmitBodyArm`) | **EXISTS, drafted, untested** | `ruff/crates/ruff_csharp_spo/harvester/Program.cs:97-105` (call site) + `:147-257` (impl) |
| Predicates it emits | `writes_field`, `reads_field`, `raises`, `calls`, `writes_if_blank` | Program.cs body of `EmitBodyArm` |
| Closed `Predicate` vocab | **already accepts all of them** | `ruff/crates/ruff_spo_triplet/src/triple.rs:100-456` (`ReadsField`/`WritesField`/`WritesIfBlank`/`Raises`/`Calls`) |
| `ruff_csharp_spo::load` | validates ndjson vs the closed vocab | `ruff/crates/ruff_csharp_spo/src/lib.rs:52` |
| **Loader test fixture** | **STALE** — exercises only the 6 THINK-arm predicates; comment falsely says that's "every predicate `Program.cs` can emit" | `ruff/crates/ruff_csharp_spo/src/lib.rs:60-86` |
| `reassemble(&[Triple]) -> ModelGraph` | ships | `ruff/crates/ruff_spo_triplet/src/reassemble.rs:89` |
| `ogar-from-ruff::lift_actions(&Model)` | ships, language-agnostic, consumes the effect facts | `OGAR/crates/ogar-from-ruff/src/lib.rs:574` |
| C++ DO-arm (the TEMPLATE to mirror) | ships (#57) | `ruff/crates/ruff_cpp_spo/src/clang_walker.rs:873` (`method_body_arm`) |
| Generic `(cap, classid)` derive | **SHIPPED this session** | `OGAR/crates/ogar-vocab/src/capability_registry.rs` `entries_from_actions` (OGAR commit `ec52376`) |

So the whole pull-in chain **C# → triples → reassemble → lift_actions →
entries_from_actions** is wired end to end; the only untrusted link is the
harvester's DO-arm, and the only *test* gap is the loader fixture.

---

## 2. Close-out checklist (in order)

- [ ] **(Rust, writable now — see §3 for the ready patch)** Extend the
  `ruff_csharp_spo` loader fixture to include the DO-arm predicates and
  correct the "every predicate" comment. This proves `load()` accepts what
  the harvester actually emits, and turns the stale comment true.
- [ ] **(needs `dotnet` + a C# corpus)** Build + run the harvester against
  a real MedCare tree:
  `dotnet run --project ruff/crates/ruff_csharp_spo/harvester/CSharpSpoHarvester.csproj -- <medcare-src> out.ndjson`
  then `ruff_csharp_spo::load(&fs::read_to_string("out.ndjson")?)` must be
  `Ok`. Confirm DO-arm rows appear (grep `writes_field`/`raises`/`calls`).
- [ ] **(review)** Diff `EmitBodyArm`'s semantics against the C++
  `method_body_arm` template (clang_walker.rs:873) for parity of
  conventions (reads exclude the assignment LHS; `calls` gated to the
  persistence-mutator set; `writes_if_blank` ⊆ `writes_field`; `raises`
  against the `exc:` namespace). The C# draft already follows these — sanity
  re-check on a real corpus, don't rewrite blind.
- [ ] **(after green)** De-draft: drop the "DTO ARM (DRAFT) … Untested"
  wording at Program.cs:97-104 and :147; leave the provenance note.
- [ ] **(OGAR side, then)** With real MedCare ActionDefs in hand
  (`lift_actions(&reassemble(load(ndjson)).models[..])`), author the one
  thin file `OGAR/crates/ogar-vocab/src/healthcare_actions.rs` that hands
  those defs to `entries_from_actions` and registers one
  `capability_registry::domain_tables()` entry +
  `HEALTHCARE_{SUBJECT_CLASSIDS,EXPECTED_EXECUTORS}`. Do NOT hand-author
  capabilities — they come from the harvest (`has_function` names). See
  the plan doc P3.
- [ ] **(medcare)** `medcare-rs` `HOT_PLUG` const + activation test
  (`resolve_hotplug` roundtrip). See `.claude/knowledge/hotplug-consumer-migration.md`.

---

## 3. Ready-to-apply patch (the one writable-now Rust change)

Replace the fixture doc-comment + ndjson body in
`ruff/crates/ruff_csharp_spo/src/lib.rs` (currently lines 60-86) with the
version below. It (a) corrects the "every predicate" claim, (b) adds one
DO-arm row per predicate the harvester emits, (c) bumps the count assert
from 6 to 11. Every added predicate is already in the closed vocab
(triple.rs:100-456), so a green load is the standing proof the loader
accepts the harvester's full DO-arm output.

```rust
    /// The shape the Roslyn harvester emits for one MedCare model. This
    /// fixture exercises *every* predicate `harvester/Program.cs` can emit —
    /// the THINK-arm structure (`rdf:type`, `inherits_from`, `has_field`,
    /// `field_type`, `has_function`, `is_static`) AND the DO-arm method-body
    /// effect facts `EmitBodyArm` produces (`writes_field`, `writes_if_blank`,
    /// `reads_field`, `raises`, `calls`) — so a clean load is the standing
    /// proof that the full emitted set stays inside the closed vocabulary. If
    /// the harvester grows a new predicate, it must be added to
    /// [`super::Predicate`] first, or this load fails.
    #[test]
    fn loads_and_validates_harvester_ndjson() {
        let ndjson = concat!(
            // ── THINK arm (structure) ──
            r#"{"s":"medcare:Patient","p":"rdf:type","o":"ogit:ObjectType","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient","p":"inherits_from","o":"medcare:DbBase","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient","p":"has_field","o":"medcare:Patient.kdnr","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient.kdnr","p":"field_type","o":"string","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient","p":"has_function","o":"medcare:Patient.Save","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient.Save","p":"is_static","o":"true","f":1.0,"c":0.9}"#,
            "\n",
            // ── DO arm (method-body effect facts, EmitBodyArm) ──
            r#"{"s":"medcare:Patient.Save","p":"writes_field","o":"medcare:Patient.status","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient.Save","p":"writes_if_blank","o":"medcare:Patient.createdAt","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient.Save","p":"reads_field","o":"medcare:Patient.kdnr","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient.Save","p":"raises","o":"exc:ValidationException","f":1.0,"c":0.9}"#,
            "\n",
            r#"{"s":"medcare:Patient.Save","p":"calls","o":"ctx.SaveChanges","f":1.0,"c":0.9}"#,
            "\n",
        );
        let triples = load(ndjson).expect("every harvester predicate is in the closed vocab");
        assert_eq!(triples.len(), 11);
        assert_eq!(triples[0].s, "medcare:Patient");
    }
```

Verify: `cargo test -p ruff_csharp_spo` (pure Rust, no dotnet needed).

> Optional hardening the verifying session may add: a second fixture with a
> deliberately out-of-vocab DO-arm-looking predicate (e.g. `writes_maybe`)
> asserting `load` errors — but `rejects_out_of_vocab_predicate`
> (lib.rs:91) already covers the reject path generically.

---

## 4. Verification the full chain works (once dotnet + corpus available)

```text
MedCare C# ──dotnet run harvester──▶ out.ndjson
  ──ruff_csharp_spo::load──▶ Vec<Triple>          (validates vocab)
  ──ruff_spo_triplet::reassemble──▶ ModelGraph    (reassemble.rs:89)
  ──ogar_from_ruff::lift_actions(&model)──▶ Vec<ActionDef>   (lib.rs:574; effect facts now populated)
  ──ogar_vocab::capability_registry::entries_from_actions──▶ Vec<(cap, classid)>   (SHIPPED, ec52376)
  ──healthcare_actions.rs + domain_tables()──▶ resolve_hotplug GREEN
```

The DO-arm rows only affect the *richness* of the ActionDefs (projection /
`kausal` / RBAC). The hot-plug table itself keys on `has_function` name +
subject classid, so it goes green even on name-only lifts — see the plan
doc §1 and `entries_from_actions_ignores_effect_facts` (the OGAR test).

---

## 5. Cross-references

- **Plan:** `OGAR/docs/integration/MEDCARE-CSHARP-PARITY-INTEGRATION-PLAN.md`
  (P3 = the healthcare table on the existing chain; P6 = the DO-arm walk —
  now known to be *drafted*, so P6 collapses to "verify + de-draft").
- **C++ template:** `ruff/crates/ruff_cpp_spo/src/clang_walker.rs:873`
  (`method_body_arm`, shipped #57) — the reference the C# `EmitBodyArm`
  mirrors.
- **Generic seam (shipped):** `OGAR/crates/ogar-vocab/src/capability_registry.rs`
  `entries_from_actions` (OGAR `ec52376`).
- **Fuzzy bodies method:** `ruff/.claude/knowledge/fuzzy-recipe-codebook.md`
  (the `(verb, criteria)` recipe codebook the DO-arm facts feed).
- **Hot-plug consumer recipe:** `OGAR/.claude/knowledge/hotplug-consumer-migration.md`.

---

## 6. The one blocker that must clear first

Reconnect the Claude GitHub App for the `AdaWorldAPI` org (org-admin
action) and confirm `ruff` is in its repo scope. Until then no ruff commit
(the §3 patch, the de-draft, any harvester change) can be pushed. OGAR /
lance-graph / medcare writes are unaffected (this handover pushed fine).
