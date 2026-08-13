# ogar-doc — the INGESTION SPINE — transferred production experience

> **Status:** `[S]` — **transferred claim, NOT measured in this stack.** Every
> invariant below is evidence from another project's production history, cited
> to its source. None of it has been falsified *here*. Read it as borrowed
> operating time, not as a finding. Where a cheap falsifier exists it is named;
> until one runs, each `S-n` stays `[S]`.
>
> **Why this document exists.** `OGAR-DOC-W4-BUILD-SPEC.md` is strong on the
> half it covers — persist / read / reconstruct, `content_sha256` idempotency,
> the SoA subtree, the four-leg renderer. It **starts from a `DocIr` that
> already exists.** Nothing in OGAR covers the stage before that: a file
> arrives, and something must decide what it is, whether it even needs
> recognition, and in what order the work happens. That stage is where a
> document system accumulates its scar tissue, and we have none — the stack is
> months old on this axis.
>
> `paperless-ngx` has run that stage in production for roughly a decade. This
> document extracts its **spine** — the ordering constraints, the
> centralisation rules, and the failure modes it fixed — so the W4 council can
> check the build against experience we have not had to pay for.
>
> **Scope discipline:** this transfers *pipeline and dispatch shape only*.
> §NT names what must NOT be transferred, and why.

---

## A. Ingestion — the stage W4 does not have

### S-1. Ingestion carries no decisions [S]

A decade in, the primary ingestion path is still: **put a file in a directory.**
No form, no type picker, no upload dialog, no metadata prompt
(`documents/management/commands/document_consumer.py`; the `consume_file` task
at `documents/tasks.py:181-278`).

The transferable claim is not "build a watched folder" — it is that **every
decision moved into ingestion is a decision a human must make before the system
has done any work for them**, and that is the point where adoption dies. Type,
correspondent, account, patient: all of it is inferred after the fact and
corrected later (§S-9), never asked up front.

**Lands:** the `ogar-doc` ingestion ActionDef surface, which W4 does not yet
define. Its mandatory parameter set should be *bytes + provenance*, nothing
that requires a human to classify anything.

### S-2. Dedup is the first real gate, and it runs BEFORE the expensive work [S]

`ConsumerPreflightPlugin.run` (`consumer.py:1038-1050`) does file-exists,
duplicate detection, and directory creation **before any parsing**. The
duplicate check (`consumer.py:976-1027`) matches an incoming SHA-256 against
**both** `checksum` (original) **and** `archive_checksum` (derived artifact) —
because a re-ingested *export* of a document you already hold is the same
document, and only the derived hash catches it.

**This is the sharpest single delta against W4 as specced.** W4 makes
`persist_document` idempotent on `content_sha256` (`W4-4`) — correct, but that
is *after* recognition has run. For OCR the cost asymmetry is severe: a full
recognition pass is seconds-to-minutes of compute that a hash lookup would have
avoided entirely. Idempotency at persist prevents a duplicate *subtree*; it
does not prevent a duplicate *spend*.

**Lands:** a preflight gate keyed on `content_sha256` **ahead of** the OCR
ActionDefs, not only inside `persist_document`. The convergence key already
exists (`DocIr.content_sha256`); the lesson is *where it is consulted*.

**Falsifier (cheap):** ingest the same PGM twice through the current
`recognize_document` path and measure elapsed time on the second pass. If it is
not ~0, the gate is missing.

### S-3. A stage may halt the chain without erroring [S]

`StopConsumeTaskError` (`plugins/base.py:10-20`) exists so a stage can *cleanly*
terminate the pipeline — the barcode-splitter spawns child tasks for the split
pages and the parent must stop, which is success, not failure. The orchestrator
loop (`tasks.py:224-274`) always runs `cleanup()` in a `finally`, and threads
mutated metadata forward between stages via `overrides = plugin.metadata`.

OGAR's `ActionDef` already carries `guard_failure_policy` and `on_enter` Rubicon
hooks (`ogar-vocab/src/lib.rs:395-457`) — arguably a *richer* `able_to_run`. The
genuinely absent concept is the **clean halt**: a terminal-but-successful
outcome distinct from both completion and failure.

