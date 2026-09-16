//! The 34 NARS reasoning recipes as a loco vocabulary — and the column that
//! says **which calls are masking ops**.
//!
//! # The mapping was already data
//!
//! Each [`Recipe`] in `lance_graph_contract::recipes::RECIPES` carries a
//! `bucket`, and the three buckets ARE the three execution tiers. This is not
//! an assignment invented here; it is a column that has been sitting in the
//! catalogue:
//!
//! | `Bucket` | the catalogue's own words | who executes it |
//! |---|---|---|
//! | `Datapath` | *"uniform, branch-free, every-cycle SIMD"* | **the masking ops** |
//! | `Control`  | *"branchy decision at a control point"* | **loco's interpreter** |
//! | `Gate`     | *"a cheap marker that gates whether deeper work fires"* | a predicate, before either |
//!
//! Measured over the real catalogue: **9 Datapath, 19 Control, 6 Gate**. The
//! Datapath nine are literally mask/VSA primitives — #25 `HPM` is *"the
//! substrate: fingerprint cosine/Hamming sweep (SIMD)"*, #19 `ARE` is
//! `A⊗B⊗B=A`, #27 `MPC` is *"bundle = majority-vote-per-bit"*. For those,
//! **a `(function : value)` call IS a masking op**, and [`tier_of`] is how a
//! consumer asks.
//!
//! # Why depending on the contract costs nothing
//!
//! `lance-graph-contract` is itself zero-dep — "a trait-only crate" that "MUST
//! stay dependency-free even of optional path deps". EIGHT OGAR crates already
//! pull it (`ogar-auth`, `ogar-class-view`, `ogar-doc-ir`, `ogar-from-ruff`,
//! `ogar-r2il`, `ogar-rbac`, `ogar-render-askama`, `ogar-render-typst` —
//! counted, not recalled; an earlier draft of this paragraph said four),
//! and `ogar-from-ruff`'s manifest says so in as many words: *"Pulls the
//! ZERO-DEP `lance-graph-contract` only."* This crate's "Zero-dep" description
//! was never an argument against carrying the catalogue.
//!
//! # Byte allocation
//!
//! The 34 occupy `0x90..=0xB1` — `DOMAIN_FLOOR` plus 34. That they overlap
//! `ogar-r2il`'s range is not a collision: a vocabulary is selected by the
//! node's **classid**, so every dialect starts its own bytes at the floor.
//! That is what the registry is for.
//!
//! # What is DERIVED and what is POLICY
//!
//! Derived (read from the catalogue, never chosen here): the code, the name,
//! the bucket, the substrate string, the id→byte mapping.
//!
//! **Policy (chosen here, and re-pinnable):** the stack arities below. The
//! catalogue records what realizes a tactic, not how many operands its call
//! pops, so the arities are a stated default per bucket rather than a
//! measurement — say so rather than let a future session inherit them as
//! fact. Each is the narrowest shape its bucket implies:
//!
//! - `Datapath` → **2**: the VSA/mask primitives here are binary at their
//!   core (`bind(A,B)`, bundle-of-two, a cosine sweep of query against
//!   corpus). A ternary form like `ARE`'s `A⊗B⊗B` composes two binds.
//! - `Gate` → **1**: a marker reads one thing and answers.
//! - `Control` → **1**: an orchestration call takes the subject it
//!   orchestrates.
//!
//! # The seam this does NOT close
//!
//! `body_refs` is **0 for every recipe**, including Control. A Control recipe
//! genuinely orchestrates a body, so it wants a body reference — but the
//! engine's control band is `0x01..=0x1F` and these bytes sit above
//! `DOMAIN_FLOOR`, so a body reference here would branch through a path
//! nothing has executed. Declaring `0` keeps the catalogue honest about what
//! runs today; the orchestration seam lands with its own falsifier, not as an
//! unexercised field.

use crate::{DOMAIN_FLOOR, FnIndex, Vocabulary};
use lance_graph_contract::recipes::{Bucket, RECIPES, Recipe};

/// First byte this vocabulary owns — [`DOMAIN_FLOOR`].
pub const NARS_BASE: u8 = DOMAIN_FLOOR;

/// How many bytes it owns: one per recipe.
pub const NARS_COUNT: u8 = RECIPES.len() as u8;

/// Last byte this vocabulary owns, inclusive.
pub const NARS_LAST: u8 = NARS_BASE + NARS_COUNT - 1;

const _: () = assert!(
    NARS_LAST < 0xFF,
    "the 34 recipes must fit above the domain floor"
);

/// The recipe a byte names, or `None` outside `NARS_BASE..=NARS_LAST`.
///
/// Recipe ids are `1..=34` and the catalogue is id-ascending, so the byte is
/// `NARS_BASE + (id - 1)`. Resolved by INDEX rather than by searching for a
/// matching `id`, and the debug assert below is what keeps those two readings
/// from drifting if the catalogue's order ever stops matching its ids.
#[must_use]
pub fn recipe_at(f: FnIndex) -> Option<&'static Recipe> {
    let i = f.0.checked_sub(NARS_BASE)? as usize;
    let r = RECIPES.get(i)?;
    debug_assert_eq!(
        r.id as usize,
        i + 1,
        "RECIPES is documented as id-ascending; byte↔id resolution relies on it"
    );
    Some(r)
}

/// Which tier executes this byte — the question "is this call a masking op?"
///
/// `Some(Bucket::Datapath)` is a yes.
#[must_use]
pub fn tier_of(f: FnIndex) -> Option<Bucket> {
    recipe_at(f).map(|r| r.bucket)
}

/// Is this call a masking op — i.e. does the Datapath tier execute it?
#[must_use]
pub fn is_mask_op(f: FnIndex) -> bool {
    matches!(tier_of(f), Some(Bucket::Datapath))
}

/// The byte a recipe id (`1..=34`) is addressed by, or `None` out of range.
#[must_use]
pub fn byte_of_id(id: u8) -> Option<FnIndex> {
    (1..=NARS_COUNT)
        .contains(&id)
        .then(|| FnIndex(NARS_BASE + id - 1))
}

/// The 34 recipes as a plug-in vocabulary.
///
/// Below [`DOMAIN_FLOOR`] it is transparent — the shared core answers, as the
/// conformance rule requires. Above it, every byte's shape comes from its
/// recipe's bucket.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NarsVocabulary;

impl Vocabulary for NarsVocabulary {
    fn domain_stack_arity(&self, f: FnIndex) -> Option<u8> {
        // POLICY, not a measurement — see the module doc.
        tier_of(f).map(|b| match b {
            Bucket::Datapath => 2,
            Bucket::Gate | Bucket::Control => 1,
        })
    }

    fn domain_body_refs(&self, _f: FnIndex) -> u8 {
        // Zero for every recipe, Control included. See "the seam this does
        // NOT close" in the module doc: a body reference here would branch
        // through a path nothing has executed.
        0
    }

    fn domain_pushes_result(&self, f: FnIndex) -> Option<bool> {
        // All three tiers answer with something: Datapath a mask, Gate a
        // marker, Control a verdict. Declared so bodies using these bytes are
        // statement-segmentable rather than refused.
        tier_of(f).map(|_| true)
    }

    fn domain_name(&self, f: FnIndex) -> Option<&'static str> {
        recipe_at(f).map(|r| r.code)
    }
}
