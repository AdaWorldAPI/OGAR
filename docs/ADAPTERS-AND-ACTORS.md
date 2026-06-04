# Adapters & Actors — Resolving Semantik / Syntax / Pragmatik

> **Purpose.** Carves the two halves of OGAR's runtime story:
> data-side **HHTL-inherited adapters** (DTO conversion / cross-form
> projection / interfaces) and behavior-side **actors** with
> **SPO + TeKaMoLo** annotations (Subject-Predicate-Object plus
> Temporal-Kausal-Modal-Lokal — full sentence-grammar
> decomposition of business actions). Both halves use the same
> NiblePath prefix-radix routing.
>
> Companion to `IDENTITY-MAPPING.md` (vocabulary) and
> `ODOO-TRANSCODING.md` (Odoo coverage). This doc adds the
> **dynamic** dimension — actions, transactions, business rules.
>
> Status: **CARVED v0** (2026-06-04).

## 1. The two ingestion flows

OGAR ingests both **structure** and **behavior** through the same
AST→IR→triples pipeline:

```
                            ┌─── ERP datasets ────────────┐
   (static / declarative)   │  (CSV, XML, JSON, SQL dump) │
                            └──────────┬──────────────────┘
                                       │
                                       ▼
                            ┌─── DLL / ERP AST ────────────┐
                            │  Class declarations,         │
                            │  field types, relations      │
                            └──────────┬──────────────────┘
                                       │
                                       ▼
                            ┌─── OGAR IR (vocab) ──────────┐
                            │  Class, Association, Enum,   │
                            │  Attribute, ComputedField    │
                            └──────────┬──────────────────┘
                                       │
                                       ▼
                                  lance-graph triples
                                       │
                            ┌──────────┴──────────┐
                            ▼                     ▼
                         storage          actor materialization

   ┌─── ERP transactions / actions / business rules ────────┐
   │  Odoo def action_confirm(), @api.depends,              │
   │  Rails after_save callbacks, hand-rolled business      │
   │  logic, workflow methods, button handlers              │
   └────────────────────────┬───────────────────────────────┘
                            │
                            ▼
                ┌─── DLL / ERP AST (behavior arm) ──────────┐
                │  Method bodies, decorators,               │
                │  state-machine transitions                │
                └────────────────────────┬──────────────────┘
                                         │
                                         ▼
                ┌─── OGAR Action IR (Part B below) ─────────┐
                │  Action with SPO + TeKaMoLo annotation    │
                │  (Subject, Predicate, Object, Temporal,   │
                │   Kausal, Modal, Lokal)                   │
                └────────────────────────┬──────────────────┘
                                         │
                                         ▼
                                   lance-graph triples
                                         │
                                         ▼
                              actor-spec for callcenter
                              (lance-graph-callcenter)
```

**Carve-out**: data ingestion produces `ogar:Class` triples;
behavior ingestion produces `ogar:Action` triples. Same store,
same prefix-radix, **two orthogonal traversal axes** — query
shape (what classes exist) vs choreography (what actions happen).

## 2. Part A — HHTL-inherited DTO adapter

### 2.1 The adapter trait

```rust
pub trait Adapter {
    /// Map an OGAR canonical Identity to a target-form identity
    /// (or emit string). Pure prefix-radix lookup; no semantic
    /// interpretation at lookup-time.
    fn map(&self, canonical: &Identity) -> Option<TargetForm>;

    /// Reverse map: parse a target-form identity back into a
    /// canonical OGAR Identity. Symmetric with `map`.
    fn unmap(&self, target: &TargetForm) -> Option<Identity>;
}
```

Each adapter is a **sparse NiblePath HHTL** of leaves. Walking
the HHTL is O(path-depth), independent of the number of leaves.

### 2.2 HHTL composition is layer-independent

A class rename and a field rename are independent leaves at
different HHTL depths. No global "if class=X then field-rename"
logic — the radix-position determines which leaf applies.

```
                   OGAR canonical                        Odoo target
   class:          ogit-erp::move           →   odoo::transport
   field:          ogit-erp::move::                odoo::transport.
                       attribute::pieces    →       quantity
   association:    ogit-erp::move::                odoo::transport.
                       memberof::driver     →       partner_id
   callback:       ogit-erp::move::                odoo::transport.
                       callback::0::                 write
                       before_save          →
```

Each row is one independent HHTL leaf. The adapter walks both
sides in lock-step.

### 2.3 Bidirectional & composable