**Lands:** a third outcome in the doc-pipeline policy vocabulary. Without it,
page-splitting, multi-document scans, and "this is a cover sheet" all have to be
encoded as errors.

### S-4. One decision, two consumers, ONE function [S] — corroborated here

`is_born_digital_text` (`paperless/parsers/utils.py:133-166`) gates **both**
archive-generation and OCR-skip, from one place. Its own docstring
(`utils.py:144-147`) says why: the two used to be computed independently and
disagreed — their issue #13387.

**This one is not purely transferred.** `tesseract-rs` hit the identical failure
independently: `region_is_table` had to become one shared primitive feeding both
the recognition-side and classification-side block lists, because two
independent computations of one decision drifted apart silently. Two projects,
same shape, same fix.

**Lands:** a stated invariant for the doc layer. Any decision consumed by two
paths in `ogar-doc` (does this need OCR? should an archive artifact exist? is
this a duplicate?) is ONE function with ONE call site per consumer, never two
derivations.

**Status note:** this is the one entry here that is better than `[S]` — it is
independently corroborated in-stack. Grade `[H]` on the *rule*, `[S]` on the
specific pairing above.

### S-5. Ordering: the addressable record and its bytes must not be able to diverge [S]

The document row is created inside `transaction.atomic()`, `document_consumption_finished`
fires, and **only then** are files written to disk under `FileLock(settings.MEDIA_LOCK)`
(`consumer.py:666-720`). A failure during file placement rolls the row back, so
there is never an addressable document whose bytes are missing.

W4's shape inverts the risk: the subtree root's raw-ref carries `{content_sha256,
kv_key, mime, …}` (`W4-2`) — an address into a blob store the consumer owns. If
the subtree is written and the KV put fails, the graph holds a resolvable
`document_guid` pointing at nothing.

**Lands:** W4 should state which of the two may be orphaned safely. The
defensible answer is the same as paperless's — **bytes first, address second** —
so a failed put leaves an unreferenced blob (garbage, collectable) rather than a
dangling reference (corruption, undetectable until read).

**This is a question W4 currently does not answer, not a defect in it.**

---

## B. Escalation

### S-6. A mode ladder needs an incompatibility table [S]

The OCR ladder is `AUTO / FORCE / REDO / SKIP` with a retry-once on
`NoTextFoundException` → `safe_fallback=True` → forced OCR
(`tesseract.py:620-647`). We already do escalation better — ours is internal and
fine-grained (`binarize_page_escalating`; the signal ladder runs word → cell →
field → page → document), where theirs must re-run an opaque subprocess over the
whole document.

The part worth taking is not the ladder. It is the **accumulated knowledge of
which options conflict**, encoded directly in the argument builder
(`tesseract.py:265-382`): `deskew` is skipped under `REDO` (incompatible);
`clean_final` is skipped under `REDO`; `pages` is mutually exclusive with
`sidecar`. Every one of those is a bug someone shipped.

**Lands:** whatever declares escalation policy (`guard_failure_policy` is the
natural home) must be able to express *mutual exclusion between rungs*, not only
ordering. Our own stack already has one such pair discovered the same way —
deskew must run before rectify, and the rectify pass must then measure inert
(`deskew_then_rectify_measures_near_zero_shear_on_a_purely_rotated_page`).

---

## C. Metadata and rules

### S-7. A small fixed set of orthogonal axes beats free-form metadata [S]

Four: correspondent, document type, tags, storage path. Not forty. Custom
typed metadata exists (`CustomFieldInstance`, `models.py:1176+`) but sits
**beside** the four, never replacing them. Users can hold four axes in working
memory; they cannot hold an open schema.

**Lands:** resist the temptation to let `typed_field 0x080A` become an open
key-value bag. A small closed axis set plus an open typed-field space is the
proven shape.

### S-8. The rule attaches to the thing being recognised [S]

A `Correspondent` carries its **own** `matching_algorithm` + `match` string
(`documents/models.py:47-53`, matching in `matching.py:165-259`). Users never
meet a rule engine; they meet "this supplier, matched this way." The vocabulary
— `MATCH_ANY / ALL / LITERAL / REGEX / FUZZY / AUTO` — is a low-code rule
language with a CRUD form on it.

