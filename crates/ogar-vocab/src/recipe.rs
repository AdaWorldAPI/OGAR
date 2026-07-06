//! Recipe-concept codebook — the **VERB axis**, sibling to the
//! class-concept codebook (`crate::class_ids` / `crate::CODEBOOK`).
//!
//! # Why this exists
//!
//! A recipe is a canonicalized SPO triple, and the OGAR invocation grammar
//! `<port>::<path>(<shape>)` (`E-ONE-MASK-THREE-PORTS`,
//! `E-GRAMMAR-IS-THE-RECIPE-SHAPE`) is that triple:
//!
//! - `<path>` = `part_of::is_a` = the **subject**, a class facet → a
//!   [`crate::class_ids`] class id (the NOUN codebook — already shipped).
//! - `<shape>`'s verb = the **predicate**, a recipe → a
//!   [`RecipeConceptId`] (this VERB codebook).
//! - `<shape>`'s `input[type]` = the **object**, typed from the schema /
//!   association stratum.
//!
//! Producer frontends (`ruff_ruby_spo`, `ruff_python_spo`) emit the
//! predicate as a **surface string** (`"before_save"`, `"belongs_to"`,
//! `"presence"`, `"list_for_tenant"`) — that string is the per-consumer
//! *zoo*. [`recipe_concept_from_surface`] is the **lift-time resolver**
//! that maps a surface string to the shared canonical id, keeping the
//! string as the per-language label skin. Rails `belongs_to` and Odoo
//! `Many2one` resolve to the **same** [`recipe_ids::REL_MANY_TO_ONE`] —
//! the verb-side twin of `WorkPackage ≡ Issue ≡ 0x0102`.
//!
//! Per RAILS-COVERAGE-KIT §5 the recipe vocabulary MUST converge the same
//! way class concepts do, or the behavioural arm fragments back into the
//! zoo the structural arm escaped.
//!
//! # Id layout — `RESERVE-DON'T-RECLAIM`
//!
//! A recipe id is a `u16` whose **high byte is the family** (mirroring the
//! class codebook's domain high-byte) and whose low byte enumerates the
//! concept within the family:
//!
//! | family byte | [`RecipeFamily`] | surfaces (Rails ‖ Odoo) |
//! |---|---|---|
//! | `0x01` | [`RecipeFamily::Lifecycle`] | `before_save` / `after_commit` ‖ `@api.constrains` / `@depends` |
//! | `0x02` | [`RecipeFamily::Guard`] | `validates presence:/uniqueness:` ‖ `_check_*` |
//! | `0x03` | [`RecipeFamily::Relation`] | `belongs_to`/`has_many` ‖ `Many2one`/`One2many` |
//! | `0x04` | [`RecipeFamily::Action`] | controller `HandlerKind` ‖ `action_*` |
//!
//! Ids are stable forever: they only ARRIVE (append a new low byte / a new
//! family), never move, never get repurposed. Enforced by the drift-gate
//! tests below, exactly like `class_ids`.
//!
//! # Type safety over the class codebook
//!
//! Unlike `class_ids` (bare `u16`), recipe ids are wrapped in
//! [`RecipeConceptId`]. A recipe id `0x0101` (`LIFECYCLE_BEFORE_VALIDATION`)
//! and a class id `0x0101` (`project`) are numerically equal but live in
//! different address spaces; the newtype makes handing one where the other
//! is expected a **compile error**, per the noun-vs-verb-port guardrail
//! (`E-GRAMMAR-IS-THE-RECIPE-SHAPE`).

/// A canonical **recipe-concept id** — the predicate leg of the
/// `<port>::<path>(<shape>)` grammar. Wraps the stable `u16` codebook id;
/// the high byte is the [`RecipeFamily`].
///
/// ```
/// use ogar_vocab::recipe::{recipe_ids, RecipeFamily};
/// let id = recipe_ids::REL_MANY_TO_ONE;
/// assert_eq!(id.family(), RecipeFamily::Relation);
/// assert_eq!(id.as_u16(), 0x0301);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RecipeConceptId(pub u16);

