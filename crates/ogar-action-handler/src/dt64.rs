//! `dt64` — register OGAR's [`ActionDef`]s as lance-graph DT64 methods.
//!
//! DT64's [`BindStore`] binds, per classid, an ordered set of
//! [`MethodBinding`]s (identity, ON x WHEN x DO, read set). OGAR's authoritative
//! domain tables ([`ogar_vocab::capability_registry::domain_tables`]) declare
//! the same behaviour as [`ActionDef`]s. Until this module nothing took one to
//! the other: [`method_of`] converts one def, [`register_domain_table`]
//! registers a whole table, and [`register_domain_table_activated`] does so
//! only after OGAR's own hot-plug resolution ([`resolve_hotplug`]) is green,
//! the order `dt64-hotplug-probe`'s `register_activated` fixes (activation
//! first, then DT64).
//!
//! # Where this lives
//!
//! `ogar-vocab` stays free of lance-graph, so the converter cannot live there.
//! It lives here, behind the optional `dt64` feature, for three reasons: this
//! crate already carries the `ontology-cache` and `quack` features that bring
//! lance-graph by git, so `dt64` is the same pattern and adds no workspace
//! member or pin; its dependency direction is already `ogar-vocab` (the defs)
//! plus lance-graph; and it is where the executor seam
//! ([`ogar_from_schema::action_ws::CapabilityExecutor`]) lives, which is what
//! the external hand-off a [`Registered`] row carries will be bridged to. A
//! separate `ogar-dt64` crate would pull the same two dependencies and
//! duplicate that seam's imports for no gain.
//!
//! # The mapping
//!
//! A behaviour is a method addressed as `class :: F###` (owner ruling
//! 2026-11-18, D-DT64-7 D11''-D15''): the classid's LEFT 16 bits are the class
//! it applies on (ON), the RIGHT 16 bits in `0xF000..=0xFFFF` name the
//! behaviour (DO), and WHEN is what the `register` call supplies. Every
//! [`ActionDef`] registers at its own classid. The two halves have two owners:
//!
//! - the **arm** (right half) belongs to the behaviour: ogar-vocab supplies the
//!   [`ActionDef`]s, and the arm ledger ([`ogar_vocab::arm_ledger`]) gives each
//!   its number from the def's own `identity`. It is the SAME whichever
//!   consumer registers the def.
//! - the **left half** belongs to the registering consumer (D14'': a consumer
//!   registers behaviour under its own class). It comes from the caller's
//!   [`ConsumerClasses`], resolved per def from `object_class`; a def whose
//!   class the consumer does not have is refused by name
//!   ([`ConvertRefusal::ConsumerClassMissing`]), never read as the canonical
//!   concept. The basis is the consumer's own [`ClassView`] (O10).
//!
//! | `ActionDef` | `MethodBinding` / store |
//! |---|---|
//! | `object_class` (last `/` segment) | the consumer class (left half), through [`ConsumerClasses`] |
//! | `identity` | the arm, see [Behaviour arms](#behaviour-arms) |
//! | `predicate` | `name`, carried and never identity |
//! | the arm | `id`: `MethodId(arm)`; one method per behaviour classid |
//! | `kausal: External` | `when: None` |
//! | `kausal: StateGuard` over a field of the class with decimal-literal values | `when: Some(Guard)`, an `Or` of `eq_u32` lanes under `Logic::Kleene` |
//! | `kausal: Depends` over fields of the class | `on` (the positions whose change triggers it) |
//! | `reads` | `reads`: each name must be a field label of the class |
//! | `params`, `results` | not in the binding: the call's parameter list stays with the declaration, carried on [`Registered`] (see [Call parameters](#call-parameters)) |
//! | `writes`, `calls`, `raises` | not in the binding, see [Effects](#effects) |
//! | everything else | refused by name, see [`ConvertRefusal`] |
//! | `required_role` (a const on the gate, not on this `ActionDef`) | not part of the method: it stays on the gate |
//! | priority (arago applicability, not on `ActionDef`) | none |
//!
//! `sig_rev` is `SigRev(1)`, `population` is `Distinct`, `truth` is
//! `ThreeValued` exactly when the guard is Kleene (else `TwoValued`), and
//! `demand` is `Values` when the method reads a field or has a guard, else
//! `MapOnly`.
//!
//! ## Behaviour arms
//!
//! An arm is assigned by a ledger entry, `identity -> arm`, never computed.
//! OGAR ships the canonical ledger for all 50 of its own defs
//! ([`ogar_vocab::arm_ledger::CANONICAL`], `0xF001` upward in table order,
//! append-only, pinned literally by tests). A consumer that registers
//! behaviours OGAR does not know supplies its own entries as [`ConsumerArms`]
//! through [`register_domain_table_with_arms`]; every other entry point uses the
//! canonical ledger alone. The arm is a function of the identity and the ledger
//! only: not of the position in a table, of which other defs exist, of whether
//! a def converts, or of the consumer. A changed behaviour is a new def (a new
//! identity, a new ledger line, a new arm) or a derived classid, never a
//! renumbered one (D5). `0xF000` is never assigned: it is the class's own
//! polyfill lens (the `0x020C_F000` precedent, D-DT64-7 §1g).
//!
//! Consumer arms are kept apart from OGAR's by range: OGAR assigns only
//! `0xF001..=0xF7FF`, a consumer only `0xF800..=0xFFFF`. A consumer entry that
//! reuses a canonical arm, rebinds a canonical identity to another arm, or takes
//! an arm in OGAR's range is refused by name ([`BehaviourArmRefusal::Ledger`],
//! [`ogar_vocab::arm_ledger::LedgerRefusal`]), as is an identity with no ledger
//! entry at all ([`BehaviourArmRefusal::NoLedgerEntry`]): there is no fallback
//! number. A table that lists one identity twice is refused
//! ([`BehaviourArmRefusal::DuplicateDef`]). All of these refuse the whole table
//! before DT64 is called, so a refused def never decides another's fate. There
//! is no collision case and no overflow case: the ledger is unique in both
//! columns by validation, so two identities never share an arm and a class never
//! has more defs than arms. `tests/dt64_methods.rs` pins today's 50 arms
//! through the ledger.
//!
//! ## The view of a behaviour
//!
//! DT64 reads the basis of a behaviour lens (right half `0xF###`) by its FULL
//! classid ([`ClassView::lens_fields`], O9), never from the left half's data
//! basis. [`BehaviourView`] is the consumer's view with each registered
//! behaviour declared as a reading of its class: the lens basis IS the class's
//! field basis, answered explicitly per declared classid, so an undeclared
//! behaviour classid is `UnknownLensBasis`, not an inherited data basis.
//!
//! ## Effects
//!
//! DT64 effects are polyfill words over fields. The Frozen lens
//! (`0xFFFF::0xF000`) has none for an external command, and none is invented
//! here, so a converted method registers with **empty `effects`**, which DT64
//! accepts. The external hand-off is the [`Registered`] row returned beside the
//! id: its `capability` is the name a `commit_via` bridge dispatches on (to a
//! `CapabilityExecutor`), and its `identity` finds the [`ActionDef`] that still
//! holds `writes`, `calls`, `raises`, the statem carriers and `required_role`.
//! [`ConvertRefusal::ExternalEffect`] is therefore never produced; an
//! `on_enter` (a structured `field := value` transition) is refused as
//! [`ConvertRefusal::UnportedEffect`] rather than dropped.
//!
//! ## Effect class
//!
//! Each [`Registered`] row also carries an [`EffectClass`]
//! ([`ogar_vocab::effect_class::classify`], the shared classification of
//! `docs/SHARED-DO-DESIGN.md`). A def is a write, a lifecycle step, a
//! transaction or an external call, never a population fold, so the invariant
//! is: **a method whose class is not F000-admissible
//! ([`EffectClass::is_f000_admissible`]) has empty `effects`**; it is dispatched
//! through the gate by `capability`. [`method_of`] always yields empty effects
//! today, so the invariant holds for every row, and `tests/dt64_methods.rs`
//! checks it so that the day a def does get F000 effects, its class is what
//! allows it.
//!
//! The classes of OGAR's 50 `ActionDef`s (`classify` over the five
//! authoritative tables): all 50 are `ExternalHost`: each is `kausal: External`
//! and names no `reads`, `writes`, `calls`, `raises` or `on_enter`, so the one
//! thing the def evidences is an external cause with a body outside OGAR. (The
//! ocr and document defs were `LifecycleWrite` while their output names sat in
//! `writes`; those names are call results and now live in `results`.) Several
//! of the defs are in fact reads (`get_*`, `list_*`, `locate_*`); the def does
//! not say so, so none is admitted as an F000 query.
//!
//! ## Call parameters
//!
//! A call parameter (`grey_line`, `width`, the inputs of `recognize_line`) is an
//! input of the call, not a stored field of the class. It is declared on the
//! [`ActionDef`] (`params`, with a mandatory/optional flag, and `results`) and
//! never added to the class. The converter resolves `reads`, a guard field and
//! a `Depends` path against the class's field basis only. A call parameter in
//! `reads` is refused as `UnknownField`, and a guard or `Depends` path over a
//! call parameter that is not also a field is refused as
//! [`ConvertRefusal::CallParameterInGuard`]. A def is never refused because it
//! HAS call parameters. The registered method carries class fields only; the
//! parameter list travels on the [`Registered`] row beside the registration, for
//! the executor and the later invocation contract.
//!
//! ## Immutability
//!
//! DT64 never rebinds a classid (D5). Each behaviour classid is registered
//! once; the same def again is accepted, a def with the same identity and
//! another binding (a changed guard, reads) is refused as `BehaviourImmutable`
//! (a derived classid is the override path). Adding defs never touches the
//! earlier classids.
//!
//! # Not migrated yet
//!
//! The source-reading producers (`ogar-from-schema::do_arm::lift_action_defs`,
//! `ogar-from-ruff::lift_actions`, `ogar-from-rails::extract_actions`,
//! `ogar-from-elixir::extract_action_defs`) emit `ActionDef`s too and are not
//! routed through here. They are the next step: [`method_of`] already takes any
//! def, they need a table-shaped caller and a `ClassView` over the harvested
//! class.