That vocabulary maps almost 1:1 onto `ogar-loco`'s existing computational core:
`CONTAINS` `0x69`, `LIST_CONTAINS` `0x7F`, `EQ` `0x30`, `NEQ` `0x31`
(`ogar-loco/src/lib.rs:431-573`), with `Vocabulary` (`vocabulary.rs:458`) as the
domain seam. `REGEX` and `FUZZY` (theirs: `rapidfuzz.partial_ratio`, cutoff 90)
would need one `FnIndex` each.

**Lands:** Kontenerkennung as an `ogar-loco` domain `Vocabulary`, where the
rule-bearing object is the **Konto** — exactly as the rule-bearing object there
is the Correspondent. The chart of accounts already exists in-workspace
(`woa-rs/crates/skr_data/{skr03,skr04,konto}.rs`), as do booking validation
(`buchungs_validator`) and export (`datev_encoder`).

**Falsifier (worth running before committing to the mapping):** take a real set
of SKR03 booking rules and express them with `CONTAINS`/`LIST_CONTAINS`/`EQ`
only. If `REGEX`/`FUZZY` turn out load-bearing rather than convenience, the
`FnIndex` additions are not optional and the council should know that up front.

### S-9. Suggest, don't auto-apply — and bound the input [S]

Predictions feed *suggestions*. `suggestion_content` (`models.py:406-432`)
truncates to 800k head + 200k tail over 1.2M chars specifically to bound
feature-extraction cost.

We can do better than suggest-vs-apply as a binary, because our extraction
carries verification that theirs cannot: `HarvestedField.checks` records which
checks passed (`arithmetic_ok`, `iban_mod97_ok`; empty = harvested but
unverified), and confidence aggregates as a **MIN** over joined words — chosen
deliberately because *"a lab/invoice VALUE cell is a single fact (`1O.5` vs
`10.5`), and one misread character invalidates the whole reading"*
(`tesseract-ocr/src/structured.rs:388`).

**Lands:** auto-apply is gated on verification, not on match strength. Posting
to a ledger on a fuzzy string match at 90 — which is what paperless does for
*filing* — is not defensible for *booking*. `checks` is what makes the same
ergonomics safe in a business-critical path.

### S-10. The learning tier must be skippable, versioned, and explainable [S]

Their classifier carries `FORMAT_VERSION = 10` on a signed artifact
(`classifier.py:143-219`) and **skips retraining** when a hash of all
`MATCH_AUTO` label assignments plus the latest-modified timestamp is unchanged
(`classifier.py:281-303`).

The skip-condition and the versioned artifact transfer directly. The estimator
does not (§NT-1) — and our replacement is strictly better on the axis that
matters most here. `lance-graph-arm-discovery` is a float-free Aerial+ transcode
emitting `CandidateRule` → `{s,p,o,f,c}` with integer support/confidence and
`TruthU8`. An association rule **is its own explanation**, so a discovered rule
can be shown, edited, and accepted — **materialising into the same `ogar-loco`
representation as a hand-authored one.**

Paperless structurally cannot do this: `MATCH_AUTO` and `MATCH_LITERAL` are
different mechanisms, and an `MLPClassifier` guess cannot be promoted into a
rule. That the learned tier and the authored tier converge on one
representation is our advantage, and it is worth protecting in the design.

---

## D. Business identity and lifecycle

### S-11. Dates: filename first, then content, and reject the impossible [S]

`RegexDateParserPlugin` tries the filename before the content when a
`filename_date_order` is configured, then filters: reject `year <= 1900`, reject
any future date, honour a configured `ignore_dates` set
(`plugins/date_parsing/base.py:98-114`).

For invoices the date **is** a business key (payment terms, posting period), and
for lab values it orders the trend. Getting it wrong is worse than not having it.

### S-12. Humans need a monotonic handle [S]

`archive_serial_number` — a unique, monotonic, human-facing integer
(`models.py:295-311`) for cross-referencing physical paper. Unglamorous, and
exactly what an office with a filing cabinet needs. A `document_guid` is not
something anyone writes on a folder tab.

### S-13. Supersession is a different relation from idempotency [S]

`root_document` / `version_index` / `version_label` (`models.py:313-336`), with a
partial unique constraint on `(root_document, version_index)`.

