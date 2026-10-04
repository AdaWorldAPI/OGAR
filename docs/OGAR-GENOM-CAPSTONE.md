# OGAR-GENOM — packed genomics, expression folds, evolutionary replay

> **Status:** architecture and feasibility capstone (2026-10-04), plus a first
> proof crate, `crates/ogar-genom`. Measured where it says "measured";
> everything else is labelled. No classid is minted here: the Genetics domain
> `0x0E` holds zero concepts by design (`ogar-vocab` lib.rs:1383-1389), and
> minting stays with the operator.
>
> **The question this answers:** does the existing substrate support
>
> ```
> genotype ─fold under environment→ phenotype/fitness ─select→ sparse write → next generation
> ```
>
> without copying genomes to look at them? **Short answer:** the *expression*
> half (`resident sequence → coordinates + masks → folds`) holds and is
> measured. Two of the shortcuts in the hypothesis do not hold:
>
> - The *heredity* half does not work as "per-individual delta streams on a
>   version line". Version lines are linear, and delta streams duplicate
>   shared ancestry.
> - The model that survives for ancestry is the **tree sequence / ancestral
>   recombination graph**: interval-labelled parent→child edges. It is prior
>   art, and the plan adopts it rather than reinventing it.

---

## 0. What bytes move — the rule this document is judged by

Each operation is classified as one of four kinds:

| class | meaning |
|---|---|
| **VIEW** | a borrow plus coordinates; zero bytes of sequence move |
| **FOLD** | streams over resident bytes and emits a small result (count, residue stream, mask) |
| **MATERIALIZE** | writes a derived object proportional to its length; must be named `materialize*`/`decode` |
| **WRITE** | durable state change (new reference, new variant row, new lineage edge) |

---

## 1. Inventory — what already exists (reuse, do not re-mint)

Each entry was read from source. "Zero hits" lines name the closed search space.

### 1.1 ndarray (`AdaWorldAPI/ndarray`, `src/simd.rs` facade)

- `U8x64::{permute_bytes, shuffle_bytes, cmpeq_mask, shl_epi16, saturating_add, movemask, popcnt}`.
  - `permute_bytes` is `vpermb` when VBMI is present. It is a full 64-entry
    byte LUT in one instruction, which is the exact shape of `Codon6 → amino acid`.
  - **Gotcha:** it is `#[inline]` and does a `simd_caps()` check on every call,
    so a hot loop pays one branch per 64 codons.
  - `shuffle_bytes` (`vpshufb`) is a 16-entry LUT per 128-bit lane, which is
    the exact shape of the V4 nibble algebra.
- `simd_masking_ops::{eq_u8_to_mask, mask_and/or/xor/andnot, mask_ternlog*, mask_set_range, masked_group_*}`.
- `popcount_batch_u64`.

### 1.2 lance-graph

- **`lance-graph-mask-risc`** (engine side). Masks are `u64` bit-planes over
  rows.
  - `ir::Program{ops: Vec<MaskOp>, terminal}` with `MaskOp::{Pred, And, Or,
    Xor, AndNot, Not, Ternlog, Gather}`.
  - `Pred::Range{lo,hi}` covers row-index ranges.
  - Terminals include `Count`, `Any`, `Keep` and the masked/grouped reductions.
  - "Fold" here means a reduction terminal that never materializes the mask
    (`FusedFold`, `Tern2Fold`, tiled `TILE_WORDS=256`).
  - **This is the population-fold engine** for genotype bitplanes (§10).
  - It has no notion of value intervals or coordinate transforms.
- **Contract `class_view.rs`.** `FieldMask(u64)` is presence-only, at most 64
  fields; `WideFieldMask` widens it. `trait ClassView` covers field selection
  and byte reading.
  - **It does not describe coordinate transforms.**
  - So `GenomeView` is *not* a ClassView: a ClassView selects fields of a
    row, and a genome view remaps positions in one sequence. These are
    different signatures, and merging them would distort both (§5).
- **Contract `alpha.rs::AlphaMask`.** A bitmap over base ordinals, with
  `materialize_ordinals` as its one named materializer. It is the shape for
  "which positions" masks.
- **Contract `temporal_pov.rs`.** `LanceVersion = u64`, `VersionRange`, and
  `TemporalPov` form a **linear** version line.
  - `scenario.rs::ScenarioBranch` has **one** parent (`forked_from: u64`).
  - There is no two-parent merge anywhere in the repo.
  - **Consequence: H7 is falsified** (§11).
- **Contract `canonical_node.rs`.**
  - `NodeRow` = key 16 | edges 16 | value 480. Its doc already says that bulk
    raw data — *"a ~3.2 Gbp genome"* — is **a separate Lance table addressed
    by key/classid, never inlined**.
  - `CLASSID_CPIC_V3 = 0x0E01_1000`. Its doc rules that the V3 basins are
    **genomic mereology** (genome → chromosome → region → locus → gene as
    HHTL coordinates), not labels.
  - **This is H6, already decided upstream.**