use std::collections::{BTreeMap, BTreeSet};

use lance_graph_contract::class_view::{ClassId, ClassView, WideFieldMask, is_behaviour_lens};
use lance_graph_contract::ogar_codebook::{classid_concept, compose_classid};
use lance_graph_contract::ontology::{DisplayTemplate, FieldRef};
use lance_graph_contract::set_algebra::{Logic, SetExpr};
use lance_graph_contract::set_rules::{Population, Truth};
use lance_graph_dt64::{
    Axis, BindStore, ClassSchema, Extent, Guard, MethodBinding, MethodId, Refusal, SchemaRev,
    SigRev, ValueDemand, Width,
};
use ogar_vocab::arm_ledger::ConsumerArms;
pub use ogar_vocab::arm_ledger::LedgerRefusal;
use ogar_vocab::capability_registry::{DomainTable, HotplugDrift, resolve_hotplug};
use ogar_vocab::effect_class::{EffectClass, classify};
use ogar_vocab::{ActionDef, CallParam, KausalSpec, canonical_concept_id};

/// Which `ActionDef` field named a field the class does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldRole {
    /// An entry of `ActionDef::reads`.
    Reads,
    /// The `guard_field` of a `KausalSpec::StateGuard`.
    GuardField,
    /// A path of a `KausalSpec::Depends`.
    DependsPath,
}