impl RecipeConceptId {
    /// The raw stable codebook `u16`.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// The 2 little-endian wire bytes — same convention as
    /// [`crate::canonical_concept_id`]'s `u16::to_le_bytes` contract, so a
    /// recipe id serialises identically to a class id (both are `u16` LE).
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; 2] {
        self.0.to_le_bytes()
    }

    /// The [`RecipeFamily`] this id belongs to, from its high byte. Pure,
    /// O(1), no table lookup — mirrors [`crate::canonical_concept_domain`].
    #[must_use]
    pub const fn family(self) -> RecipeFamily {
        match self.0 >> 8 {
            0x01 => RecipeFamily::Lifecycle,
            0x02 => RecipeFamily::Guard,
            0x03 => RecipeFamily::Relation,
            0x04 => RecipeFamily::Action,
            _ => RecipeFamily::Unassigned,
        }
    }
}

/// The recipe families (RAILS-COVERAGE-KIT §5). The high-byte tag of a
/// [`RecipeConceptId`] — the VERB-axis counterpart of
/// [`crate::ConceptDomain`]. `#[non_exhaustive]`: new families append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RecipeFamily {
    /// `0x01XX` — lifecycle hooks (`before_save` / `after_commit` …).
    Lifecycle,
    /// `0x02XX` — validation guards (`presence` / `uniqueness` …).
    Guard,
    /// `0x03XX` — associations (`belongs_to` / `has_many` …).
    Relation,
    /// `0x04XX` — controller actions (the `HandlerKind` surface).
    Action,
    /// Any high byte not yet assigned a family (`0x00XX`, `0x05XX`+).
    Unassigned,
}

/// Per-language surface dialect a recipe predicate was harvested from. The
/// `lang` axis of the label DTO (`RAILS-COVERAGE-KIT` §5: "one ontology,
/// N label DTOs"). `#[non_exhaustive]`: new frontends append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum RecipeLang {
    /// Ruby / Rails ActiveRecord DSL (`ruff_ruby_spo`).
    Ruby,
    /// Python / Odoo ORM DSL (`ruff_python_spo`).
    Python,
}

/// **Compile-time recipe-id constants** — every promoted recipe concept as
/// a named [`RecipeConceptId`], so consumers dispatch on canonical verb
/// identity without a [`recipe_concept_id`] lookup. Canonical home; drift
/// against [`RECIPE_CODEBOOK`] is caught by [`tests::constants_match_codebook`].
///
/// `RESERVE-DON'T-RECLAIM`: append only; never reorder, never repurpose.
pub mod recipe_ids {
    use super::RecipeConceptId;

    // ── 0x01XX — Lifecycle hooks ──
    /// `lifecycle_before_validation` — Rails `before_validation`.
    pub const LIFECYCLE_BEFORE_VALIDATION: RecipeConceptId = RecipeConceptId(0x0101);
    /// `lifecycle_before_persist` — Rails `before_save`; Odoo `@api.constrains`
    /// (a pre-persist guard body).
    pub const LIFECYCLE_BEFORE_PERSIST: RecipeConceptId = RecipeConceptId(0x0102);
    /// `lifecycle_after_create` — Rails `after_create`.
    pub const LIFECYCLE_AFTER_CREATE: RecipeConceptId = RecipeConceptId(0x0103);
    /// `lifecycle_after_update` — Rails `after_update`.
    pub const LIFECYCLE_AFTER_UPDATE: RecipeConceptId = RecipeConceptId(0x0104);
    /// `lifecycle_after_save` — Rails `after_save`.
    pub const LIFECYCLE_AFTER_SAVE: RecipeConceptId = RecipeConceptId(0x0105);
    /// `lifecycle_after_destroy` — Rails `after_destroy`.
    pub const LIFECYCLE_AFTER_DESTROY: RecipeConceptId = RecipeConceptId(0x0106);
    /// `lifecycle_after_commit` — Rails `after_commit` / `after_create_commit`;
    /// Odoo `_compute_*` `@depends` (on-commit recompute).
    pub const LIFECYCLE_AFTER_COMMIT: RecipeConceptId = RecipeConceptId(0x0107);

