//! Q4 over the OGIT tables: is `(src, verb, dst)` an allowed edge?
//!
//! [`OgitEdges`] answers by binary search over the three lanes of
//! [`Table::Allowed`], which [`OgitTables::load`] keeps sorted by
//! `(src, verb, dst)` with no repeated row. It borrows the lanes
//! ([`OgitEdges::lanes`] returns the tables' own slices) and allocates
//! nothing on a query.
//!
//! - **Sortedness is checked, not assumed.** [`OgitEdges::new`] walks the
//!   lanes once and refuses a table whose rows are not strictly increasing
//!   ([`EdgeError::Unsorted`]), so a change to the load order cannot turn into
//!   silent wrong answers.
//! - **One tree.** An [`OgitEdges`] keeps the [`OgitWitness`] of the tables it
//!   was built from. [`OgitEdges::allowed_with`] answers a query only for ids
//!   of that code space: a witness of another tree, or of the same commit
//!   with another name-to-id mapping, is [`EdgeError::StaleCodeSpace`]. Tables
//!   whose commit cannot be read cannot be vouched for, and
//!   [`OgitEdges::new`] fails closed with [`EdgeError::Unwitnessed`].
//! - **Names are refused, not missed.** [`OgitEdges::allowed_names`] resolves
//!   through the tables' own lookup; a name the tree does not declare is
//!   [`EdgeError::UnknownName`], never `Ok(false)`.
//!
//! Multi-hop reachability (plan Q5) is not here.

use std::cmp::Ordering;
use std::fmt;

use super::{Lane, OgitTables, OgitWitness, Table, TypeId, VerbId};

/// Why an [`OgitEdges`] could not be built, or a query was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EdgeError {
    /// The tables' tree has no readable commit, so no witness can be taken.
    Unwitnessed,
    /// The witness held is not of these tables' tree and name-to-id mapping.
    StaleCodeSpace,
    /// The `allowed` rows are not strictly increasing by `(src, verb, dst)`
    /// (or its lanes differ in length): the binary search would be wrong.
    Unsorted {
        /// The first row that does not sort after its predecessor.
        row: usize,
    },
    /// A name the tree does not declare.
    UnknownName {
        /// What the name was for: `"src"`, `"verb"` or `"dst"`.
        role: &'static str,
        /// The name, as given.
        name: String,
    },
}

impl fmt::Display for EdgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdgeError::Unwitnessed => {
                write!(f, "the tables' tree has no readable commit to vouch for")
            }
            EdgeError::StaleCodeSpace => {
                write!(
                    f,
                    "the witness is of another OGIT tree or name-to-id mapping"
                )
            }
            EdgeError::Unsorted { row } => {
                write!(f, "the allowed rows are not sorted at row {row}")
            }
            EdgeError::UnknownName { role, name } => {
                write!(f, "{role} {name:?} is not declared by the tree")
            }
        }
    }
}

impl std::error::Error for EdgeError {}

/// The allowed-edge membership question over one [`OgitTables`]; see the
/// [module docs](self).
#[derive(Clone, Debug)]
pub struct OgitEdges<'t> {
    tables: &'t OgitTables,
    src: &'t [u32],
    verb: &'t [u32],
    dst: &'t [u32],
    witness: OgitWitness,
}

impl OgitTables {
    /// The edge membership question over these tables. It borrows them and
    /// copies nothing.
    ///
    /// # Errors
    /// See [`OgitEdges::new`].
    pub fn edges(&self) -> Result<OgitEdges<'_>, EdgeError> {
        OgitEdges::new(self)
    }
}

impl<'t> OgitEdges<'t> {
    /// Builds the question over `tables`, borrowing their `allowed` lanes.
    ///
    /// # Errors
    /// [`EdgeError::Unwitnessed`] if the tables' commit is unknown, and
    /// [`EdgeError::Unsorted`] if the rows are not strictly increasing.
    pub fn new(tables: &'t OgitTables) -> Result<Self, EdgeError> {
        let witness = tables.witness().ok_or(EdgeError::Unwitnessed)?;
        let (src, verb, dst) = (
            tables.lane(Lane::AllowedSrc),
            tables.lane(Lane::AllowedVerb),
            tables.lane(Lane::AllowedDst),
        );
        if src.len() != verb.len() || src.len() != dst.len() {
            return Err(EdgeError::Unsorted {
                row: src.len().min(verb.len()).min(dst.len()),
            });
        }
        let key = |i: usize| (src[i], verb[i], dst[i]);
        if let Some(row) = (1..src.len()).find(|&i| key(i - 1) >= key(i)) {
            return Err(EdgeError::Unsorted { row });
        }
        Ok(Self {
            tables,
            src,
            verb,
            dst,
            witness,
        })
    }

    /// The code space the ids this answers for belong to.
    #[must_use]
    pub fn witness(&self) -> OgitWitness {
        self.witness.clone()
    }

    /// The `(src, verb, dst)` lanes searched: the tables' own slices, lent
    /// without a copy.
    #[must_use]
    pub fn lanes(&self) -> (&'t [u32], &'t [u32], &'t [u32]) {
        (self.src, self.verb, self.dst)
    }

    /// Whether `(src, verb, dst)` is an `ogit:allowed` row of these tables.
    /// Ids that are no row of their table are simply not in any edge.
    /// Allocation-free.
    #[must_use]
    pub fn allowed(&self, src: TypeId, verb: VerbId, dst: TypeId) -> bool {
        let want = (u32::from(src.0), u32::from(verb.0), u32::from(dst.0));
        let (mut lo, mut hi) = (0, self.src.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let row = (self.src[mid], self.verb[mid], self.dst[mid]);
            match row.cmp(&want) {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return true,
            }
        }
        false
    }

    /// [`OgitEdges::allowed`] for ids kept under `held`: answered only if
    /// `held` is the witness of these tables.
    ///
    /// # Errors
    /// [`EdgeError::StaleCodeSpace`] if `held` is of another tree or
    /// name-to-id mapping.
    pub fn allowed_with(
        &self,
        held: &OgitWitness,
        src: TypeId,
        verb: VerbId,
        dst: TypeId,
    ) -> Result<bool, EdgeError> {
        if *held == self.witness && held.admits(self.tables) {
            Ok(self.allowed(src, verb, dst))
        } else {
            Err(EdgeError::StaleCodeSpace)
        }
    }

    /// [`OgitEdges::allowed`] by CURIE, resolved through the tables' code
    /// space.
    ///
    /// # Errors
    /// [`EdgeError::UnknownName`] for the first of `src`, `verb`, `dst` that
    /// the tree does not declare.
    pub fn allowed_names(&self, src: &str, verb: &str, dst: &str) -> Result<bool, EdgeError> {
        let unknown = |role: &'static str, name: &str| EdgeError::UnknownName {
            role,
            name: name.to_owned(),
        };
        let s = self
            .tables
            .type_id(src)
            .ok_or_else(|| unknown("src", src))?;
        let v = self
            .tables
            .verb_id(verb)
            .ok_or_else(|| unknown("verb", verb))?;
        let d = self
            .tables
            .type_id(dst)
            .ok_or_else(|| unknown("dst", dst))?;
        Ok(self.allowed(s, v, d))
    }

    /// How many edges there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tables.rows(Table::Allowed)
    }

    /// Whether there are no edges.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