- **Counterfactuals.**
  - `counterfactual.rs` keeps counterfactuals in their own lane, never as
    observed SPO.
  - `planner/dismech_counterfactual.rs::counterfactual_replay` cuts one edge
    and replays.
  - The cognitive `world/counterfactual.rs` explicitly says **"NOT
    do-calculus"**.
  - There is no general SCM.
- **Provenance.**
  - `causal_audit.rs::SupportBasis` has `TextAttested`, `DirectlyObserved`,
    `InterventionBacked`, `SimulationOnly`, … with `EvidenceSourceId`,
    `SupportReceipt` and `SupportLedger`.
  - `dismech_evidence.rs::EvidenceSource` has `HumanClinical`,
    `ModelOrganism`, `InVitro`, `Computational`.
  - The INDRA harvest (`.claude/harvest/indra-reference-wiring.md`) records
    these as *shipped types, unwired*.
- **Prior genetics synthesis** (`.claude/handovers/2026-06-16-genetic-research-headstone-exploration.md`,
  `docs/GENETIC_RESEARCH_VIA_STACK.md`): synthesis only, no code.
  - It sets standing invariants: no `ValueSchema::Genetic`; counterfactuals in
    their own lane; htslib stays upstream; research tooling, not diagnostics.
  - Its proposed `adapter-genetics-experimental` crate does not exist.
  - This document keeps all of those invariants.

### 1.3 OGAR

- **`ogar-obo`.** `parse_obo`, `bake`, `NsRegistry`/`NsSpec` (namespaces as
  data), `Crosswalk`, `Predicate` edge byte, `spine::SpineLens`,
  `reason::saturate`.
  - Baked today: MONDO, HPO, UBERON, PATO, RO.
  - `META_STUDY_SPINE` (BFO, IAO, **OBI**, OBCS, SEPIO, **ECO**, COB, FBbi) is
    registered but bake-gated.
- **`ogar-ro`.** The `RELATIONS` palette has `IS_A`, `PART_OF`
  (`BFO:0000050`), `REGULATES` (`RO:0002211`), `HAS_PHENOTYPE`
  (`RO:0002200`), …
  - **Absent:** transcribed-from / gene-product / translated-from relations.
    These are candidates from RO, to be checked against an RO release when
    minted.
- **`ogar-vocab`.** `ConceptDomain::Genetics = 0x0E`, with **zero** concepts
  (a test asserts the count is 0). `0x96` BiologicalProcess and `0x97`
  Substance are reserved homes for GO and ChEBI.
- **`ogar-cpic`.** A gene → phenotype-axis → drug recommendation table (7
  rows). No alleles, no genotypes, by design.
- **`ogar-dismech`.** Contains a `VARIANT_OF` predicate (`0xA2`).
- **`ogar-dir-sim/provenance.rs`.** `Origin{Observed, Simulated{rule, evidence}}`.

### 1.4 MedCare-rs (private)

- CPIC wiring: `medcare-cohorts` `Pgx`, `AXIS_PGX`.
- `medcare-dismech` holds Genetic nodes with raw HGNC and GO CURIEs, never
  addresses.
- GO is unbaked (slot `0x96`); ChEBI is unbaked (slot `0x97`).
- SO, PRO, HGNC, ClinVar and dbSNP are docs-only or absent.
- The doc layer records the chain `SO → VARIO → HGNC → HP → MONDO` as
  architecture only.
- Commitments #9 (no sourcing or licensing talk in public repos) and #10
  (types exist only before the bake) **bind this design**. Nothing in this
  document discusses corpus terms.

### 1.5 Zero hits — closed search space

No type named `GenomeId`, `VariantId`, `GeneId`, `SequenceId`, `LineageId`,
`PopulationId`, `Strand`, `Allele*`, `Haplotype*`, `Genotype*` or
`GenomicInterval` exists in lance-graph, OGAR or MedCare-rs Rust (non-`target/`).
The same space has no FASTA, VCF, GFF, BAM, codon or nucleotide code.
(`indra` is the upstream Python INDRA; its `MutCondition.to_hgvs` is protein-level only.)

**Do not overload:**

- `NodeGuid` / `ClassId` (classes, not positions);
- `CausalEdge64` (causal claims, not ancestry);
- HHTL `NiblePath` (a tree; one parent per node);
- `LanceVersion` (storage time, not biological time).

---

## 2. Real genomics requirements that break the simple analogy

1. **Coordinates are not stable across individuals.** An insertion upstream
   shifts every downstream position, so "patch the bytes" is not a variant
   model. Two coordinate spaces are needed:
   - **reference** coordinates (stable, shared);
   - **haplotype** coordinates (per individual, derived);
   - plus a liftover map between them, which is piecewise-affine over indel
     breakpoints.