Every adapter leaf is bidirectional (parse-side ↔ emit-side).
Two adapters compose: `OdooAdapter ∘ RailsAdapter` maps an
Odoo-canonical identity through OGAR canonical to Rails. The
intermediate canonical form (`ogit-erp::*` / `ogit-op::*`) IS
the meeting point.

```rust
let canonical = odoo_adapter.unmap(&"odoo::transport.write")?;
//   canonical = ogit-erp::move::callback::0::write
let rails = rails_adapter.map(&canonical)?;
//   rails = "Move#after_save :update_status"
```

### 2.4 Inherited DTO shapes

DTOs (Data Transfer Objects) live at the HHTL boundary between
two adapters. A canonical OGAR class becomes:

- An Odoo source-class via `OdooAdapter` (`move → transport`)
- A Rails source-class via `RailsAdapter` (`move → Move`)
- A SurrealQL projection via `SurrealQLAdapter` (`move → DEFINE TABLE move`)
- A TypeScript interface via `TsAdapter` (`move → interface Move {}`)

A class can **inherit** from another class via HHTL prefix
sharing: `ogit-erp::move::lateral_movement` inherits all
adapter leaves under `ogit-erp::move::` automatically. This is
RDFs `subClassOf` modeled via HHTL prefix nesting — and the
adapter pattern composes through inheritance for free.

### 2.5 Adapter as interface contract

The adapter trait IS the interface between OGAR canonical world
and a target language. Each target language has exactly one
adapter; each adapter is a static HHTL.

Adapter equivalence: two adapters covering different targets but
the same canonical paths produce CROSS-COMPATIBLE projections.
If both `OdooAdapter` and `RailsAdapter` map `ogit-erp::move`,
then querying `move` instances across both deployments is one
lance-graph traversal (the prefix `ogit-erp::move::*` lights up
all triples regardless of which producer emitted them).

## 3. Part B — Actors + SPO + TeKaMoLo

### 3.1 The actor abstraction

An **actor** is the runtime materialization of an OGAR class.
Per `lance-graph-callcenter`, every `ogar:Class` registers one
actor; messages addressed to instances of the class flow
through the actor's mailbox.

Each actor accepts **actions**. Each action has structured
annotations: **SPO + TeKaMoLo**.

### 3.2 SPO + TeKaMoLo: full sentence grammar for actions

| Slot | Letter | Meaning | OGAR vocabulary term |
|---|---|---|---|
| **S**ubject | (S) | Who/what initiates the action | `ogar:actionSubject` |
| **P**redicate | (P) | What action is taken | `ogar:actionPredicate` |
| **O**bject | (O) | What is acted upon | `ogar:actionObject` |
| **Te**mporal | (Te) | When does it happen | `ogar:actionTemporal` |
| **Ka**usal | (Ka) | Why / what triggered it | `ogar:actionKausal` |
| **Mo**dal | (Mo) | How is it performed | `ogar:actionModal` |
| **Lo**kal | (Lo) | Where does it execute | `ogar:actionLokal` |

The first three (S/P/O) are the classic RDF triple. The four
**TeKaMoLo** annotations lift the action into a full
sentence-grammar form — borrowed from German adverbial-order
mnemonic (Temporal, Kausal, Modal, Lokal — the canonical
adverbial order in German prose) and applied to business
actions.

This is the **resolution of semantics / syntax / pragmatics**:

- **Semantik** (sign → object): S, P, O — what is being done to what
- **Syntax** (sign → sign): the AST that captured this — method
  decorators, body shape, naming convention
- **Pragmatik** (sign → interpreter): Te, Ka, Mo, Lo — context
  of execution (when, why, how, where)

### 3.3 Action vocabulary