/// Why one `ActionDef` was not converted. Every arm is a named reason; a def
/// is never approximated into a binding that means something else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConvertRefusal {
    /// The `object_class` concept is not minted in the codebook, so there is
    /// no classid to bind the method under.
    UnmintedConcept {
        /// The last `/` segment of `object_class`.
        concept: String,
    },
    /// `reads`, a guard field or a `Depends` path names something that is not
    /// a field label of the class (in the view's current basis). A call
    /// parameter such as `grey_line` is such a name: it is not a field.
    UnknownField {
        /// Where the name came from.
        role: FieldRole,
        /// The name as written on the def.
        name: String,
    },
    /// A guard field or a `Depends` path names a call parameter of the def
    /// (`ActionDef::params`) that is not a stored field of the class. A guard
    /// is over stored fields only.
    CallParameterInGuard {
        /// Where the name came from.
        role: FieldRole,
        /// The call parameter's name.
        name: String,
    },
    /// The def's `kausal` is not an equality over the class's fields that a
    /// contract `SetExpr` states: a string state value, an empty value set, a
    /// lifecycle event, a validation or UI trigger, a context key.
    UnportedGuard {
        /// The `KausalSpec` variant, and what in it could not be stated.
        kausal: String,
    },
    /// An effect with no Frozen word: the def's `on_enter` transition.
    UnportedEffect {
        /// Which effect carrier.
        what: &'static str,
    },
    /// The consumer's class view has no class for the def's `object_class`
    /// (its last `/` segment): there is no left side to register the behaviour
    /// under. Never read as the canonical concept.
    ConsumerClassMissing {
        /// The last `/` segment of `object_class`.
        concept: String,
    },
    /// An external command with no Frozen word AND an effect list DT64
    /// requires to be non-empty. DT64 accepts an empty list, so this is never
    /// produced today; it is here so the day that changes, the refusal has a
    /// name instead of an approximation.
    ExternalEffect,
}