2. **Structural variants are not local.** Inversion, translocation and
   duplication rearrange intervals across contigs. In an SV-dense region the
   faithful representation is a sequence *graph* (GFA, variation graphs).
   Even then it is a graph of **segments**, never of bases.
3. **Recombination writes ancestry, not sequence.** A crossover produces no
   new bases. It changes *which parent each interval descends from*.
4. **Two parents.** Sexual reproduction is a DAG over intervals: an ancestral
   recombination graph (ARG). A linear version chain cannot hold it.
5. **Ploidy and phase.** Humans are diploid. A genotype at a site is two
   alleles, possibly unphased.
6. **Splicing is ordered and strand-specific.** Exons are concatenated in
   transcript order. On the minus strand that is *descending* genomic order,
   and CDS phase crosses exon junctions.
7. **Expression is not a function of sequence alone.** It depends on
   chromatin, methylation, cell type, environment and time. Sequence identity
   and regulatory state are separate axes. Nothing epigenetic goes in the
   nucleotide bits.
8. **Ambiguity has three readings.** Matching a motif against `N` is
   *possible*, not *definite*. Measured on chr21:
   - `TATAWAWR` definite hits: **30,430**;
   - "N matches anything" hits: **6,651,513**.

   An API that collapses the two produces garbage. `BaseSet` keeps them
   apart.

---

## 3. V4 vs DNA2 — measured, not chosen by elegance

**Data.** GRCh38 chr21 (UCSC soft-masked FASTA; the Ensembl copy agrees on
counts), 46,709,983 bases.

| measurement | value |
|---|---|
| `N` | 6,621,364 bases (14.2%) in **52 runs** (largest 5.01 Mbp) |
| non-`N` IUPAC symbols (`R`,`Y`,…) | **0** |
| soft-masked (lower-case) | 20,718,742 bases in **48,677 runs** |
| u8 | 46.7 MB |
| V4 everywhere | 23.4 MB (4 b/base) |
| **DNA2** | **11.68 MB (2.000 b/base)** |
| sidecars, 8 B/run (probe) | 0.39 MB (0.067 b/base) |
| sidecars, 16 B/run (crate: `u64` intervals) | ~0.78 MB (0.13 b/base) |
| `N`-only sidecar | **416 B** |
| encode u8→DNA2 | 14.1 ms (3.3 GB/s) |
| exact round trip (case + `N` restored) | OK |

**Break-even.** With 16-byte runs, an ambiguity sidecar beats V4-everywhere
while it holds fewer than ~1 run per 64 bases. References sit five orders of
magnitude below that. Consensus or ancient-DNA sequences with dense ambiguity
could approach it. That is the one case where V4 as resident storage could
win, and it is not measured here.

**Verdict.**

- DNA2 is resident.
- Ambiguity and soft-mask live in sparse interval sidecars.
- V4 is the **query and answer algebra**: query masks, possible-base sets,
  and Fitch state sets (§12).

V4 is not storage.

**Prior art, honestly.** This is UCSC **`.2bit`**'s design: 2 bits/base plus
N blocks plus mask blocks, with `T=0 C=1 A=2 G=3` there. The ACGT order chosen
here makes complement `3 − b`. Import and export should be byte-compatible
adapters, and this document claims no novelty for the encoding.

---

## 4. Codon ISA — measured

Translation is NCBI table 1, frame 0, chr21, single core. The machine has
AVX-512 VBMI. The probe uses the `ndarray::simd` facade only.

| path | time | per codon |
|---|---|---|
| naive u8 (3 table lookups + LUT, push to Vec) | 294 ms | 18.9 ns |
| DNA2 scalar (6-bit extract from a `u128` window + LUT) | 32.8 ms | 2.11 ns |
| DNA2 + `vpermb`: codon at **every** position (all 3 frames at once) | 30.1 ms | 0.64 ns per position = 1.93 ns per frame-codon |

What this shows:

- **The LUT is not the bottleneck; codon extraction is.** The 64-entry table
  is one `vpermb` per 64 codons. Building the 6-bit index (unpack DNA2, shift,
  combine three shifted windows) dominates.
- The SIMD path's real win is that **three frames cost the same as one**: it
  computes a codon at every offset and lets a frame be a stride.
- Six-frame translation = this fold plus the same fold over a reverse-complement view.
- Lane and word boundaries are handled by reading two bytes past each
  64-base block. Correctness is pinned against a naive `u8` translator across
  lengths 5..4099, all frames, both strands
  (`all_frames_both_strands_across_word_boundaries_match_naive`).
- **Ambiguous codons.** A codon translates only if **every expansion agrees**
  (`GCN→A`, `GAR→E`, `TRA→*`); otherwise it yields `X`.
- **H5: confirmed with the nuance above.** The table is biology's NCBI
  table 1 and stays separate from every R2IL / 0..63 cognitive vocabulary.

---

## 5. The central law — what bytes move, per operation