```turtle
ogar:Action a owl:Class ;
    rdfs:label "Business action with full SPO + TeKaMoLo annotation" .

ogar:actionSubject   a rdf:Property ; rdfs:domain ogar:Action ; rdfs:range ogar:ActionSubject .
ogar:actionPredicate a rdf:Property ; rdfs:domain ogar:Action ; rdfs:range xsd:string .
ogar:actionObject    a rdf:Property ; rdfs:domain ogar:Action ; rdfs:range ogar:Class .

ogar:actionTemporal  a rdf:Property ; rdfs:domain ogar:Action ; rdfs:range ogar:TemporalSpec .
ogar:actionKausal    a rdf:Property ; rdfs:domain ogar:Action ; rdfs:range ogar:KausalSpec .
ogar:actionModal     a rdf:Property ; rdfs:domain ogar:Action ; rdfs:range ogar:ModalSpec .
ogar:actionLokal     a rdf:Property ; rdfs:domain ogar:Action ; rdfs:range ogar:LokalSpec .

# Subject enumeration
ogar:ActionSubject a owl:Class .
ogar:User       a ogar:ActionSubject ; rdfs:label "Human user via UI/RPC" .
ogar:System     a ogar:ActionSubject ; rdfs:label "Internal system trigger" .
ogar:Cron       a ogar:ActionSubject ; rdfs:label "Scheduled (ir.cron / Rails Whenever)" .
ogar:Trigger    a ogar:ActionSubject ; rdfs:label "Reactive (DB event / @api.depends)" .
ogar:Cascade    a ogar:ActionSubject ; rdfs:label "Cascade from a parent action" .

# Temporal enumeration
ogar:TemporalSpec a owl:Class .
ogar:Immediate  a ogar:TemporalSpec ; rdfs:label "Synchronous, on-call" .
ogar:Deferred   a ogar:TemporalSpec ; rdfs:label "Queued, async background" .
ogar:Scheduled  a ogar:TemporalSpec ; rdfs:label "Run at specific time/interval" .
ogar:OnCommit   a ogar:TemporalSpec ; rdfs:label "After DB transaction commits" .

# Modal enumeration
ogar:ModalSpec a owl:Class .
ogar:Sync       a ogar:ModalSpec ; rdfs:label "Synchronous, blocking" .
ogar:Async      a ogar:ModalSpec ; rdfs:label "Fire-and-forget" .
ogar:Idempotent a ogar:ModalSpec ; rdfs:label "Safe to retry" .
ogar:Atomic     a ogar:ModalSpec ; rdfs:label "All-or-nothing transaction" .
ogar:Requires   a ogar:ModalSpec ; rdfs:label "Requires user confirmation" .

# Kausal: typed as a reference to the trigger
# (a state, an event, a dependency path, a user-input field)

# Lokal: typed as a reference to the executing actor
# (which ogar:Class actor, which tenant slice, which company)
```

### 3.4 Worked examples

#### 3.4.1 Odoo `def action_confirm(self): ...`

Source:
```python
class SaleOrder(models.Model):
    @api.depends('state')
    def action_confirm(self):
        if self.state != 'draft':
            raise UserError(_("..."))
        self.state = 'sale'
        self._send_order_confirmation_mail()
```

OGAR Action triple:
```turtle
ogit-erp:sale.order::action::confirm_42 a ogar:Action ;
    ogar:actionSubject   ogar:User ;            # invoked from button
    ogar:actionPredicate "confirm" ;
    ogar:actionObject    ogit-erp:sale.order ;
    ogar:actionTemporal  ogar:Immediate ;       # synchronous
    ogar:actionKausal    [ a ogar:StateGuard ;
                           ogar:guardField "state" ;
                           ogar:guardValue "draft" ] ;
    ogar:actionModal     ogar:Atomic ;          # state transition is atomic
    ogar:actionLokal     ogit-erp:sale.order::actor .
```

Plus an associated method-body opaque source (for projection
emission):
```turtle
ogit-erp:sale.order::action::confirm_42
    ogar:methodBody "if self.state != 'draft': raise..." ;
    ogar:resultsIn ogit-erp:sale.order::state-transition::draft-to-sale ;
    ogar:sideEffects [ ogar:emails "_send_order_confirmation_mail" ] .
```

#### 3.4.2 Rails `before_save :touch_parent`

Source:
```ruby
class WorkPackage < ApplicationRecord
  belongs_to :project
  before_save :touch_parent

  def touch_parent
    project.touch
  end
end
```

OGAR Action triple:
```turtle
ogit-op:WorkPackage::action::touch_parent_42 a ogar:Action ;
    ogar:actionSubject   ogar:Cascade ;         # cascaded from save
    ogar:actionPredicate "touch_parent" ;
    ogar:actionObject    ogit-op:Project ;      # target IS the parent
    ogar:actionTemporal  ogar:OnCommit ;        # before_save fires pre-commit
    ogar:actionKausal    [ a ogar:LifecycleTrigger ;
                           ogar:triggerEvent "save" ] ;
    ogar:actionModal     ogar:Atomic ;          # in the same transaction
    ogar:actionLokal     ogit-op:WorkPackage::actor .
```

#### 3.4.3 Odoo cron job

Source:
```xml
<record id="cron_close_open_orders" model="ir.cron">
    <field name="model_id" ref="model_sale_order"/>
    <field name="code">model._cron_close_open_orders()</field>
    <field name="interval_number">1</field>
    <field name="interval_type">days</field>
</record>
```

