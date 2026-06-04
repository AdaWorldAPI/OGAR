# SoA Implementation — Structure-of-Arrays through Every Layer

> **Purpose.** Carves how Structure-of-Arrays (SoA) — the columnar
> Apache Arrow form — flows through every OGAR layer: storage
> (Lance), contract (NiblePath identity routing), IR (OGAR vocab
> types as RecordBatches), adapter (SurrealQL DDL ↔ Arrow IPC),
> and runtime (Ractor + Kanban-bounded actor mailboxes). One
> wire form, four consumption layers, zero impedance mismatch.
>
> Companion to `ADAPTERS-AND-ACTORS.md`, `IDENTITY-MAPPING.md`,
> `ODOO-TRANSCODING.md`. Where those carve WHAT and WHERE, this
> doc carves HOW the bytes move.
>
> Status: **CARVED v0** (2026-06-04).

## 1. The four-layer SoA stack

```
┌──────────────────────────────────────────────────────────────────┐
│  Layer 4: runtime  — Ractor + Kanban-bounded mailboxes           │
│  Actors consume / produce RecordBatch slices.                    │
│  WIP limit per actor; pull-based message scheduling;             │
│  backpressure via mailbox capacity.                              │
├──────────────────────────────────────────────────────────────────┤
│  Layer 3: adapter  — SurrealQL DDL ↔ Arrow IPC                   │
│  surrealdb-core::sql::parse for inbound DDL.                     │
│  surrealdb-ast → Arrow RecordBatch via OGAR vocab.               │
│  Bidirectional: emit DDL from RecordBatch too.                   │
├──────────────────────────────────────────────────────────────────┤
│  Layer 2: IR  — OGAR vocab types as RecordBatch columns          │
│  Class, Association, EnumDecl, Attribute, Action…                │
│  each is one column-set in an Arrow schema.                      │
│  Nested types (Vec<Association>) → Arrow ListArray.              │
├──────────────────────────────────────────────────────────────────┤
│  Layer 1: contract  — NiblePath + Lance versions                 │
│  Identity strings indexed via prefix-radix dictionary encoding.  │
│  Lance Dataset with v2 manifest paths; append-only versions.     │
├──────────────────────────────────────────────────────────────────┤
│  Layer 0: storage  — Lance / Apache Arrow IPC                    │
│  Columnar bytes on disk. SoA throughout — no row-form anywhere.  │
└──────────────────────────────────────────────────────────────────┘
```

**Carve-out**: SoA is the wire form at every layer. No row-form
transformations between layers; conversions are
column-projection operations only.

## 2. Layer 0–1: Storage + Contract (Lance + NiblePath)

### 2.1 Lance dataset shape

Per R2 research findings: Lance 2.2+ adaptive structural encoding
supports `VariablePackedStruct` and native `Map` types. An
`ogar:Class` with `associations: Vec<Association>` lands natively
— no flattening, no JSON column.

```
ogar_classes.lance/
├── _versions/              # v2 manifest paths (O(1) latest lookup)
│   ├── 0000.manifest
│   ├── 0001.manifest
│   └── ...
└── data/
    └── fragments/
        ├── 00000001/
        │   └── *.lance     # one fragment per append batch
        └── 00000002/
            └── ...
```

