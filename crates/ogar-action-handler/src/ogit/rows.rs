//! Per-type row ranges over the OGIT tables, behind the `quack` feature.
//!
//! [`TypeRows`] answers "which rows of this table belong to type `t`" as a
//! half-open range `[start, end)`. [`Table::TypeAttr`] (keyed by `type`) and
//! [`Table::Allowed`] (keyed by `src`) are written type by type in
//! [`OgitTables::load`], so each type's rows are contiguous and a range is two
//! numbers.
//!
//! # How it is built
//!
//! Once per loaded tree, through the typed front end, not by a hand loop:
//!
//! ```text
//! table("type_attr").count_by("type")
//!     │  bind_typed(&OgitBinder)      the key's code space is witnessed
//!     ▼
//! Agg::GroupReduce { key: Local(type), agg: Count }
//!     │  lower, execute_into(Out::I64)  the sink is the type universe
//!     ▼
//! counts[t]  ──prefix sum──▶  starts[t], starts[t + 1]
//! ```
//!
//! The sink has one slot per row of [`Table::Type`]; a code past it would be
//! dropped by the executor, so the counts must sum to the table's row count
//! ([`TypeRowsError::Uncounted`] otherwise).
//!
//! # Sortedness is checked, not assumed
//!
//! A prefix sum of counts is a range only if the rows are sorted by type. The
//! counts cannot see order, so [`TypeRows::new`] walks the key lane once and
//! refuses a table whose key ever decreases ([`TypeRowsError::Unsorted`]),
//! instead of handing out ranges that name the wrong rows.
//!
//! # One tree, and one layout
//!
//! A [`TypeRows`] keeps the [`OgitWitness`] and the row count of the tables it
//! was built from. [`TypeRows::range`] answers only against tables of that
//! code space ([`TypeRowsError::StaleTables`] otherwise), and
//! [`TypeRows::new`] fails closed on tables with no readable commit
//! ([`TypeRowsError::Unwitnessed`]).
//!
//! The witness and the row counts do not see everything. An uncommitted edit
//! that moves an attribute-list entry from one existing type to another keeps
//! the commit, the name-to-id digest and every row count, yet moves the
//! per-type boundaries. So every range handed out is also bound to the key
//! lane of the tables it is asked of, in O(1) and without allocating. Because
//! the lane is sorted by type, a range `[s, e)` for type `t` is the type's
//! rows in that lane iff
//!
//! - non-empty: `lane[s] == t`, `lane[e - 1] == t`, and the rows on either
//!   side (`s - 1`, `e`), where there are any, are not `t`;
//! - empty: `t` has no row at its insertion point, that is `lane[s - 1] < t`
//!   and `lane[s] > t`, where those rows exist;
//!
//! and every key of the lane is below the type count (checked at the last
//! row). A range that fails is refused with [`TypeRowsError::StaleTables`],
//! never answered.
//!
//! The check leans on the lane being sorted, which [`TypeRows::new`] verifies
//! for the tables it is built from and which holds for every [`OgitTables`]
//! by construction: the only constructor is [`OgitTables::load`] (struct
//! literal at `ogit.rs` `Ok(OgitTables { .. })`), which pushes `type_attr`
//! rows from an outer loop over the types in ascending id order and sorts the
//! `allowed` tuples by `(src, verb, dst)`; every field is private, there is no
//! `&mut self` method and no `Clone` or `Default`, so nothing outside this
//! crate can reorder a lane afterwards.

use std::fmt;
use std::ops::Range;

use lance_graph_mask_risc::{Foreign, LaneRef, Out, Planes, Scratch, execute_into, words_for};
use lance_graph_quack::bind::{Binder, table};
use lance_graph_quack::{Col, lower};

use super::{Lane, OgitTables, OgitWitness, Table, TypeId};

/// Why [`TypeRows`] could not be built, or a range was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeRowsError {
    /// The table is not keyed by type (only [`Table::TypeAttr`] and
    /// [`Table::Allowed`] are).
    NotPerType(Table),
    /// The tables' tree has no readable commit, so no witness can be taken.
    Unwitnessed,
    /// The tables are not those the ranges were built from: another tree or
    /// name-to-id mapping, or another row count.
    StaleTables,
    /// The key lane is not sorted: `row` holds a smaller type than the row
    /// before it, so a count per type is not a contiguous range.
    Unsorted {
        /// The first row whose key is below its predecessor's.
        row: usize,
    },
    /// The counts do not add up to the rows: a key names a type the type
    /// table does not have.
    Uncounted {
        /// Rows the grouped count reached.
        counted: usize,
        /// Rows the table has.
        rows: usize,
    },
    /// The grouped count did not bind, lower or run.
    Prepare(String),
}