Reverse complement on chr21:

| path | time | bytes written |
|---|---|---|
| u8 materialized | 23–26 ms | 46.7 MB |
| DNA2 materialized (word bit-reverse + NOT + funnel shift) | 2.8 ms | 11.7 MB |
| **DNA2 word-streaming view** (reversed words produced on the fly into a fold) | **4.0 ms** | **0** |
| DNA2 per-base view (`len−1−i` per base) | 42.8 ms | 0 |
| strand-invariant fold (GC via `popcount((w>>1)^w)`) | 0.46 ms | 0 |

What this shows:

- **A view is only competitive when it is word-granular.** A per-base
  coordinate transform is ~10× slower than materializing.
- The first-PR crate ships the per-base path as the *correctness reference*
  (`SeqView::base_set`). The word-streaming kernel is follow-up #1.
- Some folds are strand-invariant and need no transform at all.

| operation | class | notes |
|---|---|---|
| chromosome → region | **VIEW** | `SeqView::sub` |
| reverse complement | **VIEW** | strand flip; fold via word stream |
| reading frame | **VIEW** | an offset |
| region → motif hits | **FOLD** → mask | `AlphaMask`/bitplane output; never a hit list unless materialized |
| CDS → codons → amino acids | **FOLD** | `translate()` is an iterator, allocation-free (pinned by `no_alloc.rs`) |
| gene → exon subset | **VIEW** (scatter list) | ordered interval recipe |
| exons → transcript | **VIEW**, *materialize on demand* | concatenation is a coordinate recipe (§7) |
| transcript → protein sequence | **MATERIALIZE** when stored; **FOLD** when consumed | proteins outlive their transcript in most consumers (structure, alignment) |
| ingest reference | **WRITE** once | DNA2 + sidecars, out-of-line Lance table |
| SNV/indel per haplotype | **WRITE** (sparse) | §6 |
| recombination | **WRITE** of ancestry edges | not sequence (§10–11) |
| phenotype / fitness | **FOLD** | model output; written only if kept as evidence (§14) |
| selection | **FOLD** → mask | AND + popcount, §10 |

**H2 is partly supported.** Region, strand, frame and translation are views
and folds, measured. Splicing is a view in principle (§7, not built).
Indel-bearing haplotypes need a liftover map, so they are *not* a pure view
of the reference. Downstream consumers of proteins usually need the
materialized sequence.

---

## 6. Variants are sparse writes — and coordinates are the hard part

- **Density.** A human genome differs from the reference at ~4–5 M sites
  (published population-scale figures; not measured here). That is ~0.15% of
  3.1 Gbp, so per-haplotype variants are sparse. **H3: partly supported.**
  Large SVs and CNVs are not sparse in bytes.
- **Overlay read cost** (measured on chr21; 1 SNV/kbp; 10k random 1-kb windows):
  - with the overlay merged on the fly by `partition_point` + walk: 13.0 ms;
  - without: 9.3 ms;
  - **+40%**, without rewriting the reference.