/// The external hand-off for one registered def: what a later `commit_via`
/// bridge needs that the DT64 binding (which has no word for it) does not
/// carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registered {
    /// The method's identity in its behaviour classid: `MethodId(arm)`.
    pub id: MethodId,
    /// The consumer class the behaviour applies on, as a data classid
    /// (`compose_classid(on, 0)`): the class DT64 reads fields through.
    pub class: u32,
    /// ON: the consumer's class id, the LEFT half of [`Registered::behaviour`].
    pub on: u16,
    /// DO: the behaviour arm, the RIGHT half (`0xF001..=0xFFFF`). The same
    /// for this def whichever consumer registers it.
    pub arm: u16,
    /// The classid the method is registered under: `on :: arm`.
    pub behaviour: u32,
    /// The capability name (`ActionDef::predicate`), the executor's key.
    pub capability: String,
    /// `ActionDef::identity`, which finds the def (its `writes`, `calls`,
    /// `raises`, statem carriers).
    pub identity: String,
    /// The def's call parameters (`ActionDef::params`), in declared order. Not
    /// part of the registered method, which has class fields only.
    pub params: Vec<CallParam>,
    /// The def's call results (`ActionDef::results`), in declared order.
    pub results: Vec<String>,
    /// What the def is known to do, from [`ogar_vocab::effect_class::classify`].
    /// A class for which [`EffectClass::requires_gate`] holds is never an F000
    /// method and goes to the gate (`commit_via`) as an `ActionInvocation`.
    pub effect: EffectClass,
}

/// One def's outcome, in table order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowOutcome {
    /// The def's position in its table (also its [`MethodId`]).
    pub id: MethodId,
    /// `ActionDef::predicate`.
    pub predicate: String,
    /// Registered, or refused by name.
    pub result: Result<Registered, ConvertRefusal>,
}

/// The outcome of one table: every def either registered or refused by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableOutcome {
    /// [`DomainTable::domain`].
    pub domain: &'static str,
    /// One row per def, in declared order.
    pub rows: Vec<RowOutcome>,
}

impl TableOutcome {
    /// The registered rows, in table order.
    pub fn registered(&self) -> impl Iterator<Item = &Registered> {
        self.rows.iter().filter_map(|r| r.result.as_ref().ok())
    }

    /// The refused rows with their reasons, in table order.
    pub fn refused(&self) -> impl Iterator<Item = (&RowOutcome, &ConvertRefusal)> {
        self.rows
            .iter()
            .filter_map(|r| r.result.as_ref().err().map(|e| (r, e)))
    }

    /// The ids that were registered, in table order.
    pub fn method_ids(&self) -> Vec<MethodId> {
        self.registered().map(|r| r.id).collect()
    }
}

/// Why a table was not registered (as opposed to a def not converting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableRefusal {
    /// OGAR's hot-plug resolution refused the consumer for this table; DT64
    /// was not called.
    NotActivated(HotplugDrift),
    /// DT64 refused one class's registration. Classes registered before it
    /// stay registered (the store never rebinds, D5).
    Dt64 {
        /// The classid being registered.
        class: u32,
        /// DT64's refusal.
        refusal: Refusal,
    },
    /// The arms of the table's defs could not be assigned. DT64 was not called.
    Arm(BehaviourArmRefusal),
}

/// Why the arms of a table's defs could not be assigned (see the module doc).
/// None picks a winner or invents a number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviourArmRefusal {
    /// The consumer's ledger entries were refused.
    Ledger(LedgerRefusal),
    /// A def's identity has no entry in the canonical ledger or the consumer's.
    NoLedgerEntry {
        /// The def's `identity`.
        identity: String,
    },
    /// A table lists one identity twice on one consumer class.
    DuplicateDef {
        /// The consumer class (left half).
        on: u16,
        /// The identity listed twice.
        identity: String,
    },
}

/// How a registering consumer names the class a def applies on: the left half
/// of its behaviour classids (D14''). ogar-vocab supplies the behaviours; the
/// consumer supplies this.
pub trait ConsumerClasses {
    /// The consumer's class id for the concept named by the last `/` segment
    /// of a def's `object_class`, or `None` if the consumer has no such class.
    fn class_of(&self, concept: &str) -> Option<u16>;
}

/// An explicit consumer mapping: concept name to the consumer's class id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassMap(BTreeMap<String, u16>);

impl ClassMap {
    /// An empty mapping: every def is refused until classes are added.
    #[must_use]
    pub fn new() -> ClassMap {
        ClassMap::default()
    }

    /// Map `concept` to the consumer's `class`.
    #[must_use]
    pub fn with(mut self, concept: &str, class: u16) -> ClassMap {
        self.0.insert(concept.to_string(), class);
        self
    }
}

impl ConsumerClasses for ClassMap {
    fn class_of(&self, concept: &str) -> Option<u16> {
        self.0.get(concept).copied()
    }
}