    // ── 0x02XX — Validation guards ──
    /// `guard_presence` — Rails `validates …, presence: true`.
    pub const GUARD_PRESENCE: RecipeConceptId = RecipeConceptId(0x0201);
    /// `guard_uniqueness` — Rails `validates …, uniqueness: true`.
    pub const GUARD_UNIQUENESS: RecipeConceptId = RecipeConceptId(0x0202);
    /// `guard_length` — Rails `validates …, length: {…}`.
    pub const GUARD_LENGTH: RecipeConceptId = RecipeConceptId(0x0203);
    /// `guard_format` — Rails `validates …, format: {…}`.
    pub const GUARD_FORMAT: RecipeConceptId = RecipeConceptId(0x0204);
    /// `guard_inclusion` — Rails `validates …, inclusion: {…}`.
    pub const GUARD_INCLUSION: RecipeConceptId = RecipeConceptId(0x0205);
    /// `guard_numericality` — Rails `validates …, numericality: {…}`.
    pub const GUARD_NUMERICALITY: RecipeConceptId = RecipeConceptId(0x0206);
    /// `guard_associated` — Rails `validates_associated`.
    pub const GUARD_ASSOCIATED: RecipeConceptId = RecipeConceptId(0x0207);

    // ── 0x03XX — Associations (Relation kind) ──
    /// `rel_many_to_one` — Rails `belongs_to`; Odoo `Many2one`.
    pub const REL_MANY_TO_ONE: RecipeConceptId = RecipeConceptId(0x0301);
    /// `rel_one_to_many` — Rails `has_many`; Odoo `One2many`.
    pub const REL_ONE_TO_MANY: RecipeConceptId = RecipeConceptId(0x0302);
    /// `rel_many_to_many` — Rails `has_and_belongs_to_many`; Odoo `Many2many`.
    pub const REL_MANY_TO_MANY: RecipeConceptId = RecipeConceptId(0x0303);
    /// `rel_one_to_one` — Rails `has_one`.
    pub const REL_ONE_TO_ONE: RecipeConceptId = RecipeConceptId(0x0304);
    /// `rel_through` — Rails `has_many …, through:` (the join-model form).
    pub const REL_THROUGH: RecipeConceptId = RecipeConceptId(0x0305);

    // ── 0x04XX — Controller actions (HandlerKind) ──
    /// `action_list_for_tenant` — the paged tenant-scoped index handler.
    pub const ACTION_LIST_FOR_TENANT: RecipeConceptId = RecipeConceptId(0x0401);
    /// `action_detail_for_tenant` — the tenant-scoped show handler.
    pub const ACTION_DETAIL_FOR_TENANT: RecipeConceptId = RecipeConceptId(0x0402);
    /// `action_soft_delete` — the `active = false` soft-delete handler.
    pub const ACTION_SOFT_DELETE: RecipeConceptId = RecipeConceptId(0x0403);
    /// `action_toggle_bool` — the toggle-a-boolean-field handler.
    pub const ACTION_TOGGLE_BOOL: RecipeConceptId = RecipeConceptId(0x0404);
    /// `action_ajax_json` — the JSON-blob GET handler.
    pub const ACTION_AJAX_JSON: RecipeConceptId = RecipeConceptId(0x0405);
    /// `action_csrf_form_post` — the POST-create form handler.
    pub const ACTION_CSRF_FORM_POST: RecipeConceptId = RecipeConceptId(0x0406);
    /// `action_create` — the generic AR create lifecycle.
    pub const ACTION_CREATE: RecipeConceptId = RecipeConceptId(0x0407);
    /// `action_update` — the generic AR update lifecycle (the `MySQL{shape:update}`
    /// storage verb resolves here).
    pub const ACTION_UPDATE: RecipeConceptId = RecipeConceptId(0x0408);
    /// `action_destroy` — the generic AR destroy lifecycle.
    pub const ACTION_DESTROY: RecipeConceptId = RecipeConceptId(0x0409);
}