- **Coordinate model** (proposed):
  - Internal coordinates are 0-based half-open on the **reference**
    (`coord::Interval`). VCF's 1-based positions are converted by the
    adapter, never inside.
  - Variant identity is the **normalized** allele: left-aligned, parsimonious
    `(contig, start, end, alt)`. External identifiers are computed by GA4GH
    VRS where interop needs one.
  - A haplotype view is `reference + sorted overlay`. Its coordinate space is
    derived through a piecewise-affine **liftover** built from the indel
    breakpoints. This is a small sorted array, O(#indels).
  - Liftover is a *view* when consumed and a *materialization* when the
    haplotype is stored.
  - **SVs** are interval rearrangements over segments:
    - inversion = strand flip of a segment view;
    - translocation = re-ordered segment recipe;
    - duplication = repeated segment reference.

    Past a density threshold the honest form is a segment graph (GFA),
    adapted at the boundary and not native here.
- **Compaction.** When an overlay outgrows its reference (diverged lineages,
  other species), it is promoted to a new reference: a new WRITE with new
  identity. Overlays never chain without bound (see H9).

---

## 7. Transcripts and alternative splicing

A transcript is `(reference, strand, ordered exon intervals)`. The exon list
is the **coordinate recipe**, typically 2–100 intervals of 16 B each.

```
genomic:    A   B   C   D   E          (resident, unchanged)
isoform 1:  A   B   C       E          recipe = [A,B,C,E]
isoform 2:  A       C   D   E          recipe = [A,C,D,E]
```

- **Extra memory per isoform** = its recipe. Per-isoform memory is constant
  in transcript length: ~10 exons × 16 B = 160 B against kilobases of RNA.
- On the minus strand, the recipe lists exons in descending genomic order and
  each segment is read as a reverse-complement view.
- Translation over the recipe needs a **cross-junction codon window**: the
  codon fold must carry up to two bases across a segment boundary. This is
  the one kernel the splice view adds, and it is the *same* carry problem the
  SIMD path already solves at 64-base block edges.
- **When materialization is needed:** feeding external tools (aligners,
  folding models), exporting FASTA, and repeated random access into one
  isoform where the recipe walk dominates.
- This is the place to prove a real advantage, and it is **PR #2**.

---

## 8. Regulation and epigenetic state

These are separate axes, each its own sidecar or table keyed by reference
coordinates:

- accessibility (interval masks);
- methylation (per-CpG values, sparse);
- expression (per gene × sample × condition matrix — a dense numeric table,
  not a sequence property).

```
expression_fold(gene, sample, environment) =
    f( sequence view , regulatory masks(sample) , environment )
```

Nothing in this document claims heritable epigenetic transmission. If a
model needs it, it is a separately versioned regulatory-state table with its
own provenance, never bits in the nucleotide words.

---

## 9. Ontology layer — addresses over regions, never bases

**H6 holds, and it was already decided upstream:**

- `canonical_node.rs` says the genome is out-of-line and that basins are
  genomic mereology.
- OGAR addresses `Chromosome`, `GenomicRegion`, `Gene`, `Exon`,
  `Transcript`, `Protein`, `Variant`, `RegulatoryRegion` and `Phenotype` as
  *classes*.
- Instances carry coordinates (`contig, Interval, strand`) pointing into the
  resident sequence. One graph node per base is never needed.

**Use standards, do not mint lookalikes:**

| concept | source | today in the stack |
|---|---|---|
| feature types (gene, exon, transcript, SNV, insertion, …) | Sequence Ontology (SO) | docs only |
| gene identity | HGNC | raw CURIEs only |
| proteins | PRO / UniProt | docs only |
| functions and processes | GO | slot `0x96` reserved, unbaked |
| disease | MONDO | **baked** |
| phenotype | HPO | **baked** |
| relations | RO / BFO (`ogar-ro`) | **baked**; missing transcribed-from, gene-product, translation relations |
| evidence and assay | ECO, OBI | registered, bake-gated |

- The integration path is the existing one: `NsSpec` rows in `ogar-obo`, the
  `Crosswalk` for xrefs, and RO predicates in `ogar-ro`.
- **Every new concept or namespace allocation is an operator mint**
  (`0x0E` has zero concepts by ruling).
- Per MedCare-rs commitment #10, relation kinds resolve before the bake.
  Nothing here proposes a post-bake type that describes a relation.

---

## 10. Population model — measured

**Layout.** Genotypes are site-major **bitplanes**: per site, two planes over
individuals (`carries_alt`, `hom_alt`), and dosage = popcount(p0) + popcount(p1).

| fold | 100k individuals × 2,000 sites | 1M individuals × 200 sites |
|---|---|---|
| allele-frequency (popcount) | 2.48 ms — **1.2 µs/site** | 1.98 ms — **9.9 µs/site** |
| AF among selected parents (mask AND + popcount) | 1.81 ms | 2.10 ms |

- Both folds run at **20–28 GB/s: memory-bound**. That is ~0.01 ns per
  individual-site.
- **H8 confirmed:** selection is a mask; AF-among-selected is AND + popcount.
  In production both are `lance-graph-mask-risc` Programs (`And` then `Count`).
- Recombination and mutation are the only writes.

**Storage per individual, human-scale estimates** (not measured unless marked):

| representation | per diploid individual | 1,000 | 1,000,000 |
|---|---|---|---|
| full copies, u8 | 6.2 GB | 6.2 TB | 6.2 PB |
| full copies, DNA2 (2 haplotypes) | 1.55 GB | 1.55 TB | 1.55 PB |
| reference + sparse variants (~4.5 M/haplotype × 8 B) | ~72 MB | ~72 GB | ~72 TB |
| site-major bitplanes (dense, 2 bits per individual-site) | — | 0.25 KB/site | 250 KB/site |
| shared ancestry (tree sequence / ARG) | sublinear | literature: whole-chromosome histories for millions of samples in GB-scale files | — |

- Bitplanes are dense for common variants. Rare variants want carrier lists:
  a sorted row-id list, i.e. a sparse mask. The hybrid is what PGEN/BGEN-class
  formats do.
- Per-individual sparse deltas are **~21× smaller than copies but still
  linear** in population × variants. They repeat every allele each carrier
  shares by descent.
- **Shared-ancestry representations scale with the number of distinct
  ancestral events, not carriers.** That is the decisive scaling fact for long
  runs (H9).

---

## 11. Evolutionary time ≠ storage version

Four kinds of time stay distinct:

1. **storage version** — `LanceVersion`; linear; when bytes were written;
2. **generation** — integer, biological;
3. **ancestry** — a DAG over intervals;
4. **sample time** — when an organism was observed or collected.

**H7 is falsified.** The temporal machinery is linear and `ScenarioBranch`
has one parent. Overloading either for sexual reproduction would be the
`CausalEdge64`-for-everything mistake.

**What survives — the tree-sequence / ARG table model** (msprime, tskit,
SLiM). Proposed as the lineage layer, adopted, not reinvented:

```
nodes      (id, time/generation, flags, individual)
edges      (left, right, parent, child)     ← interval-labelled; 2 parents fall out naturally
sites      (position, ancestral_state)
mutations  (site, node, derived_state, parent_mutation)
```

- A crossover is two edges with complementary intervals and different
  parents. A mutation is a row on the node where it arose.
- A genome is **never stored**. It is a *replay*: walk the edges covering an
  interval up to the root and apply the mutations on that path. This is
  exactly "evolution = fold → select → sparse write → repeat", with the
  writes being edges and mutations.
- Simplification (dropping extinct lineages) is the compaction step that
  keeps it sublinear.
- Edges are interval-keyed, so they reuse `coord::Interval`. The tables are
  ordinary columnar rows and fit Lance natively.

---

## 12. Missing links — constraint-based candidate reconstruction

This is framed as **candidate reconstruction**, never history.

There is a genuine mechanism match, graded **[H]**: **Fitch parsimony's
state sets are exactly V4 `BaseSet`s.**

- Per site, bottom-up: a node's set = intersection of its children's sets if
  non-empty, else the union.
- Top-down resolution then picks members.
- Over many sites at once this is lane-parallel `AND` / `OR` / empty-test on
  nibbles: the same `vpand` / `vpor` / `cmpeq_mask` shapes measured in §3–4.

Given genomes A and C plus a topology, candidate intermediates B are the
per-site resolutions consistent with the constraints.

Kept separate:

- maximum-likelihood or Bayesian ancestral reconstruction (substitution
  models, branch lengths) — statistical, not mask algebra;
- indel-aware reconstruction, which needs alignment first.

Output is a candidate set with its constraint receipts. It is written, if at
all, as `SupportBasis::SimulationOnly`.

---

## 13. Counterfactual genomics

A model intervention, "same ancestral state without V", is cheap here:

- drop one mutation row in the tree sequence, or one overlay entry;
- replay the phenotype or fitness fold.

This matches the shape of `dismech_counterfactual::counterfactual_replay`:
cut one edge, replay, return a thresholded verdict.

It is **not** biological causality:

| | model intervention | experimental intervention |
|---|---|---|
| what is changed | a row in our model | the organism (CRISPR, knockout) |
| provenance | `SimulationOnly` | `InterventionBacked` |
| licenses a causal claim? | no | yes, within its design |

Counterfactual results stay in their own lane (the standing
`counterfactual.rs` rule). The repo's own counterfactual module disclaims
do-calculus; this design does not re-claim it.

---

## 14. Evidence and provenance

- A **variant** is not evidence.
- An **observation of a variant in a sample by an assay** is.

| claim | basis (`SupportBasis`) | source class (`EvidenceSource`) |
|---|---|---|
| paper reports gene X associated with phenotype Y | `TextAttested` | — |
| cohort observes variant V with phenotype Y | `DirectlyObserved` | `HumanClinical` |
| simulation predicts V changes fitness | `SimulationOnly` | `Computational` |
| CRISPR perturbation of V changes Y | `InterventionBacked` | `InVitro` / `ModelOrganism` |

**H10: partly supported.** The four witness kinds already exist as distinct
variants. The missing record is the **observation row**: `(sample, assay,
variant, call quality, …)` joined to a receipt. The INDRA harvest names the
same gap: evidence types shipped, unwired. Assays map to OBI and evidence
codes to ECO, both registered and bake-gated.

---

## 15. Interop boundary

| format | role | where |
|---|---|---|
| FASTA / **2bit** | reference identity and content | ingest/export adapter → DNA2 + sidecars (2bit is near byte-compatible) |
| refget / seqcol digests | external sequence identity | carried as data on the reference row (an external standard id — not an internal pin) |
| FASTQ | reads + qualities | boundary only; not resident |
| VCF / BCF | variants + genotypes | adapter → normalized variant table + genotype bitplanes / carrier lists |
| GFF3 / GTF | annotation | adapter → interval tables + transcript recipes |
| BED | intervals | 1:1 with `coord::Interval` (both 0-based half-open) |
| SAM / BAM / CRAM | alignments | out of scope. CRAM is the same law (resident reference + per-read deltas) — prior art, not a target |
| tskit `.trees` | ancestry | lineage-layer import/export |
| GFA | SV-dense graphs | boundary adapter only |

Parsers come from existing crates (the 2026-06-16 handover named `noodles`).
htslib stays upstream. The internal canonical forms are:

- DNA2 + sidecars;
- interval tables;
- variant rows;
- bitplanes;
- tree-sequence tables.

---

## 16. Performance — where the time goes

Per human haploid reference (3.1 Gbp):

| form | size |
|---|---|
| u8 | 3.1 GB |
| V4 | 1.55 GB |
| DNA2 | 775 MB |
| DNA2 + sidecars | ~780–830 MB |

Soft-mask runs scale from chr21 to ~3.2 M genome-wide, so 26–52 MB at 8–16 B/run.

Throughput, single core, chr21-measured rates extrapolated to 3.1 Gbp:

| fold | rate | bound by |
|---|---|---|
| encode | 3.3 GB/s → ~1 s/genome | branchy per-byte classification |
| strand-invariant composition (popcount) | ~25 GB/s of packed bytes → ~30 ms/genome | **memory** |
| reverse-complement word view | ~3 GB/s of packed bytes | **compute** (bit reversal) |
| translation, DNA2, 3 frames at once | ~1.9 ns/frame-codon → ~6 s per genome-frame | **compute** (codon extraction) |
| exact 12-mer, u8 `windows ==` | 1.4–1.8 GB/s | memcmp, vectorized |
| exact 12-mer, DNA2 rolling scalar | 0.6–0.7 GB/s | **branch/serial dependency** |
| V4 IUPAC motif over DNA2 (`vpshufb` onehot + AND-table + `cmpeq`) | 1.25–1.4 GB/s | compute |
| AF fold over bitplanes | 20–28 GB/s | **memory** |
| overlay read | +40% vs plain at 1 SNV/kbp | binary-search + branch |

**Honest negatives:**

- The scalar rolling-hash k-mer over DNA2 is ~2.5× *slower* than u8 window
  compare. Packed k-mer search needs a SIMD shift-and-compare kernel to pay
  off. Not built.
- The per-base view path is 10× slower than word-granular.

Probe source: `crates/ogar-genom/lab/genom_probe.rs`. It is lab code, not
built by the workspace, and its SIMD comes from `ndarray::simd` only.

---

## 17. Falsification table

| # | hypothesis | verdict | why |
|---|---|---|---|
| H1 | DNA sequence is a good resident immutable population | **CONFIRMED** | exact round trip; 2 b/base; one named materializer; matches 2bit / CRAM practice |
| H2 | gene/transcript/protein interpretation is mostly views + folds | **PARTLY SUPPORTED** | region / strand / frame / translation measured as views/folds; splicing is a view in principle (PR #2); indel haplotypes need liftover; proteins often must materialize |
| H3 | durable genomic change is sparse | **PARTLY SUPPORTED** | SNVs/indels ~0.15%/haplotype; recombination writes ancestry, not sequence; SVs/CNVs not sparse in bytes |
| H4 | DNA2 + sparse ambiguity beats V4 resident | **CONFIRMED** (reference) / **NEEDS BENCHMARK** (dense-ambiguity sequences) | chr21: 52 N runs, 0 other IUPAC → 0.067–0.13 b/base vs +2 b/base; V4 kept as query algebra, exact agreement with scalar on `TATAWAWR` |
| H5 | codon translation benefits from a 6-bit LUT | **CONFIRMED, nuanced** | LUT = one `vpermb`/64 codons; extraction dominates; DNA2 9× faster than naive u8; SIMD gives 3 frames for the price of 1 |
| H6 | OGAR addresses regions, not bases | **CONFIRMED** | already ruled upstream (out-of-line genome; mereology basins); standards (SO, HGNC, GO, RO) supply semantics; mints operator-gated |
| H7 | generations fit the existing temporal infrastructure | **FALSIFIED** | versions linear; `ScenarioBranch` single-parent; no two-parent merge — needs an ARG/tree-sequence lineage layer |
| H8 | selection is mainly masks/folds | **CONFIRMED** (bench) | AF and AF-among-selected = AND + popcount at memory bandwidth (9.9 µs/site at 1M individuals) |
| H9 | lineage delta storage stays efficient under long runs | **FALSIFIED** for per-individual ordered delta streams / **NEEDS BENCHMARK** for tree sequences | deltas grow with carriers × generations and duplicate shared descent; tree sequences scale with distinct events (literature-reported, not measured here) |
| H10 | provenance distinguishes observed / literature / experimental / simulated | **PARTLY SUPPORTED** | `SupportBasis` + `EvidenceSource` carry the distinction; the per-sample observation record is missing and evidence types are unwired |

---

## 18. Minimal architecture

**Not one mega crate.** Each layer lands where its kind already lives.

| layer | home | depends on | status |
|---|---|---|---|
| alphabet, sequence, coord, view, translate | `OGAR/crates/ogar-genom` | nothing | **PR #1 (this)** |
| transcript (splice recipe view + cross-junction codon fold) | `ogar-genom::transcript` | — | PR #2 |
| variant (normalized rows, overlay view, liftover) | `ogar-genom::variant` | — | PR #3 |
| word-stream and SIMD kernels (codon@every-pos `vpermb`, V4 `vpshufb`, revcomp word view) | `ogar-genom` behind an `ndarray` dep (`ndarray::simd` facade only) | ndarray | after PR #1–3; cargo-substrate review for the dep |
| resident storage (words + sidecars as an out-of-line table) | lance-graph side, per the `NodeRow` doctrine | Lance | later |
| population folds (bitplanes, selection) | `lance-graph-mask-risc` Programs | — | exists; needs an adapter |
| lineage (tree-sequence tables, replay, simplify) | new crate, after a probe | `ogar-genom::coord` | later |
| phenotype | trait only — a fold over (sequence view, regulatory masks, environment) | — | interface |
| ontology (SO / HGNC / GO namespaces, RO relations) | `ogar-obo` `NsSpec` rows + `ogar-ro` | — | **operator mint gate** |
| evidence (observation rows → `SupportReceipt`) | lance-graph contract | — | later |
| file adapters (2bit / FASTA / VCF / GFF / BED / tskit) | boundary crates; parsers upstream | noodles etc. | later |

---

## 19. PR #1 — what landed

`crates/ogar-genom` (zero deps, no `unsafe`, no classid mints):

- `alphabet` — `Base` (DNA2), `BaseSet` (V4 nibble: union, intersection,
  subset, complement, IUPAC both ways).
- `sequence` — `PackedSeq`, `encode`/`decode` with **exact** round trip
  (case, `N` and all IUPAC runs), `AmbiguityRun`, soft-mask runs,
  `base_set(pos)` overlay-aware. `U` is refused rather than silently read as `T`.
- `coord` — `Interval` (0-based half-open), `Strand`, `Frame`.
- `view` — `SeqView`: `sub` and `reverse_complement` move zero bytes;
  `materialize_iupac` is the named materializer.
- `translate` — `CODON_TABLE` (NCBI 1, built from the TCAG table and checked
  against an independent hand-written ACGT table), `translate()` as an
  allocation-free `ExactSizeIterator`, ambiguity resolved only when
  determinate.

**Tests** (11). Each compares against an independent derivation:

- naive `u8` reverse complement and translation;
- a hand-written codon table;
- the insulin `MALWMRLLPL` fixture;
- a counting global allocator proving views and folds allocate **0** times
  (with an anti-vacuity half proving the counter fires on a materialization).

**Disable-verified red-then-green, 6 of 6:**

| disable | goes red in |
|---|---|
| identity complement | `v4_algebra_over_all_iupac_letters` |
| wrong table order | 4 tests |
| overlay ignored | `reverse_complement_is_a_view`, ambiguity test |
| reverse-sub arithmetic | `sub_views_compose_on_both_strands` |
| a collecting `base_sets` | `views_and_folds_allocate_nothing` |
| ambiguity-always-resolves | ambiguity test |

## 20. Deliberately NOT implemented

- Evolutionary simulator.
- Population or lineage storage.
- Phenotype models.
- Any file parser.
- Any classid, namespace or relation mint.
- SIMD kernels in the shipped crate.
- Lance persistence.
- Graph nodes per base.
- Epigenetic state in sequence bits.
- Any claim of causal truth from a model intervention.
- Any discussion of reference-data sourcing or terms.

## 21. Architecture

```
                         ┌──────────────────── OGAR ontology (addresses, not bases) ─────────────────┐
                         │  SO / HGNC / GO / MONDO / HPO classes  ·  RO relations  ·  operator mints  │
                         └───────────────┬──────────────────────────────────────────┬───────────────┘
                                         │ classid + (contig, Interval, strand)      │ evidence receipts
                                         ▼                                          ▼
 FASTA/2bit ─adapter─► WRITE ┌──────────────────────────┐                ┌───────────────────────────┐
                             │ resident reference        │   VIEW (0 B)   │ SupportBasis / Evidence    │
                             │ DNA2 words + N / softmask │──sub, strand──►│ Observed · Text · Sim · Exp│
                             │ sidecars  (out-of-line)   │                └───────────────────────────┘
                             └──────┬──────────┬─────────┘
          GFF ─adapter─► recipes ───┤          │◄── VCF ─adapter─► WRITE sparse variant rows + liftover
                                    ▼          ▼
                      VIEW transcript      VIEW haplotype = reference ⊕ overlay   (MATERIALIZE by name only)
                      (ordered exons)              │
                                    │              │
                                    ▼              ▼
                           FOLD codon → aa    FOLD expression / phenotype (sequence view × regulatory masks × environment)
                                                   │
                                                   ▼
                                     FOLD fitness ─► MASK selection (AND + popcount over genotype bitplanes)
                                                   │
                                                   ▼
                       WRITE  tree-sequence rows: edges(left,right,parent,child) + mutations
                       (lineage = DAG over intervals; replay = walk edges + apply mutations; simplify = compaction)
                                                   │
                                                   └──────────────► next generation (repeat)
```