/// OGAR's canon as the left side: a concept is its minted canonical id. This
/// is a first-class choice, not a fallback. A generic concept (`crm_ticket`,
/// `patient`) is shared on purpose by several consumers, so their data and its
/// behaviours are one thing and nothing is migrated when a second consumer
/// plugs in: each registers the same def at the same `concept :: F###`. A
/// consumer-specific left side (a seat such as `erp_polyfill`, or the
/// consumer's own class through [`ClassMap`]) is for behaviour that is not
/// meant to be shared. [`register_domain_table`] is this choice.
#[derive(Debug, Clone, Copy, Default)]
pub struct CanonicalClasses;

impl ConsumerClasses for CanonicalClasses {
    fn class_of(&self, concept: &str) -> Option<u16> {
        canonical_concept_id(concept)
    }
}

/// A consumer's [`ClassView`] with its registered behaviours declared as
/// readings of their classes. DT64 reads a behaviour lens (right half
/// `0xF###`) by its FULL classid (O9); this view answers
/// [`ClassView::lens_fields`] (and `lens_fields_at`) only for a declared
/// behaviour, with the field basis of the consumer class it applies on. An
/// undeclared behaviour classid is unknown: it never reads the data class's
/// basis by accident.
#[derive(Debug)]
pub struct BehaviourView<'v, V: ClassView + ?Sized> {
    inner: &'v V,
    readings: BTreeMap<u32, ClassId>,
}

impl<'v, V: ClassView + ?Sized> BehaviourView<'v, V> {
    /// The consumer's view with no behaviour declared.
    pub fn new(inner: &'v V) -> Self {
        BehaviourView {
            inner,
            readings: BTreeMap::new(),
        }
    }

    /// Declare `behaviour` (`on :: 0xF###`) as a reading of consumer class
    /// `on`. A classid whose right half is not in `0xF000..=0xFFFF` is not a
    /// behaviour and is not declared; returns whether it was.
    pub fn declare(&mut self, behaviour: u32) -> bool {
        if !is_behaviour_lens(behaviour) {
            return false;
        }
        self.readings.insert(behaviour, classid_concept(behaviour));
        true
    }

    /// The view with every registered row of `outcomes` declared.
    pub fn from_outcomes<'o>(
        inner: &'v V,
        outcomes: impl IntoIterator<Item = &'o TableOutcome>,
    ) -> Self {
        let mut view = BehaviourView::new(inner);
        for o in outcomes {
            for r in o.registered() {
                view.declare(r.behaviour);
            }
        }
        view
    }
}

impl<V: ClassView + ?Sized> ClassView for BehaviourView<'_, V> {
    fn fields(&self, class: ClassId) -> &[FieldRef] {
        self.inner.fields(class)
    }
    fn template(&self, class: ClassId) -> DisplayTemplate {
        self.inner.template(class)
    }
    fn dolce_category_id(&self, class: ClassId) -> u8 {
        self.inner.dolce_category_id(class)
    }
    fn lens_fields(&self, lens: u32) -> Option<&[FieldRef]> {
        self.readings.get(&lens).map(|&c| self.inner.fields(c))
    }
    fn fields_at(&self, class: ClassId, schema_rev: u32) -> Option<&[FieldRef]> {
        self.inner.fields_at(class, schema_rev)
    }
    fn lens_fields_at(&self, lens: u32, schema_rev: u32) -> Option<&[FieldRef]> {
        let &c = self.readings.get(&lens)?;
        self.inner.fields_at(c, schema_rev)
    }
}

/// The last `/` segment of a def's `object_class`: the concept name.
fn concept_name(def: &ActionDef) -> String {
    def.object_class
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

/// The concept id of a def's `object_class` in OGAR's canon codebook.
fn concept_of(def: &ActionDef) -> Result<(String, u16), ConvertRefusal> {
    let concept = concept_name(def);
    match canonical_concept_id(&concept) {
        Some(id) => Ok((concept, id)),
        None => Err(ConvertRefusal::UnmintedConcept { concept }),
    }
}

/// The position of the field labelled `name`, if the basis has one inside a
/// `WideFieldMask`'s `u8` range.
fn position_of(fields: &[FieldRef], name: &str) -> Option<u8> {
    fields
        .iter()
        .position(|f| f.label == name)
        .and_then(|p| u8::try_from(p).ok())
}

fn field_at(fields: &[FieldRef], name: &str, role: FieldRole) -> Result<u8, ConvertRefusal> {
    position_of(fields, name).ok_or_else(|| ConvertRefusal::UnknownField {
        role,
        name: name.to_string(),
    })
}

/// [`field_at`] for a guard field or a `Depends` path: a name that is a call
/// parameter of the def and not a field of the class is refused as
/// [`ConvertRefusal::CallParameterInGuard`], not as an unknown field.
fn guard_field_at(
    def: &ActionDef,
    fields: &[FieldRef],
    name: &str,
    role: FieldRole,
) -> Result<u8, ConvertRefusal> {
    match position_of(fields, name) {
        Some(p) => Ok(p),
        None if def.params.iter().any(|p| p.name == name) => {
            Err(ConvertRefusal::CallParameterInGuard {
                role,
                name: name.to_string(),
            })
        }
        None => field_at(fields, name, role),
    }
}

fn variant_name(k: &KausalSpec) -> &'static str {
    match k {
        KausalSpec::StateGuard { .. } => "StateGuard",
        KausalSpec::LifecycleTrigger { .. } => "LifecycleTrigger",
        KausalSpec::Depends { .. } => "Depends",
        KausalSpec::Constrains { .. } => "Constrains",
        KausalSpec::Onchange { .. } => "Onchange",
        KausalSpec::ContextDepends { .. } => "ContextDepends",
        KausalSpec::External => "External",
        _ => "unknown KausalSpec variant",
    }
}