/// The recipe-concept registry — `(canonical_name, id)`, sibling of
/// [`crate::class_ids::ALL`]. Codebook order; append only.
pub const RECIPE_CODEBOOK: &[(&str, RecipeConceptId)] = &[
    // 0x01XX — Lifecycle
    (
        "lifecycle_before_validation",
        recipe_ids::LIFECYCLE_BEFORE_VALIDATION,
    ),
    (
        "lifecycle_before_persist",
        recipe_ids::LIFECYCLE_BEFORE_PERSIST,
    ),
    ("lifecycle_after_create", recipe_ids::LIFECYCLE_AFTER_CREATE),
    ("lifecycle_after_update", recipe_ids::LIFECYCLE_AFTER_UPDATE),
    ("lifecycle_after_save", recipe_ids::LIFECYCLE_AFTER_SAVE),
    (
        "lifecycle_after_destroy",
        recipe_ids::LIFECYCLE_AFTER_DESTROY,
    ),
    ("lifecycle_after_commit", recipe_ids::LIFECYCLE_AFTER_COMMIT),
    // 0x02XX — Guard
    ("guard_presence", recipe_ids::GUARD_PRESENCE),
    ("guard_uniqueness", recipe_ids::GUARD_UNIQUENESS),
    ("guard_length", recipe_ids::GUARD_LENGTH),
    ("guard_format", recipe_ids::GUARD_FORMAT),
    ("guard_inclusion", recipe_ids::GUARD_INCLUSION),
    ("guard_numericality", recipe_ids::GUARD_NUMERICALITY),
    ("guard_associated", recipe_ids::GUARD_ASSOCIATED),
    // 0x03XX — Relation
    ("rel_many_to_one", recipe_ids::REL_MANY_TO_ONE),
    ("rel_one_to_many", recipe_ids::REL_ONE_TO_MANY),
    ("rel_many_to_many", recipe_ids::REL_MANY_TO_MANY),
    ("rel_one_to_one", recipe_ids::REL_ONE_TO_ONE),
    ("rel_through", recipe_ids::REL_THROUGH),
    // 0x04XX — Action
    ("action_list_for_tenant", recipe_ids::ACTION_LIST_FOR_TENANT),
    (
        "action_detail_for_tenant",
        recipe_ids::ACTION_DETAIL_FOR_TENANT,
    ),
    ("action_soft_delete", recipe_ids::ACTION_SOFT_DELETE),
    ("action_toggle_bool", recipe_ids::ACTION_TOGGLE_BOOL),
    ("action_ajax_json", recipe_ids::ACTION_AJAX_JSON),
    ("action_csrf_form_post", recipe_ids::ACTION_CSRF_FORM_POST),
    ("action_create", recipe_ids::ACTION_CREATE),
    ("action_update", recipe_ids::ACTION_UPDATE),
    ("action_destroy", recipe_ids::ACTION_DESTROY),
];