OGAR Action triple:
```turtle
ogit-erp:sale.order::action::cron_close_42 a ogar:Action ;
    ogar:actionSubject   ogar:Cron ;
    ogar:actionPredicate "_cron_close_open_orders" ;
    ogar:actionObject    ogit-erp:sale.order ;
    ogar:actionTemporal  [ a ogar:Scheduled ;
                           ogar:cronInterval "1 day" ] ;
    ogar:actionKausal    ogar:None ;            # no precondition
    ogar:actionModal     ogar:Atomic ;
    ogar:actionLokal     ogit-erp:sale.order::actor .
```

### 3.5 Mapping to existing OGAR types

| Existing OGAR type | Becomes Action with...                               |
|--------------------|------------------------------------------------------|
| `Callback`         | `actionPredicate` = event name; `actionKausal` = lifecycle |
| `MethodDecl{CrudOverride}` | `actionPredicate` = method name; `actionSubject` = User/Cascade |
| `MethodDecl{ApiModelCreateMulti}` | bulk-create action with `actionModal` = Atomic |
| `Validation`       | `actionPredicate` = "validate"; `actionKausal` = field-change |
| `Workflow.Transition` | `actionPredicate` = transition method; full SPO+TeKaMoLo always |
| `ScheduledJob` (ext) | `actionSubject` = Cron; `actionTemporal` = Scheduled |
| `ComputedField`    | `actionSubject` = Trigger; `actionKausal` = `@api.depends` paths |

The existing `Callback`, `MethodDecl`, `Validation` etc. are
STRUCTURAL captures. The `Action` adds the SPO+TeKaMoLo
**pragmatic** annotation on top — turning a method into a
fully-described business operation.

**Carve-out**: every `Callback` / `MethodDecl` / `Validation` /
`Workflow.Transition` / `ScheduledJob` / `ComputedField` SHOULD
have a corresponding `Action` triple with full SPO+TeKaMoLo.
The structural type captures syntax; the Action captures
pragmatik.

### 3.6 The actor as resolved sentence

A `lance-graph-callcenter` actor for a class processes messages
that ARE Actions (in the SPO+TeKaMoLo sense). Dispatch:

```
incoming message: Action(S=User, P=confirm, O=SaleOrder.42,
                         Te=Immediate, Ka=state=draft,
                         Mo=Atomic, Lo=sale.order::actor)
            │
            ▼
ontology lookup: find actor for ogit-erp::sale.order via NiblePath
            │
            ▼
actor dispatch: SaleOrderActor receives Action
            │
            ▼
guard check (Ka): is state == draft? (Kausal precondition)
            │
            ▼
execute (Mo=Atomic): wrap in DB transaction
            │
            ▼
state transition (P=confirm): state := sale
            │
            ▼
emit downstream Actions (Cascade subjects):
  - confirm_42_email_send  (modal=Async, temporal=OnCommit)
  - confirm_42_inventory_check  (cascade to stock.move::actor)
```

The SPO+TeKaMoLo decomposition is what makes the actor's
behavior **fully introspectable** at the IR level. No
"black-box method body" — every action declares its full
sentence-grammar shape.

## 4. Interaction between Part A and Part B

The HHTL-inherited adapter (Part A) and the actor model (Part B)
**share the same NiblePath prefix-radix**. Consequences:

1. **Adapter renames at all SPO+TeKaMoLo levels**: an Odoo
   adapter renames not only class names (`move → transport`)
   but also action predicates (`action_confirm → confirm`),
   subject types, modal specs. All as HHTL leaves.

2. **Actor instances are named by HHTL paths**: the actor
   registered for `ogit-erp::sale.order` IS reachable as
   `actor:ogit-erp::sale.order::actor` — same prefix-radix.
   Cross-system queries traverse the same index.

3. **TeKaMoLo annotations are HHTL leaves too**: a `ogar:Temporal`
   spec is a node in the HHTL; the adapter for Odoo maps it to
   Odoo's interval syntax, the adapter for Rails maps it to
   `Whenever` schedule, etc. Same as class/field renames.

4. **DTO conversions across actor boundaries**: when an actor
   on node A sends a message to an actor on node B, the message
   is a DTO. Adapter pattern handles the on-the-wire form. SPO+
   TeKaMoLo content is unchanged across the wire (semantics +
   pragmatics preserved); only the syntactic form is adapted.

## 5. Producer responsibilities

