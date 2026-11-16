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
//! | `ActionDef` | `MethodBinding` / store |
//! |---|---|
//! | `object_class` (last `/` segment, a minted concept) | the class: `render_classid(app, concept)`, one `BindStore::register` per class |
//! | `predicate` | `name`, carried and never identity |
//! | table position | `id`, see [Method ids](#method-ids) |
//! | `kausal: External` | `when: None` |
//! | `kausal: StateGuard` over a field of the class with decimal-literal values | `when: Some(Guard)`, an `Or` of `eq_u32` lanes under `Logic::Kleene` |
//! | `kausal: Depends` over fields of the class | `on` (the positions whose change triggers it) |
//! | `reads` | `reads`: each name must be a field label of the class |
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
//! ## Method ids
//!
//! A [`MethodId`] is the def's position in its table's declared order, counted
//! from 0, and a refused def keeps its number (so refusing or un-refusing a row
//! never renumbers the others). The ids are append-only by construction only if
//! the table is: **a reorder, an insertion in the middle or a removal renumbers
//! every later def.** `tests/dt64_methods.rs` pins today's ids so that is
//! visible.
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
//! ## Immutability
//!
//! DT64 never rebinds a class (D5). A class is registered once with the
//! methods that converted; a later run that converts more of its defs is
//! refused as `BehaviourImmutable` (a derived classid is the override path).
//!
//! # Not migrated yet
//!
//! The source-reading producers (`ogar-from-schema::do_arm::lift_action_defs`,
//! `ogar-from-ruff::lift_actions`, `ogar-from-rails::extract_actions`,
//! `ogar-from-elixir::extract_action_defs`) emit `ActionDef`s too and are not
//! routed through here. They are the next step: [`method_of`] already takes any
//! def, they need a table-shaped caller and a `ClassView` over the harvested
//! class.

use std::collections::BTreeMap;

use lance_graph_contract::class_view::{ClassView, WideFieldMask};
use lance_graph_contract::ontology::FieldRef;
use lance_graph_contract::set_algebra::{Logic, SetExpr};
use lance_graph_contract::set_rules::{Population, Truth};
use lance_graph_dt64::{
    Axis, BindStore, ClassSchema, Extent, Guard, MethodBinding, MethodId, Refusal, SchemaRev,
    SigRev, ValueDemand, Width,
};
use ogar_vocab::app::render_classid;
use ogar_vocab::capability_registry::{DomainTable, HotplugDrift, resolve_hotplug};
use ogar_vocab::{ActionDef, KausalSpec, canonical_concept_id};

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
    /// The method's identity in its class.
    pub id: MethodId,
    /// The classid it was registered under.
    pub class: u32,
    /// The capability name (`ActionDef::predicate`), the executor's key.
    pub capability: String,
    /// `ActionDef::identity`, which finds the def (its `writes`, `calls`,
    /// `raises`, statem carriers).
    pub identity: String,
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
}

/// The concept id of a def's `object_class` (`<prefix>/<concept>`, the concept
/// the last `/` segment, as `capability_registry` resolves it).
fn concept_of(def: &ActionDef) -> Result<(String, u16), ConvertRefusal> {
    let concept = def
        .object_class
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string();
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
    kausal: Option<&KausalSpec>,
    fields: &[FieldRef],
) -> Result<(WideFieldMask, Option<Guard>), ConvertRefusal> {
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
                on = on.with(field_at(fields, path, FieldRole::DependsPath)?);
            }
            Ok((on, None))
        }
        Some(KausalSpec::StateGuard {
            guard_field,
            guard_values,
        }) => {
            let lane = u16::from(field_at(fields, guard_field, FieldRole::GuardField)?);
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
    let fields = view.fields(concept);
    if def.on_enter.is_some() {
        return Err(ConvertRefusal::UnportedEffect { what: "on_enter" });
    }
    let (on, when) = when_and_on(def.kausal.as_ref(), fields)?;
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

/// Convert every def of `table` (ids are table positions) and register each
/// class once, with the methods that converted, under
/// `render_classid(app, concept)` at `schema_rev`. Every def is either
/// registered or refused by name in the returned [`TableOutcome`]; a def that
/// does not convert never stops the others.
///
/// A class none of whose defs converted is not registered at all (DT64 would
/// bind it with no behaviour, which reads as "no methods" rather than the
/// refusals it is).
///
/// # Errors
///
/// [`TableRefusal::Dt64`] if DT64 refuses a class's registration.
pub fn register_domain_table<V: ClassView + ?Sized>(
    store: &mut BindStore,
    view: &V,
    table: &DomainTable,
    app: u16,
    schema_rev: SchemaRev,
) -> Result<TableOutcome, TableRefusal> {
    let defs = (table.actions)();
    let mut rows = Vec::with_capacity(defs.len());
    // concept id -> the methods that converted, in table order.
    let mut by_class: BTreeMap<u16, Vec<MethodBinding>> = BTreeMap::new();
    for (i, def) in defs.iter().enumerate() {
        let id = MethodId(u32::try_from(i).expect("a table has fewer than 2^32 defs"));
        let result = match method_of(def, id, view) {
            Ok(binding) => {
                let (_, concept) = concept_of(def).expect("method_of resolved it");
                by_class.entry(concept).or_default().push(binding);
                Ok(Registered {
                    id,
                    class: render_classid(app, concept),
                    capability: def.predicate.clone(),
                    identity: def.identity.clone(),
                })
            }
            Err(refusal) => Err(refusal),
        };
        rows.push(RowOutcome {
            id,
            predicate: def.predicate.clone(),
            result,
        });
    }
    for (concept, methods) in by_class {
        let class = render_classid(app, concept);
        let fields = view.fields(concept).len();
        let schema = ClassSchema {
            schema_rev,
            field_count: fields,
            extent: extent_for(fields, methods.len()),
        };
        store
            .register(view, class, schema, methods)
            .map_err(|refusal| TableRefusal::Dt64 { class, refusal })?;
    }
    Ok(TableOutcome {
        domain: table.domain,
        rows,
    })
}

/// [`register_domain_table`] only if OGAR's hot-plug resolution is green for
/// `consumer`: [`resolve_hotplug`] over the table's subject classids and the
/// `covered` capability names. The activation runs BEFORE DT64, as
/// `dt64-hotplug-probe::register_activated` does; on
/// [`TableRefusal::NotActivated`] the store is unchanged.
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
    let mut subjects: Vec<u16> = (table.entries)().into_iter().map(|(_, id)| id).collect();
    subjects.sort_unstable();
    subjects.dedup();
    resolve_hotplug(consumer, &subjects, covered).map_err(TableRefusal::NotActivated)?;
    register_domain_table(store, view, table, app, schema_rev)
}