/// Per-language **label DTO** table — `(id, lang, surface)`. The surface
/// string is render-only and per-language; the id is the truth. Multiple
/// surfaces (across languages) map to one id — that IS the cross-consumer
/// convergence (Rails `belongs_to` ≡ Odoo `Many2one` → `rel_many_to_one`).
///
/// Append only; a new surface for an existing concept adds a row, mints
/// nothing (`RAILS-COVERAGE-KIT` §5 rule 2). Fuzzy Odoo body-pattern
/// surfaces (`_check_*`, `_compute_*`) are intentionally NOT seeded here —
/// they need the F17 body triage, not a lexical alias.
const RECIPE_LABELS: &[(RecipeConceptId, RecipeLang, &str)] = &[
    // Lifecycle — Ruby callback phases (the token before the ':' in a
    // `has_callback "<phase>:<target>"` triple).
    (
        recipe_ids::LIFECYCLE_BEFORE_VALIDATION,
        RecipeLang::Ruby,
        "before_validation",
    ),
    (
        recipe_ids::LIFECYCLE_BEFORE_PERSIST,
        RecipeLang::Ruby,
        "before_save",
    ),
    (
        recipe_ids::LIFECYCLE_AFTER_CREATE,
        RecipeLang::Ruby,
        "after_create",
    ),
    (
        recipe_ids::LIFECYCLE_AFTER_UPDATE,
        RecipeLang::Ruby,
        "after_update",
    ),
    (
        recipe_ids::LIFECYCLE_AFTER_SAVE,
        RecipeLang::Ruby,
        "after_save",
    ),
    (
        recipe_ids::LIFECYCLE_AFTER_DESTROY,
        RecipeLang::Ruby,
        "after_destroy",
    ),
    (
        recipe_ids::LIFECYCLE_AFTER_COMMIT,
        RecipeLang::Ruby,
        "after_commit",
    ),
    (
        recipe_ids::LIFECYCLE_AFTER_COMMIT,
        RecipeLang::Ruby,
        "after_create_commit",
    ),
    // Guard — Ruby validation kinds.
    (recipe_ids::GUARD_PRESENCE, RecipeLang::Ruby, "presence"),
    (recipe_ids::GUARD_UNIQUENESS, RecipeLang::Ruby, "uniqueness"),
    (recipe_ids::GUARD_LENGTH, RecipeLang::Ruby, "length"),
    (recipe_ids::GUARD_FORMAT, RecipeLang::Ruby, "format"),
    (recipe_ids::GUARD_INCLUSION, RecipeLang::Ruby, "inclusion"),
    (
        recipe_ids::GUARD_NUMERICALITY,
        RecipeLang::Ruby,
        "numericality",
    ),
    (recipe_ids::GUARD_ASSOCIATED, RecipeLang::Ruby, "associated"),
    // Relation — Ruby association kinds.
    (recipe_ids::REL_MANY_TO_ONE, RecipeLang::Ruby, "belongs_to"),
    (recipe_ids::REL_ONE_TO_MANY, RecipeLang::Ruby, "has_many"),
    (
        recipe_ids::REL_MANY_TO_MANY,
        RecipeLang::Ruby,
        "has_and_belongs_to_many",
    ),
    (recipe_ids::REL_ONE_TO_ONE, RecipeLang::Ruby, "has_one"),
    (recipe_ids::REL_THROUGH, RecipeLang::Ruby, "through"),
    // Relation — Python/Odoo field kinds (the convergence pins).
    (recipe_ids::REL_MANY_TO_ONE, RecipeLang::Python, "Many2one"),
    (recipe_ids::REL_ONE_TO_MANY, RecipeLang::Python, "One2many"),
    (
        recipe_ids::REL_MANY_TO_MANY,
        RecipeLang::Python,
        "Many2many",
    ),
    // Action — controller HandlerKind surfaces (framework-neutral ids;
    // tagged Ruby as the harvest frontend that produces them today).
    (
        recipe_ids::ACTION_LIST_FOR_TENANT,
        RecipeLang::Ruby,
        "list_for_tenant",
    ),
    (
        recipe_ids::ACTION_DETAIL_FOR_TENANT,
        RecipeLang::Ruby,
        "detail_for_tenant",
    ),
    (
        recipe_ids::ACTION_SOFT_DELETE,
        RecipeLang::Ruby,
        "soft_delete",
    ),
    (
        recipe_ids::ACTION_TOGGLE_BOOL,
        RecipeLang::Ruby,
        "toggle_bool_field",
    ),
    (recipe_ids::ACTION_AJAX_JSON, RecipeLang::Ruby, "ajax_json"),
    (
        recipe_ids::ACTION_CSRF_FORM_POST,
        RecipeLang::Ruby,
        "csrf_form_post_engine_call",
    ),
    (recipe_ids::ACTION_CREATE, RecipeLang::Ruby, "create"),
    (recipe_ids::ACTION_UPDATE, RecipeLang::Ruby, "update"),
    (recipe_ids::ACTION_DESTROY, RecipeLang::Ruby, "destroy"),
];

/// **Recipe codebook lookup** — resolve a canonical recipe name to its
/// stable [`RecipeConceptId`]. Sibling of [`crate::canonical_concept_id`].
/// `None` for unpromoted names.
///
/// ```
/// use ogar_vocab::recipe::{recipe_concept_id, recipe_ids};
/// assert_eq!(recipe_concept_id("rel_many_to_one"), Some(recipe_ids::REL_MANY_TO_ONE));
/// assert_eq!(recipe_concept_id("not_a_recipe"), None);
/// ```
#[must_use]
pub fn recipe_concept_id(name: &str) -> Option<RecipeConceptId> {
    RECIPE_CODEBOOK
        .iter()
        .find_map(|(n, id)| if *n == name { Some(*id) } else { None })
}