impl fmt::Display for TypeRowsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeRowsError::NotPerType(t) => write!(f, "{t:?} is not keyed by type"),
            TypeRowsError::Unwitnessed => write!(
                f,
                "the OGIT tables' tree commit is unknown, so the ranges cannot be vouched for"
            ),
            TypeRowsError::StaleTables => {
                write!(f, "the tables are not those these ranges were built from")
            }
            TypeRowsError::Unsorted { row } => {
                write!(f, "the rows are not sorted by type at row {row}")
            }
            TypeRowsError::Uncounted { counted, rows } => write!(
                f,
                "the grouped count reached {counted} of {rows} rows: a key is no type"
            ),
            TypeRowsError::Prepare(e) => write!(f, "the grouped count could not run: {e}"),
        }
    }
}

impl std::error::Error for TypeRowsError {}

/// The `[start, end)` rows of one table for each type; see the
/// [module docs](self).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRows {
    table: Table,
    witness: OgitWitness,
    rows: usize,
    /// `types + 1` ascending row offsets: type `t` owns `starts[t]..starts[t + 1]`.
    starts: Vec<u32>,
}

/// The sorted key lane of a per-type table.
fn key_lane(table: Table) -> Option<Lane> {
    match table {
        Table::TypeAttr => Some(Lane::TypeAttrType),
        Table::Allowed => Some(Lane::AllowedSrc),
        _ => None,
    }
}

/// The query table and group key of a per-type table.
fn key_of(table: Table) -> Option<(&'static str, &'static str)> {
    match table {
        Table::TypeAttr => Some(("type_attr", "type")),
        Table::Allowed => Some(("allowed", "src")),
        _ => None,
    }
}

impl OgitTables {
    /// The per-type row ranges of `table`.
    ///
    /// # Errors
    /// See [`TypeRows::new`].
    pub fn type_rows(&self, table: Table) -> Result<TypeRows, TypeRowsError> {
        TypeRows::new(self, table)
    }
}

impl TypeRows {
    /// Counts the rows per type with `count_by` and turns the counts into
    /// ranges.
    ///
    /// # Errors
    /// [`TypeRowsError::NotPerType`] for a table with no type key,
    /// [`TypeRowsError::Unwitnessed`] if the commit is unknown,
    /// [`TypeRowsError::Unsorted`] if the rows are not sorted by type,
    /// [`TypeRowsError::Uncounted`] if a key is no type, and
    /// [`TypeRowsError::Prepare`] if the grouped count does not bind or run.
    pub fn new(tables: &OgitTables, which: Table) -> Result<Self, TypeRowsError> {
        let (name, key) = key_of(which).ok_or(TypeRowsError::NotPerType(which))?;
        let witness = tables.witness().ok_or(TypeRowsError::Unwitnessed)?;
        let binder = tables.binder();
        let prepare = |e: &dyn fmt::Debug| TypeRowsError::Prepare(format!("{e:?}"));

        // The grouped count, bound under the key's code space.
        let typed = table(name)
            .count_by(key)
            .bind_typed(&binder)
            .map_err(|e| prepare(&e))?;
        let query = typed.admit(&binder).map_err(|e| prepare(&e))?;
        let program = lower(query).map_err(|e| prepare(&e))?;

        // The table's lanes in the binder's column order; plane 0 is live.
        let t = binder.table(name).ok_or_else(|| prepare(&"no table"))?;
        let rows = tables.rows(which);
        let mut lanes = Vec::new();
        let mut col = 0u16;
        while let Some(lane) = binder.lane(t, Col(col)) {
            lanes.push(LaneRef::U32(lane));
            col += 1;
        }
        let key_col = binder
            .field(t, key)
            .ok_or_else(|| prepare(&"no key field"))?
            .col;
        let LaneRef::U32(keys) = lanes[usize::from(key_col.0)] else {
            return Err(prepare(&"key lane is not U32"));
        };
        if let Some(row) = (1..keys.len()).find(|&i| keys[i - 1] > keys[i]) {
            return Err(TypeRowsError::Unsorted { row });
        }

        let mut live = vec![u64::MAX; words_for(rows)];
        if !rows.is_multiple_of(64)
            && let Some(last) = live.last_mut()
        {
            *last = (1u64 << (rows % 64)) - 1;
        }
        let masks: [&[u64]; 1] = [&live];
        let planes = Planes {
            n_rows: rows,
            masks: &masks,
            lanes: &lanes,
        };
        let universe = tables.rows(Table::Type);
        let mut counts = vec![0i64; universe];
        let mut scratch = Scratch::for_program(&program, rows).map_err(|e| prepare(&e))?;
        execute_into(
            &program,
            &planes,
            &Foreign::NONE,
            &mut scratch,
            Out::I64(&mut counts),
        )
        .map_err(|e| prepare(&e))?;

        let counted = usize::try_from(counts.iter().sum::<i64>()).unwrap_or(usize::MAX);
        if counted != rows {
            return Err(TypeRowsError::Uncounted { counted, rows });
        }
        let mut starts = Vec::with_capacity(universe + 1);
        let mut at = 0u32;
        starts.push(at);
        for c in counts {
            at += u32::try_from(c).map_err(|_| prepare(&"count out of range"))?;
            starts.push(at);
        }
        Ok(Self {
            table: which,
            witness,
            rows,
            starts,
        })
    }