/// `(ON, WHEN)` from a def's `kausal`. `None` is an unconditional method.
fn when_and_on(
    def: &ActionDef,
    fields: &[FieldRef],
) -> Result<(WideFieldMask, Option<Guard>), ConvertRefusal> {
    let kausal = def.kausal.as_ref();
    let unported = |why: &str| {
        let name = kausal.map_or("None", variant_name);
        ConvertRefusal::UnportedGuard {
            kausal: format!("{name}: {why}"),
        }
    };
    match kausal {
        None | Some(KausalSpec::External) => Ok((WideFieldMask::EMPTY, None)),
        Some(KausalSpec::Depends { paths }) => {
            let mut on = WideFieldMask::EMPTY;
            for path in paths {
                on = on.with(guard_field_at(def, fields, path, FieldRole::DependsPath)?);
            }
            Ok((on, None))
        }
        Some(KausalSpec::StateGuard {
            guard_field,
            guard_values,
        }) => {
            let lane = u16::from(guard_field_at(
                def,
                fields,
                guard_field,
                FieldRole::GuardField,
            )?);
            if guard_values.is_empty() {
                return Err(unported("an empty value set"));
            }
            let mut eqs = Vec::with_capacity(guard_values.len());
            for v in guard_values {
                let code: u32 = v
                    .parse()
                    .map_err(|_| unported(&format!("value `{v}` is not a decimal u32 code")))?;
                eqs.push(SetExpr::eq_u32(lane, code));
            }
            let expr = if eqs.len() == 1 {
                eqs.remove(0)
            } else {
                SetExpr::Or(eqs)
            };
            Ok((WideFieldMask::EMPTY, Some(Guard::new(expr, Logic::Kleene))))
        }
        Some(_) => Err(unported("not an equality over the class's fields")),
    }
}

/// Convert one [`ActionDef`] into the [`MethodBinding`] it states, identified
/// as `id`, over the field basis `view` gives its class. See the module doc
/// for the mapping; a def that cannot be stated is refused by name.
///
/// The classid is not needed to build the binding (it is the store's key), so
/// only the concept's fields are read, in `view`'s current basis, which is the
/// basis [`register_domain_table`] registers against.
///
/// # Errors
///
/// A [`ConvertRefusal`] naming the first thing that could not be carried.
pub fn method_of<V: ClassView + ?Sized>(
    def: &ActionDef,
    id: MethodId,
    view: &V,
) -> Result<MethodBinding, ConvertRefusal> {
    let (_, concept) = concept_of(def)?;
    method_over(def, id, view.fields(concept))
}

/// [`method_of`] over an explicit field basis: the reading the behaviour
/// uses ([`BehaviourView`] answers it for a behaviour classid).
fn method_over(
    def: &ActionDef,
    id: MethodId,
    fields: &[FieldRef],
) -> Result<MethodBinding, ConvertRefusal> {
    if def.on_enter.is_some() {
        return Err(ConvertRefusal::UnportedEffect { what: "on_enter" });
    }
    let (on, when) = when_and_on(def, fields)?;
    let mut reads = WideFieldMask::EMPTY;
    for name in &def.reads {
        reads = reads.with(field_at(fields, name, FieldRole::Reads)?);
    }
    let truth = match &when {
        Some(g) if g.logic == Logic::Kleene => Truth::ThreeValued,
        _ => Truth::TwoValued,
    };
    let demand = if when.is_some() || !reads.is_empty() {
        ValueDemand::Values
    } else {
        ValueDemand::MapOnly
    };
    Ok(MethodBinding {
        id,
        sig_rev: SigRev(1),
        name: def.predicate.clone(),
        on,
        when,
        reads,
        effects: Vec::new(),
        population: Population::Distinct,
        truth,
        demand,
    })
}