/// Inverse of [`recipe_concept_id`]: the canonical recipe name for an id,
/// or `None`. Round-trips (ids are unique in [`RECIPE_CODEBOOK`]). Sibling
/// of [`crate::canonical_concept_name`].
#[must_use]
pub fn recipe_concept_name(id: RecipeConceptId) -> Option<&'static str> {
    RECIPE_CODEBOOK
        .iter()
        .find_map(|(n, cid)| if *cid == id { Some(*n) } else { None })
}

/// **The lift-time predicate resolver** — map a producer-emitted surface
/// string (in dialect `lang`) to its canonical [`RecipeConceptId`]. This is
/// the seam `E-GRAMMAR-IS-THE-RECIPE-SHAPE` names: `Triple.p: String →
/// RecipeConceptId`, keeping the string as the label skin. `None` when no
/// promoted concept claims the surface (append a [`RECIPE_LABELS`] row).
///
/// ```
/// use ogar_vocab::recipe::{recipe_concept_from_surface, recipe_ids, RecipeLang};
/// // The verb-side convergence pin: Rails and Odoo relation surfaces
/// // resolve to ONE canonical concept.
/// assert_eq!(
///     recipe_concept_from_surface("belongs_to", RecipeLang::Ruby),
///     recipe_concept_from_surface("Many2one", RecipeLang::Python),
/// );
/// assert_eq!(
///     recipe_concept_from_surface("belongs_to", RecipeLang::Ruby),
///     Some(recipe_ids::REL_MANY_TO_ONE),
/// );
/// ```
#[must_use]
pub fn recipe_concept_from_surface(surface: &str, lang: RecipeLang) -> Option<RecipeConceptId> {
    RECIPE_LABELS.iter().find_map(|(id, l, s)| {
        if *l == lang && *s == surface {
            Some(*id)
        } else {
            None
        }
    })
}