**Carve-out**: enable `v2 manifest paths` from day one (breaks
readers < lance 0.10.0 but unblocks O(1) opens at scale per R2
gotcha #1).

### 2.2 NiblePath identity dictionary encoding

The identity strings (`ogit-op::WorkPackage::memberof::project`)
share enormous prefixes. Encode as Arrow `DictionaryArray<Utf8>`:

```
identity column:
  raw values: ["ogit-op", "ogit-erp", "ogar", ...]  ← dictionary
  indices:    [0, 1, 0, 2, ...]                     ← values
```

NiblePath is the segment-level dictionary. Each path-segment is
a 27-bit identity (per the lance-graph-contract spec). Storing
N triples for the same class shares the class-prefix bytes —
the compression-to-the-floor property.

**Carve-out**: identity column is ALWAYS dictionary-encoded.
Plain-string identity columns are a producer bug.

### 2.3 Append-only with bounded fragmentation

Per R2 gotcha #2: avoid tiny fragments. Batch appends to ≥1/min
or buffer producer-side. Schedule periodic `compact_files()` +
`cleanup_old_versions()` (but cleanup destroys time-travel —
mark "frozen" ontology versions as tags to preserve).

## 3. Layer 2: IR — OGAR vocab as RecordBatches

### 3.1 One RecordBatch schema per top-level vocab type

```rust
// In crates/ogar-vocab-soa/ (Sprint 4):

pub fn class_record_batch_schema() -> Schema {
    Schema::new(vec![
        Field::new("identity",          DataType::Dictionary(Box::new(DataType::UInt32), Box::new(DataType::Utf8)), false),
        Field::new("name",              DataType::Utf8, false),
        Field::new("parent",            DataType::Utf8, true),
        Field::new("language",          DataType::Dictionary(Box::new(DataType::UInt8), Box::new(DataType::Utf8)), false),
        Field::new("declared_in_module", DataType::Utf8, true),
        Field::new("source_version",    DataType::Utf8, true),
        Field::new("description",       DataType::Utf8, true),
        Field::new("record_order",      DataType::Utf8, true),
        Field::new("rec_name",          DataType::Utf8, true),
        Field::new("abstract_model",    DataType::Boolean, false),
        Field::new("transient",         DataType::Boolean, false),
        Field::new("auto_create_table", DataType::Boolean, true),
        Field::new("log_access",        DataType::Boolean, true),
        Field::new("inheritance_column_disabled", DataType::Boolean, false),
        // Nested: associations as ListArray of struct
        Field::new("associations",      DataType::List(Arc::new(
            Field::new("association_struct", DataType::Struct(association_fields()), false)
        )), false),
        Field::new("enums",             DataType::List(...), false),
        Field::new("scopes",            DataType::List(...), false),
        Field::new("callbacks",         DataType::List(...), false),
        Field::new("computed_fields",   DataType::List(...), false),
        Field::new("methods",           DataType::List(...), false),
        Field::new("validations",       DataType::List(...), false),
    ])
}
```

Each `ogar:Class` instance is one row in this RecordBatch. The
nested associations / enums / scopes are ListArrays of structs
— SoA all the way down per Lance 2.2's `VariablePackedStruct`.

### 3.2 Conversion: vocab struct ↔ RecordBatch

```rust
pub fn classes_to_record_batch(classes: &[Class]) -> RecordBatch;
pub fn record_batch_to_classes(batch: &RecordBatch) -> Vec<Class>;
```

Both directions are pure SoA — no row-form intermediate. Columns
are built via ArrayBuilder per field; nested ListArrays via
ListBuilder.

**Carve-out**: the round-trip
`classes → batch → classes` is identity (modulo `Default`
field ordering). Property-test it.

### 3.3 Action triples as their own RecordBatch

`ogar:Action` triples live in a separate RecordBatch schema with
SPO+TeKaMoLo columns:

```rust
pub fn action_record_batch_schema() -> Schema {
    Schema::new(vec![
        Field::new("action_identity",  DataType::Dictionary(..., Utf8), false),
        Field::new("action_subject",   DataType::Dictionary(UInt8, Utf8), false),  // ogar:User/System/Cron/...
        Field::new("action_predicate", DataType::Utf8, false),
        Field::new("action_object",    DataType::Dictionary(..., Utf8), false),
        Field::new("temporal",         DataType::Dictionary(UInt8, Utf8), false),  // Immediate/Deferred/...
        Field::new("kausal_spec",      DataType::Utf8, true),
        Field::new("modal",            DataType::Dictionary(UInt8, Utf8), false),
        Field::new("lokal_actor",      DataType::Dictionary(..., Utf8), false),
        Field::new("method_body",      DataType::Utf8, true),
    ])
}
```

Two RecordBatch schemas (Class + Action) cover both ingestion
arms (data + behavior). Same lance-dataset; same prefix-radix.

## 4. Layer 3: Adapter — SurrealQL DDL ↔ Arrow IPC

### 4.1 SurrealQL parser integration (per R4)

```rust
// In crates/ogar-adapter-surrealql/:

use surrealdb_core::sql::statements::{DefineTable, DefineField};
use surrealdb_core::sql::parse;

pub fn parse_surrealql_ddl(input: &str) -> Result<Vec<Class>> {
    let stmts = parse(input)?;
    let mut classes = HashMap::new();
    for stmt in stmts.iter() {
        match stmt {
            Statement::Define(Define::Table(t)) => {
                classes.entry(t.name.clone()).or_insert_with(|| Class::new(&t.name));
            }
            Statement::Define(Define::Field(f)) => {
                let class = classes.entry(f.what.clone()).or_insert_with(|| Class::new(&f.what));
                // map TYPE record<x> → Association(BelongsTo) with class_name = x
                // map TYPE string + ASSERT $value IN [...] → EnumDecl
                // map TYPE option<X> → Attribute with required=false
                // ...
                apply_define_field(class, f);
            }
            _ => {}
        }
    }
    Ok(classes.into_values().collect())
}

pub fn emit_surrealql_ddl(classes: &[Class]) -> String {
    // Reverse direction; produces DEFINE TABLE + DEFINE FIELD DDL
    // ...
}
```

### 4.2 Arrow IPC as the wire form

When SurrealQL DDL is parsed → `Vec<Class>` → RecordBatch → Arrow
IPC bytes → lance-graph write.

When OGAR reads → Arrow IPC → RecordBatch → `Vec<Class>` →
emit_surrealql_ddl → SurrealQL string.

**Bidirectional round-trip property**: parse(emit(parse(x))) == parse(x)
for any well-formed SurrealQL DDL. Tested via proptest.

### 4.3 surrealdb-core dependency pin

Per R4 verdict: depend on `surrealdb-core` SQL module with
exact-version pin. Migrate to `surrealdb-parser` + `surrealdb-ast`
crates when they reach crates.io.

## 5. Layer 4: Runtime — Ractor + Kanban

> **CORRECTION (2026-06-04, decision #3 shipped).** The `tokio::sync`
> sketches in §5.1–5.2 below are SUPERSEDED. The shipped reference impl
> is `lance-graph-callcenter::version_watcher` (`LanceVersionWatcher`),
> and it is built on **`std::sync::{Arc, RwLock, Mutex, Condvar}`, NOT
> tokio** — per the upstream **I-2 invariant**: *"tokio is reserved for
> Layer-3 outbound sinks (PhoenixServer, PostgRestHandler); the hot loop
> never uses `tokio::sync`."* The corrected hot-path shape:
>
> ```text
> hot path (NO tokio):
>   lance-graph-planner consumes via LanceVersionWatcher::subscribe()
>     → WatchReceiver
>     → wait_changed()  parks on std::sync::Condvar
>     → current()       returns Arc<CognitiveEventRow>  (Arrow-scalar; BBB invariant)
> ```
>
> SoA bridge ownership: **`lance-graph-ontology`** owns the identity
> register + classes + codebooks; **`lance-graph-callcenter`** owns the
> `LanceMembrane` (sole writer) + the watcher + `CognitiveEventRow`.
> OGAR's `ogar-runtime` is the **std::sync subscriber** that reacts to
> version ticks (cache-invalidate + WIP pull). Ractor/tokio, if used at
> all, are the **SLA-coordination / Layer-3 cold path ONLY** — never the
> hot loop. The `KanbanMailbox` below must therefore be re-expressed in
> `std::sync` (`Mutex<VecDeque> + Condvar`) for the hot path; a
> tokio-channel variant is permissible only on the cold coord side.
>
> Everything from "### 5.1" to the end of §5.2 is kept for historical
> context (the WIP/pull/backpressure *policy* is still correct); the
> `tokio::sync` *mechanism* is replaced by the std::sync Condvar pattern
> above. See `docs/TEMPORAL-TIME-TRAVEL.md` for the full corrected
> integration.

### 5.1 Ractor actor per OGAR class

Per R3 finding: Ractor is the chosen actor framework. Tokio-native,
actively maintained, supervision tree maps cleanly to OGAR class
hierarchy.

```rust
use ractor::{Actor, ActorRef, Message};

pub struct ClassActor {
    pub class_identity: Identity,     // routes via NiblePath
    pub mailbox_capacity: usize,      // Kanban WIP limit
    pub current_wip: usize,           // current in-flight messages
    pub lance_dataset: Arc<Dataset>,  // shared SoA store
}

impl Actor for ClassActor {
    type Msg = ActionMsg;             // SPO+TeKaMoLo annotated
    type State = ClassActorState;
    type Arguments = ClassActorConfig;

    async fn handle(&self, msg: ActionMsg, state: &mut Self::State) -> ActorResult {
        // 1. Check Kausal precondition (state guard) from the SPO+TeKaMoLo
        // 2. If Modal=Atomic, wrap in Lance transaction
        // 3. Read affected rows from lance-graph (SoA column projection)
        // 4. Apply Predicate to Object
        // 5. Append new version to lance-dataset (SoA write)
        // 6. Emit downstream Actions (cascade subjects) — kanban-pull-bounded
    }
}
```

### 5.2 Kanban — bounded WIP + pull-based scheduling

A Kanban-style actor mailbox carries three policies:

1. **WIP limit**: `mailbox_capacity` caps in-flight messages.
   When full, the actor rejects new messages with backpressure
   (sender retries / queues upstream).

2. **Pull-based**: actors don't push to downstream; downstream
   actors PULL when their own WIP is below limit. This prevents
   pipeline stalls under load spikes.

3. **Backpressure signal**: full mailbox propagates a
   `Backpressure(actor_identity)` event upstream so producers
   can pace their emit rate.

```rust
pub struct KanbanMailbox<M: Message> {
    queue: tokio::sync::mpsc::Sender<M>,
    capacity: usize,
    wip: AtomicUsize,
    backpressure_tx: tokio::sync::watch::Sender<bool>,
}

impl<M: Message> KanbanMailbox<M> {
    pub async fn send_if_room(&self, msg: M) -> Result<(), KanbanBackpressure> {
        if self.wip.load(Ordering::Acquire) >= self.capacity {
            return Err(KanbanBackpressure);
        }
        self.wip.fetch_add(1, Ordering::Release);
        self.queue.send(msg).await.map_err(|_| KanbanBackpressure)?;
        Ok(())
    }

    pub async fn pull(&self) -> Option<M> {
        let msg = self.queue.recv().await?;
        self.wip.fetch_sub(1, Ordering::Release);
        self.backpressure_tx.send(false).ok();  // free slot
        Some(msg)
    }
}
```

**Carve-out**: Kanban mailbox is the dispatch interface for every
`ClassActor`. No actor accepts unbounded message queues.

### 5.3 SoA on the wire between actors

When actor A sends N actions to actor B, the wire form is a
**RecordBatch** of `action_record_batch_schema()` — one row per
action, columnar. This batches N actions into one Arrow IPC
message, far cheaper than N individual message sends.

```rust
// Actor A emits a batch of cascade actions:
let cascade_batch: RecordBatch = build_action_batch(cascade_actions)?;
actor_b.send_batch(cascade_batch).await?;
//        ^ kanban-bounded; if B's WIP is high, applies backpressure
```

Actors that produce batches faster than consumers drain them get
backpressured naturally via the Kanban mailbox.

### 5.4 Mapping to lance-graph-callcenter

The four-crate stack:

| Crate                    | Role                                              |
|--------------------------|---------------------------------------------------|
| `lance-graph-contract`   | NiblePath identity routing + Lance versioning + SoA dictionary encoding |
| `lance-graph-ontology`   | Class registry, ontology cache, hot reload via append |
| `lance-graph-planner`    | Query plans over RecordBatches; column projection optimizer |
| `lance-graph-callcenter` | Actor supervisor + Kanban mailbox dispatch + cascade routing |

Each crate consumes SoA RecordBatches and produces SoA outputs.
There is NO row-form anywhere in the pipeline.

## 6. End-to-end SoA flow

Concrete trace: a SurrealQL DDL change `DEFINE FIELD priority ON
work_package TYPE int` arrives via an admin RPC.

```
1. Layer 3 adapter:
   parse_surrealql_ddl(input)
     → ogar_vocab::Class { name: "work_package",
                            attributes: [Attribute { name: "priority", type_name: "int", ... }] }

2. Layer 2 IR:
   classes_to_record_batch(&[class])
     → RecordBatch (class_record_batch_schema)

3. Layer 1 contract:
   identity = class_identity_versioned("ogit-op", "work_package", new_version)
   dataset.append(record_batch).await   # appends one Lance fragment, manifests new version

4. Layer 0 storage:
   Lance writes the fragment; commit increments _versions/N.manifest

5. Layer 4 runtime — cascade:
   lance-graph-ontology watcher sees new version → invalidates cache
   lance-graph-callcenter spawns/refreshes WorkPackageActor for new ontology version
   Existing actors continue serving messages addressed to @v(N-1)
   New incoming messages route to @vN actor via NiblePath
```

Every layer reads and writes the same RecordBatch shape. No
conversion overhead between layers.

## 7. Performance carve-outs

1. **Identity columns are always dictionary-encoded** (Layer 1).
   Plain Utf8 identity columns are a producer bug.

2. **Append granularity is ≥1 message/second OR ≥100 messages/batch**.
   Smaller appends create tiny Lance fragments (R2 gotcha #2).
   Producers must batch.

3. **Cleanup boundary**: `cleanup_old_versions()` only runs on
   versions older than 1 hour, with explicit retention for tagged
   ("frozen") ontology versions.

4. **Kanban WIP limit**: default 1024 messages per actor mailbox.
   Configurable per-class via `ogar:mailboxCapacity` triple.

5. **Column projection on read**: actors read only the columns
   they need. The `lance-graph-planner` rewrites queries to push
   projection down to Lance scans.

6. **Vec growth in IR builders**: `Vec::with_capacity` at every
   RecordBatch-building boundary, sized from known input length.

## 8. Carve-outs summary (the non-negotiable list)

1. **SoA throughout** — no row-form anywhere. All four layers
   read and write Arrow RecordBatches.

2. **One RecordBatch schema per top-level OGAR vocab type**:
   `class_record_batch_schema()` and `action_record_batch_schema()`.

3. **Identity is always dictionary-encoded** at Layer 1. Plain
   string identity columns are a bug.

4. **v2 manifest paths enabled from day one** (no legacy reader
   compat).

5. **SurrealQL adapter is bidirectional via surrealdb-core**.
   Pin exact version; migrate to surrealdb-parser when crates.io.

6. **Ractor + Kanban mailbox per ClassActor**. No unbounded
   queues. Backpressure propagates upstream by default.

7. **Inter-actor wire form is RecordBatch IPC**. No per-action
   individual sends; N actions = 1 batch.

8. **The four lance-graph crates each consume + produce SoA**.
   No row-form intermediate at any crate boundary.

## 9. Sprint impact

| Sprint | Deliverable                                                              |
|--------|--------------------------------------------------------------------------|
| 4      | `crates/ogar-vocab-soa/` — RecordBatch schemas + conversions             |
| 4.5    | `crates/ogar-adapter-surrealql/` with surrealdb-core dependency          |
| 5      | `lance-graph-contract` integration — identity dictionary + Lance writes  |
| 6      | `lance-graph-ontology` cache reads SoA RecordBatches                     |
| 7      | `lance-graph-callcenter` Ractor + Kanban prototype                       |
| 7.5    | End-to-end: SurrealQL DDL → lance-graph → actor dispatch in <10ms p99    |

## 10. Cross-references

- `docs/IDENTITY-MAPPING.md` — Identity struct, Role enum, path syntax variants
- `docs/ODOO-TRANSCODING.md` — Odoo IR surface
- `docs/ADAPTERS-AND-ACTORS.md` — adapter trait + SPO+TeKaMoLo
- `.claude/PLAN.md` — Sprint 4 + 4.5 + 5 + 6 + 7 + 7.5
- `.claude/board/EPIPHANIES.md` — Lance gotchas (R2), Ractor verdict (R3),
  SurrealQL parser availability (R4)
- Apache Arrow docs: <https://arrow.apache.org/docs/>
- Lance docs: <https://lance.org/format/table/>
- Ractor docs: <https://github.com/slawlor/ractor>
- surrealdb-core: <https://crates.io/crates/surrealdb-core>