/// The extent a class of `fields` fields and `methods` methods needs: one DR64
/// identity tile, the narrowest property and predicate axes that fit.
fn extent_for(fields: usize, methods: usize) -> Extent {
    Extent {
        identity: Axis::new(Width::W64, 1),
        property: Axis::for_survivors(fields),
        predicate: Axis::for_survivors(methods),
    }
}

/// [`register_domain_table_for`] with the shared canonical concepts as the
/// left side ([`CanonicalClasses`]): the left half of each def is the canonical id of
/// the subject it declares (`erp_polyfill`, `patient`, ...). `app` is no
/// longer part of any classid (the right half is the behaviour arm) and is
/// ignored; the parameter is kept so existing callers compile. A behaviour
/// that is not shared registers under the consumer's own class through
/// [`register_domain_table_for`].
///
/// # Errors
///
/// As [`register_domain_table_for`].
pub fn register_domain_table<V: ClassView + ?Sized>(
    store: &mut BindStore,
    view: &V,
    table: &DomainTable,
    app: u16,
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    let _ = app;
    register_domain_table_for(store, view, &CanonicalClasses, table, schema_rev)
}

/// [`register_domain_table_with_arms`] over the canonical ledger alone: every
/// def's identity must be one OGAR has assigned an arm.
///
/// # Errors
///
/// As [`register_domain_table_with_arms`].
pub fn register_domain_table_for<V: ClassView + ?Sized, C: ConsumerClasses + ?Sized>(
    store: &mut BindStore,
    view: &V,
    classes: &C,
    table: &DomainTable,
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    register_domain_table_with_arms(
        store,
        view,
        classes,
        &ConsumerArms::new(),
        table,
        schema_rev,
    )
}

/// Convert every def of `table` and register each one at its own classid
/// `on :: arm` at `schema_rev`: `on` is `classes`' id for the def's declared
/// subject (the consumer's, resolved per def), `arm` is the def's identity
/// looked up in the canonical ledger and then in `arms`, the consumer's own
/// entries for behaviours OGAR does not know. Each def's field basis is read
/// from `view`, the consumer's [`ClassView`], through the behaviour lens
/// ([`BehaviourView`]). Every def is either registered or refused by name in the
/// returned [`TableOutcome`]; a def that does not convert never stops the
/// others.
///
/// # Errors
///
/// [`TableRefusal::Arm`] if the consumer's ledger is refused, a def's identity
/// has no ledger entry, or the table lists one identity twice (nothing is
/// registered), else [`TableRefusal::Dt64`] if DT64 refuses a behaviour's
/// registration (earlier ones stay registered; the store never rebinds, D5).
pub fn register_domain_table_with_arms<V: ClassView + ?Sized, C: ConsumerClasses + ?Sized>(
    store: &mut BindStore,
    view: &V,
    classes: &C,
    arms: &ConsumerArms,
    table: &DomainTable,
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    register_defs_with_arms(
        store,
        view,
        classes,
        arms,
        table.domain,
        &(table.actions)(),
        schema_rev,
    )
}