### 5.1 Data producer (existing — `ogar-from-ruff`, `ogar-python` etc.)
Emits `ogar:Class` and friends — structural data IR. Already
covered by Sprint 1 / Sprint 2.

### 5.2 Behavior producer (new — Sprint 3)
Same producers extended to emit `ogar:Action` triples with full
SPO+TeKaMoLo annotation. For Odoo: walk every
`def action_*`, every `@api.depends`, every `@api.onchange`,
every cron XML record, and emit one `ogar:Action` triple per.
For Rails: every `before_save`, `after_create`, every `def`
that overrides ApplicationRecord lifecycle methods.

The behavior producer SHOULD use the same AST already parsed
for the structural producer — no second AST walk needed.

### 5.3 Adapter (new — Sprint 3)
Implements `Adapter` trait with HHTL of leaf renames. Provided
per target (OdooAdapter, RailsAdapter, SurrealQLAdapter,
PostgresAdapter, ...). Adapters are STATIC lookup tables; the
canonical form is the runtime currency.

## 6. Carve-outs (non-negotiable)

1. **Two ingestion arms, same store**: data and behavior land
   as `ogar:Class` and `ogar:Action` triples respectively.
   Same lance-graph dataset; same prefix-radix; orthogonal
   traversal axes.

2. **SPO + TeKaMoLo is the complete action grammar**:
   Subject + Predicate + Object + Temporal + Kausal + Modal +
   Lokal. Seven slots, no eighth. New dimensions become
   sub-properties of existing slots.

3. **Every Callback / MethodDecl / Validation / Workflow /
   ScheduledJob / ComputedField has a matching Action triple**.
   The structural capture (syntax) AND the pragmatic capture
   (TeKaMoLo) coexist; one is not a substitute for the other.

4. **Adapter is a static HHTL of leaves**. No semantic
   interpretation at lookup-time. No conditional logic. Just
   prefix-radix walk. This is the SKOS "minimal ontological
   commitment" principle applied to adapters.

5. **Adapter renames compose across HHTL levels**. Class-name,
   field-name, role-name, action-predicate renames are
   independent leaves; no cross-leaf dependencies allowed.

6. **Adapters are bidirectional**. `map(canonical) → target`
   and `unmap(target) → canonical` are inverse functions on
   their respective leaf sets.

7. **Actor identity uses the same NiblePath prefix-radix** as
   class identity. Actors live under their class's prefix:
   `ogit-erp::sale.order::actor`. Dispatch is one radix-lookup.

8. **TeKaMoLo annotations are HHTL leaves too** — adapter
   renames them like any other path segment.

9. **Cross-system actor communication preserves SPO+TeKaMoLo**;
   only the syntactic form is adapter-rewritten for the wire.
   Semantics + pragmatics are wire-invariant.

10. **The actor is the resolved sentence**. The actor processes
    the SPO+TeKaMoLo grammatically-decomposed action. No
    opaque method-body execution; every step is annotated.

## 7. Sprint impact

| Sprint | Deliverable                                                   |
|--------|---------------------------------------------------------------|
| 3      | `ogar:Action` vocabulary + `ogar:actionTemporal` etc. terms  |
|        | `MethodDecl` → `Action` synthesis in `ogar-from-ruff`, `ogar-python` |
|        | First adapter trait skeleton (`crates/ogar-adapter/`)        |
| 3.5    | `OdooAdapter` HHTL with leaves for the 12 carved Odoo concepts |
| 3.6    | `RailsAdapter` HHTL with leaves for the 12 carved Rails concepts |
| 4      | Producer extension to emit `Action` triples alongside `Class` |
| 7      | `lance-graph-callcenter` consuming `Action` triples for dispatch |

## 8. Cross-references

- `docs/IDENTITY-MAPPING.md` — base vocab carve-out (Role, Identity, syntax variants)
- `docs/ODOO-TRANSCODING.md` — Odoo-specific vocab + structural ingestion
- `.claude/VISION.md` — design principles (minimal commitment, compatible extensions, defer, two-layer spec)
- `.claude/PLAN.md` — Sprint 3 + 3.5 + 3.6 (Action vocabulary + adapter HHTL impls)
- `vocab/ogar.ttl` — terms; new `ogar:Action`, `ogar:ActionSubject`, `ogar:TemporalSpec` etc. added when Sprint 3 lands
- `vocab/ogar-bridges.ttl` (Sprint 2.5) — `skos:exactMatch` cross-vocab mappings (the per-adapter leaf representation)
- Lance-graph: `lance-graph-callcenter` (Sprint 7) consumes Action triples