W4's `content_sha256` idempotency answers *"these are the same bytes"*.
Supersession is *"different bytes, same business object"* — a corrected invoice,
a re-run lab. Both relations are needed and they are not the same edge. Only the
first is currently specced.

---

## NT. What must NOT be transferred

**NT-1. The estimator stack.** `MLPClassifier` + `CountVectorizer` + NLTK +
pickle + HMAC (`classifier.py:97-589`). Wrong language, violates
no-serialization-in-the-hot-path (Firewall ADR-022/023), and duplicates
`deepnsm` / `lance-graph-arm-discovery`. Take the *meta*-architecture (§S-10),
never the estimator.

**NT-2. The data model.** `Document.correspondent` / `tags` / `document_type`
are human **filing** metadata. OGAR's model is `classid → ClassView → facet
rails`. Grafting their schema imports a competing identity model. Steal the
pipeline and the dispatch; never the schema.

**NT-3. Storage-path templating.** `generate_filename` with `_NN` uniqueness
counters (`file_handling.py:44-185`) is good work, but `W4-8` already rules
storage backend to be the consumer's. Consumer-side by charter.

**NT-4. The dual store — and this one has a measured cost.** Their full-text
index (Whoosh) is a second authority beside the database, and
`index_document` carries `autoretry_for=(SearchIndexLockError,), max_retries=5,
retry_backoff=60` (`tasks.py:88-95`) — lock-contention retries that exist
*because* of the duality.

**Deliberate deferral (operator direction):** a `tantivy` inverted index is
**not** adopted as the search layer. Our extraction produces typed fields with
provenance, so the queries that force paperless into full-text search
("invoices from supplier X in period Y") are **structural queries the SoA
columns already answer** — and `palette256` answers similarity at ρ=0.9973 vs
cosine, deterministically, where BM25 is merely lexical. Paperless leans on
full-text search because its extraction stops at a flat `content` blob; ours
does not.

If free-prose retrieval is later wanted for a specific use case, the clean shape
is already implied by `W4-2`: text/key/value strings live **out-of-line in
value-slab stores keyed by classid+identity**. An index over *that*, joined by
`document_guid`, is a lens — never a parallel source of truth. One authority,
one join key, no sync problem.

---

## Where this leaves W4

The build spec is not weakened by any of the above. What §A identifies is that
W4's scope **begins one stage too late** — it is a persistence and
reconstruction spec, and the ingestion stage in front of it is undefined. The
five §A invariants are the shape that stage should take, and `S-2` (dedup ahead
of recognition spend) and `S-5` (write order) are the two that would be
expensive to retrofit.

§C and §D are not W4's business at all — they belong to the rule layer
(`ogar-loco`) and the consumer. They are recorded here so the council can see
the full arc the document layer sits inside, and so `typed_field 0x080A` is
minted knowing what it will eventually have to carry.

---

## DISCOVERY-MAP entry (mandatory per CLAUDE.md)

Append `D-OGAR-DOC-SPINE` to `docs/DISCOVERY-MAP.md`: transferred production
experience from a decade-old document system, extracted as 13 invariants over
ingestion ordering, decision centralisation, rule attachment, and business
identity; identifies that `OGAR-DOC-W4-BUILD-SPEC` begins one stage after
ingestion. Grade `[S]` throughout (transferred claim, not measured here) except
`S-4`'s rule, which is independently corroborated in `tesseract-rs`
(`region_is_table`) and grades `[H]`. Falsifiers named for `S-2` and `S-8`.

---

## Sources

- `paperless-ngx` @ `2a8579f` (`AdaWorldAPI/paperless-ngx`, clean upstream fork —
  no local divergence; read as upstream).
- `OGAR-DOC-W4-BUILD-SPEC.md`, `OGAR-DOC-LAYER-PROPOSAL.md` (this repo).
- `tesseract-rs` `docs/CONSUMER-GUIDE.md`, `crates/tesseract-ocr/src/structured.rs`.
- `ogar-loco/src/{lib.rs,vocabulary.rs}`; `lance-graph-arm-discovery/src/lib.rs`.
- `woa-rs/crates/skr_data/` (SKR03/SKR04), `buchungs_validator`, `datev_encoder`.