/// [`register_domain_table_with_arms`] for a set of defs the caller holds,
/// not a [`DomainTable`]. A `DomainTable` names its defs through a plain `fn`,
/// which cannot carry defs built at run time (a handler's registered
/// capabilities, for example); this takes them as a slice, with the `domain`
/// label the outcome carries (diagnostics only). Behaviour, refusals and
/// outcome are the same.
///
/// # Errors
///
/// As [`register_domain_table_with_arms`].
pub fn register_defs_with_arms<V: ClassView + ?Sized, C: ConsumerClasses + ?Sized>(
    store: &mut BindStore,
    view: &V,
    classes: &C,
    arms: &ConsumerArms,
    domain: &'static str,
    defs: &[ActionDef],
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    arms.validate()
        .map_err(|e| TableRefusal::Arm(BehaviourArmRefusal::Ledger(e)))?;
    let lefts: Vec<Result<u16, ConvertRefusal>> = defs
        .iter()
        .map(|d| {
            let concept = concept_name(d);
            classes
                .class_of(&concept)
                .ok_or(ConvertRefusal::ConsumerClassMissing { concept })
        })
        .collect();
    // One arm per def, whether or not the def later converts, so that refusing
    // or un-refusing one never decides another's fate.
    let mut ledger_arms: Vec<u16> = Vec::with_capacity(defs.len());
    for d in defs {
        match arms.arm_of(&d.identity) {
            Some(arm) => ledger_arms.push(arm),
            None => {
                return Err(TableRefusal::Arm(BehaviourArmRefusal::NoLedgerEntry {
                    identity: d.identity.clone(),
                }));
            }
        }
    }
    let mut listed: BTreeSet<(u16, &str)> = BTreeSet::new();
    for (def, left) in defs.iter().zip(&lefts) {
        let Ok(on) = *left else { continue };
        if !listed.insert((on, def.identity.as_str())) {
            return Err(TableRefusal::Arm(BehaviourArmRefusal::DuplicateDef {
                on,
                identity: def.identity.clone(),
            }));
        }
    }
    let arms = ledger_arms;
    let mut bview = BehaviourView::new(view);
    for (left, arm) in lefts.iter().zip(&arms) {
        if let Ok(on) = left {
            bview.declare(compose_classid(*on, *arm));
        }
    }
    let mut rows = Vec::with_capacity(defs.len());
    let mut to_register: Vec<(u32, ClassSchema, MethodBinding)> = Vec::new();
    for ((def, left), &arm) in defs.iter().zip(lefts).zip(&arms) {
        let id = MethodId(u32::from(arm));
        let result = left.and_then(|on| {
            let behaviour = compose_classid(on, arm);
            let fields = bview
                .lens_fields(behaviour)
                .expect("the behaviour was declared above");
            let binding = method_over(def, id, fields)?;
            let schema = ClassSchema {
                schema_rev,
                field_count: fields.len(),
                extent: extent_for(fields.len(), 1),
            };
            to_register.push((behaviour, schema, binding));
            Ok(Registered {
                id,
                class: compose_classid(on, 0),
                on,
                arm,
                behaviour,
                capability: def.predicate.clone(),
                identity: def.identity.clone(),
                params: def.params.clone(),
                results: def.results.clone(),
                effect: classify(def),
            })
        });
        rows.push(RowOutcome {
            id,
            predicate: def.predicate.clone(),
            result,
        });
    }
    for (behaviour, schema, binding) in to_register {
        store
            .register(&bview, behaviour, schema, vec![binding])
            .map_err(|refusal| TableRefusal::Dt64 {
                class: behaviour,
                refusal,
            })?;
    }
    Ok(TableOutcome { domain, rows })
}

/// [`register_domain_table`] only if OGAR's hot-plug resolution is green for
/// `consumer`: [`resolve_hotplug`] over the table's subject classids and the
/// `covered` capability names. The activation runs BEFORE DT64, as
/// `dt64-hotplug-probe::register_activated` does; on
/// [`TableRefusal::NotActivated`] the store is unchanged. `app` is ignored as
/// in [`register_domain_table`].
///
/// # Errors
///
/// [`TableRefusal::NotActivated`] with the [`HotplugDrift`], else as
/// [`register_domain_table`].
pub fn register_domain_table_activated<V: ClassView + ?Sized>(
    store: &mut BindStore,
    view: &V,
    table: &DomainTable,
    consumer: &str,
    covered: &[&str],
    app: u16,
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    let _ = app;
    register_domain_table_activated_for(
        store,
        view,
        &CanonicalClasses,
        table,
        consumer,
        covered,
        schema_rev,
    )
}

/// [`register_domain_table_for`] only if OGAR's hot-plug resolution is green
/// for `consumer` (the hot-plug consumer name, as in
/// [`register_domain_table_activated`]).
///
/// # Errors
///
/// [`TableRefusal::NotActivated`] with the [`HotplugDrift`], else as
/// [`register_domain_table_for`].
pub fn register_domain_table_activated_for<V: ClassView + ?Sized, C: ConsumerClasses + ?Sized>(
    store: &mut BindStore,
    view: &V,
    classes: &C,
    table: &DomainTable,
    consumer: &str,
    covered: &[&str],
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    register_domain_table_activated_with_arms(
        store,
        view,
        classes,
        &ConsumerArms::new(),
        table,
        consumer,
        covered,
        schema_rev,
    )
}

/// [`register_domain_table_with_arms`] only if OGAR's hot-plug resolution is
/// green for `consumer`.
///
/// # Errors
///
/// [`TableRefusal::NotActivated`] with the [`HotplugDrift`], else as
/// [`register_domain_table_with_arms`].
#[allow(clippy::too_many_arguments)]
pub fn register_domain_table_activated_with_arms<
    V: ClassView + ?Sized,
    C: ConsumerClasses + ?Sized,
>(
    store: &mut BindStore,
    view: &V,
    classes: &C,
    arms: &ConsumerArms,
    table: &DomainTable,
    consumer: &str,
    covered: &[&str],
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    let mut subjects: Vec<u16> = (table.entries)().into_iter().map(|(_, id)| id).collect();
    subjects.sort_unstable();
    subjects.dedup();
    resolve_hotplug(consumer, &subjects, covered).map_err(TableRefusal::NotActivated)?;
    register_domain_table_with_arms(store, view, classes, arms, table, schema_rev)
}