/// Every promoted `(name, id)` whose id lives in `family`, in codebook
/// order. Sibling of [`crate::concepts_in_domain`] — the enumeration hook a
/// family-scoped consumer uses to inherit its full verb set.
pub fn recipes_in_family(
    family: RecipeFamily,
) -> impl Iterator<Item = (&'static str, RecipeConceptId)> {
    RECIPE_CODEBOOK
        .iter()
        .copied()
        .filter(move |&(_, id)| id.family() == family)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_codebook() {
        // Forward drift gate (mirrors class_ids::tests::constants_match_codebook):
        // every registry name resolves to its declared id.
        for (name, id) in RECIPE_CODEBOOK {
            assert_eq!(
                recipe_concept_id(name),
                Some(*id),
                "{name}: 0x{:04X} disagrees with codebook lookup",
                id.as_u16()
            );
        }
    }

    #[test]
    fn recipe_ids_are_unique() {
        // RESERVE-DON'T-RECLAIM: no two names share an id (would break the
        // round-trip and silently alias two verbs).
        let mut seen = std::collections::HashSet::new();
        for (name, id) in RECIPE_CODEBOOK {
            assert!(
                seen.insert(id.as_u16()),
                "duplicate recipe id 0x{:04X} at `{name}`",
                id.as_u16()
            );
        }
    }

    #[test]
    fn recipe_concept_name_round_trips() {
        for (name, id) in RECIPE_CODEBOOK {
            assert_eq!(recipe_concept_name(*id), Some(*name));
            assert_eq!(
                recipe_concept_id(recipe_concept_name(*id).unwrap()),
                Some(*id)
            );
        }
    }

    #[test]
    fn families_resolve_from_high_byte() {
        assert_eq!(
            recipe_ids::LIFECYCLE_BEFORE_PERSIST.family(),
            RecipeFamily::Lifecycle
        );
        assert_eq!(recipe_ids::GUARD_PRESENCE.family(), RecipeFamily::Guard);
        assert_eq!(recipe_ids::REL_MANY_TO_ONE.family(), RecipeFamily::Relation);
        assert_eq!(recipe_ids::ACTION_UPDATE.family(), RecipeFamily::Action);
        // Every promoted id has a known (non-Unassigned) family.
        for (name, id) in RECIPE_CODEBOOK {
            assert_ne!(
                id.family(),
                RecipeFamily::Unassigned,
                "`{name}` (0x{:04X}) resolves to an unassigned family",
                id.as_u16()
            );
        }
    }

    #[test]
    fn recipes_in_family_partitions_the_codebook() {
        let total: usize = [
            RecipeFamily::Lifecycle,
            RecipeFamily::Guard,
            RecipeFamily::Relation,
            RecipeFamily::Action,
        ]
        .into_iter()
        .map(|f| recipes_in_family(f).count())
        .sum();
        assert_eq!(
            total,
            RECIPE_CODEBOOK.len(),
            "families do not partition the codebook"
        );
        assert_eq!(recipes_in_family(RecipeFamily::Relation).count(), 5);
    }

    #[test]
    fn surface_resolver_maps_ruby_dialect() {
        assert_eq!(
            recipe_concept_from_surface("before_save", RecipeLang::Ruby),
            Some(recipe_ids::LIFECYCLE_BEFORE_PERSIST)
        );
        assert_eq!(
            recipe_concept_from_surface("presence", RecipeLang::Ruby),
            Some(recipe_ids::GUARD_PRESENCE)
        );
        assert_eq!(
            recipe_concept_from_surface("belongs_to", RecipeLang::Ruby),
            Some(recipe_ids::REL_MANY_TO_ONE)
        );
        assert_eq!(
            recipe_concept_from_surface("soft_delete", RecipeLang::Ruby),
            Some(recipe_ids::ACTION_SOFT_DELETE)
        );
        // after_commit has two Ruby surfaces → same id.
        assert_eq!(
            recipe_concept_from_surface("after_create_commit", RecipeLang::Ruby),
            Some(recipe_ids::LIFECYCLE_AFTER_COMMIT)
        );
    }

    /// The verb-side convergence pin — the twin of
    /// `WorkPackage ≡ Issue ≡ 0x0102` on the noun side. Rails and Odoo
    /// relation surfaces resolve to ONE canonical recipe concept.
    #[test]
    fn relation_verbs_converge_across_ruby_and_python() {
        let pairs = [
            ("belongs_to", "Many2one", recipe_ids::REL_MANY_TO_ONE),
            ("has_many", "One2many", recipe_ids::REL_ONE_TO_MANY),
            (
                "has_and_belongs_to_many",
                "Many2many",
                recipe_ids::REL_MANY_TO_MANY,
            ),
        ];
        for (ruby, python, expected) in pairs {
            let r = recipe_concept_from_surface(ruby, RecipeLang::Ruby);
            let p = recipe_concept_from_surface(python, RecipeLang::Python);
            assert_eq!(
                r,
                Some(expected),
                "Ruby `{ruby}` must resolve to {expected:?}"
            );
            assert_eq!(
                p,
                Some(expected),
                "Python `{python}` must resolve to {expected:?}"
            );
            assert_eq!(
                r, p,
                "convergence broken: Ruby `{ruby}` ↔ Python `{python}`"
            );
        }
    }

    #[test]
    fn unknown_surface_resolves_to_none() {
        assert_eq!(
            recipe_concept_from_surface("not_a_verb", RecipeLang::Ruby),
            None
        );
        // Right surface, wrong dialect: `belongs_to` is Ruby-only.
        assert_eq!(
            recipe_concept_from_surface("belongs_to", RecipeLang::Python),
            None
        );
        assert_eq!(recipe_concept_from_surface("", RecipeLang::Ruby), None);
    }

    #[test]
    fn every_label_points_at_a_promoted_id() {
        // A label row for an id not in the codebook would be an orphan
        // surface (resolves to a name-less id). Guard against it.
        for (id, _lang, surface) in RECIPE_LABELS {
            assert!(
                recipe_concept_name(*id).is_some(),
                "label `{surface}` points at unpromoted id 0x{:04X}",
                id.as_u16()
            );
        }
    }

    #[test]
    fn le_bytes_match_u16_convention() {
        // Wire-compatible with the class codebook: 2 LE bytes.
        assert_eq!(
            recipe_ids::REL_MANY_TO_ONE.to_le_bytes(),
            0x0301u16.to_le_bytes()
        );
    }
}