    /// The table these ranges are of.
    #[must_use]
    pub fn table(&self) -> Table {
        self.table
    }

    /// How many types there are (the type table's row count at build).
    #[must_use]
    pub fn types(&self) -> usize {
        self.starts.len() - 1
    }

    /// The code space the ranges were built under.
    #[must_use]
    pub fn witness(&self) -> OgitWitness {
        self.witness.clone()
    }

    /// The rows of type `ty`, without checking the tables. Empty for a type
    /// with no rows or no row of the type table. Only meaningful against the
    /// tables the ranges were built from; use [`TypeRows::range`] when that
    /// is not known.
    #[must_use]
    pub fn range_unchecked(&self, ty: TypeId) -> Range<usize> {
        let t = usize::from(ty.0);
        match (self.starts.get(t), self.starts.get(t + 1)) {
            (Some(&a), Some(&b)) => a as usize..b as usize,
            _ => 0..0,
        }
    }

    /// The rows of type `ty` in `tables`, answered only if `tables` are those
    /// the ranges were built from. Allocation-free.
    ///
    /// # Errors
    /// [`TypeRowsError::StaleTables`] for tables of another tree, another
    /// name-to-id mapping, another row count, or the same of all three with
    /// the rows of the types moved (see the [module docs](self)).
    pub fn range(&self, tables: &OgitTables, ty: TypeId) -> Result<Range<usize>, TypeRowsError> {
        if self.witness.admits(tables)
            && tables.rows(self.table) == self.rows
            && tables.rows(Table::Type) == self.types()
        {
            self.range_in_lane(tables, ty)
        } else {
            Err(TypeRowsError::StaleTables)
        }
    }

    /// The rows of type `ty`, checked against the key lane of `tables` alone:
    /// the O(1) boundary test of the [module docs](self). The caller has
    /// already admitted the tables' witness and sizes (as [`TypeRows::range`]
    /// and the validator do).
    pub(super) fn range_in_lane(
        &self,
        tables: &OgitTables,
        ty: TypeId,
    ) -> Result<Range<usize>, TypeRowsError> {
        let stale = Err(TypeRowsError::StaleTables);
        let Some(lane) = key_lane(self.table).map(|l| tables.lane(l)) else {
            return stale;
        };
        let len = lane.len();
        if len != self.rows || lane.last().is_some_and(|&k| k as usize >= self.types()) {
            return stale;
        }
        let t = u32::from(ty.0);
        if usize::from(ty.0) >= self.types() {
            // No key reaches the type count, so no row is of this type.
            return Ok(0..0);
        }
        let range = self.range_unchecked(ty);
        let (s, e) = (range.start, range.end);
        let at = |i: usize| lane.get(i).copied();
        let before = |i: usize| i.checked_sub(1).and_then(at);
        let bound = if s < e {
            at(s) == Some(t)
                && at(e - 1) == Some(t)
                && before(s).is_none_or(|k| k != t)
                && at(e).is_none_or(|k| k != t)
        } else {
            s <= len && before(s).is_none_or(|k| k < t) && at(s).is_none_or(|k| k > t)
        };
        if bound { Ok(range) } else { stale }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Names, TreeId};
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    /// Three types and a `type_attr` table whose `type` lane is `keys`, built
    /// field by field so that an order `load` never writes can be tried.
    fn fixture(keys: &[u32]) -> OgitTables {
        // One directory per call: the tests of this module run in parallel in
        // one process, and a directory named by the process id alone was
        // removed by one test while another was loading it.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("ogar-rows-unit-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut t = OgitTables::load(&root).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        t.types = Names::new(vec!["ogit:A".into(), "ogit:B".into(), "ogit:C".into()]);
        t.type_attr_type = keys.to_vec();
        t.type_attr_attr = vec![0; keys.len()];
        t.type_attr_flag = vec![0; keys.len()];
        t.tree = TreeId {
            commit: Some(SHA.into()),
        };
        t
    }

    #[test]
    fn sorted_keys_give_the_prefix_sum_of_their_counts() {
        let t = fixture(&[0, 0, 2, 2, 2]);
        let rows = TypeRows::new(&t, Table::TypeAttr).unwrap();
        let of = |i| rows.range(&t, TypeId(i)).unwrap();
        assert_eq!((of(0), of(1), of(2)), (0..2, 2..2, 2..5));
        assert_eq!(rows.range_unchecked(TypeId(3)), 0..0);
    }

    /// The counts of `[0, 2, 1, 1]` are `[1, 2, 1]`, a perfectly good prefix
    /// sum: only the order check can see that rows 1 and 2 are out of place.
    #[test]
    fn an_unsorted_table_is_refused_not_given_wrong_ranges() {
        let t = fixture(&[0, 2, 1, 1]);
        assert_eq!(
            TypeRows::new(&t, Table::TypeAttr),
            Err(TypeRowsError::Unsorted { row: 2 })
        );
    }

    #[test]
    fn a_key_past_the_type_table_is_uncounted() {
        let t = fixture(&[0, 1, 5]);
        assert_eq!(
            TypeRows::new(&t, Table::TypeAttr),
            Err(TypeRowsError::Uncounted {
                counted: 2,
                rows: 3
            })
        );
    }

    /// Moving a row from one type to another keeps the witness and the row
    /// counts but moves the boundaries: every range the move touches is
    /// refused, the untouched ones still answer.
    #[test]
    fn a_row_moved_between_types_refuses_the_ranges_it_touches() {
        let mut t = fixture(&[0, 0, 2, 2, 2]);
        let rows = TypeRows::new(&t, Table::TypeAttr).unwrap();
        // Type 0 gives a row to the empty type 1.
        t.type_attr_type = vec![0, 1, 2, 2, 2];
        let of = |t: &OgitTables, i| rows.range(t, TypeId(i));
        assert_eq!(of(&t, 0), Err(TypeRowsError::StaleTables));
        assert_eq!(of(&t, 1), Err(TypeRowsError::StaleTables));
        assert_eq!(of(&t, 2), Ok(2..5), "type 2 did not move");
        // Type 2 gives its first row to the empty type 1: both neighbours move.
        t.type_attr_type = vec![0, 0, 1, 2, 2];
        assert_eq!(of(&t, 1), Err(TypeRowsError::StaleTables));
        assert_eq!(of(&t, 2), Err(TypeRowsError::StaleTables));
        assert_eq!(of(&t, 0), Ok(0..2));
        // A key at or past the type count is refused for every type.
        t.type_attr_type = vec![0, 0, 2, 2, 3];
        assert_eq!(of(&t, 0), Err(TypeRowsError::StaleTables));
        // A type past the table is empty on tables that name no such key.
        t.type_attr_type = vec![0, 0, 2, 2, 2];
        assert_eq!(of(&t, 9), Ok(0..0));
        assert_eq!(of(&t, 1), Ok(2..2));
    }
}
