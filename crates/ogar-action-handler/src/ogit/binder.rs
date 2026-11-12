//! A Quack [`Binder`] over the OGIT tables, behind the `quack` feature.
//!
//! [`OgitBinder`] is the only place in a bound query's life where an OGIT name
//! is text. It names the five tables and their lanes, turns a name literal
//! into the `u16` row ordinal the lane holds, and lends the lanes themselves
//! to whoever runs the query. Past [`Draft::bind`] no string exists.
//!
//! | Table | Fields |
//! |---|---|
//! | `type` | `domain` |
//! | `attribute` | `domain` |
//! | `type_attr` | `type`, `attr`, `flag` |
//! | `verb` | `domain` |
//! | `allowed` | `src`, `verb`, `dst` |
//!
//! - **Every field is a [`FieldKind::Code`] lane** with no validity plane: no
//!   row of these tables is NULL anywhere.
//! - **A literal is a name as the tables spell it**: a CURIE (`ogit:Person`,
//!   `ogit.MARS:Machine`) for `type`, `src` and `dst`, `attr` and `verb`; a
//!   domain (`MARS`, or `""` for the SGO core) for `domain`; `mandatory`,
//!   `optional` or `indexed` for `flag`. It is resolved by the tables' own
//!   sorted lookup and never minted: a name the tree does not declare is
//!   [`BindError::UnknownValue`].
//! - **Plane 0 is the live plane.** [`OgitBinder::live`] is `Mask(0)` for every
//!   table, and every row of every table is live, so the caller supplies an
//!   all-ones plane there.
//! - **Lanes are lent**: [`OgitBinder::lane`] hands out the slice
//!   [`OgitTables::lane`] does, for the executor's `LaneRef::U32`.
//!
//! # Code space
//!
//! A bound literal is a row ordinal of one OGIT tree, and the same ordinal
//! means another row in another tree. [`OgitBinder`] therefore implements
//! [`CodeWitness`] with the tables' [`TreeId`] as the witness:
//! [`Draft::bind_typed`] holds the tree a query was bound under and
//! [`TypedQuery::admit`] refuses it against tables of any other tree with
//! [`BindError::StaleCodeSpace`].
//!
//! A tree whose commit could not be read ([`TreeId::commit`] is `None`)
//! cannot be vouched for, so it fails closed: [`OgitBinder::code_witness`]
//! gives no witness and `bind_typed` is [`BindError::Unwitnessed`]; a held
//! witness without a commit is never admitted. Plain [`Draft::bind`] does not
//! look at the tree.
//!
//! The witness covers literal codes bound through Quack. It says nothing of
//! planes built from ids by other code.
//!
//! [`Draft::bind`]: lance_graph_quack::bind::Draft::bind
//! [`Draft::bind_typed`]: lance_graph_quack::bind::Draft::bind_typed
//! [`TypedQuery::admit`]: lance_graph_quack::bind::TypedQuery::admit

use lance_graph_quack::bind::{BindError, Binder, BoundField, CodeWitness, FieldKind, TableId};
use lance_graph_quack::{Col, Mask};

use super::{Flag, Lane, OgitTables, Table, TreeId};

/// Table names, in [`TableId`] order.
const TABLES: [(&str, Table); 5] = [
    ("type", Table::Type),
    ("attribute", Table::Attribute),
    ("type_attr", Table::TypeAttr),
    ("verb", Table::Verb),
    ("allowed", Table::Allowed),
];

/// A table's fields in lane order: the position is the field's [`Col`].
fn fields(table: Table) -> &'static [(&'static str, Lane)] {
    match table {
        Table::Type => &[("domain", Lane::TypeDomain)],
        Table::Attribute => &[("domain", Lane::AttributeDomain)],
        Table::TypeAttr => &[
            ("type", Lane::TypeAttrType),
            ("attr", Lane::TypeAttrAttr),
            ("flag", Lane::TypeAttrFlag),
        ],
        Table::Verb => &[("domain", Lane::VerbDomain)],
        Table::Allowed => &[
            ("src", Lane::AllowedSrc),
            ("verb", Lane::AllowedVerb),
            ("dst", Lane::AllowedDst),
        ],
    }
}

fn table_of(id: TableId) -> Option<Table> {
    TABLES.get(usize::try_from(id.0).ok()?).map(|&(_, t)| t)
}

/// A Quack binder over one [`OgitTables`]; see the [module docs](self).
#[derive(Clone, Copy, Debug)]
pub struct OgitBinder<'t> {
    tables: &'t OgitTables,
}

impl OgitTables {
    /// A Quack binder over these tables. It borrows them and copies nothing.
    #[must_use]
    pub fn binder(&self) -> OgitBinder<'_> {
        OgitBinder { tables: self }
    }
}

impl<'t> OgitBinder<'t> {
    /// The lane a bound `(table, col)` reads, lent from the tables without a
    /// copy: the slice [`OgitTables::lane`] returns. `None` for a table or
    /// column [`Binder::table`] and [`Binder::field`] never issued.
    #[must_use]
    pub fn lane(&self, table: TableId, col: Col) -> Option<&'t [u32]> {
        let &(_, lane) = fields(table_of(table)?).get(usize::from(col.0))?;
        Some(self.tables.lane(lane))
    }

    /// The lane of a field, if `col` is one of the table's columns.
    fn lane_of(table: TableId, col: Col) -> Option<Lane> {
        fields(table_of(table)?)
            .get(usize::from(col.0))
            .map(|&(_, lane)| lane)
    }
}

impl Binder for OgitBinder<'_> {
    fn table(&self, name: &str) -> Option<TableId> {
        let i = TABLES.iter().position(|&(n, _)| n == name)?;
        u32::try_from(i).ok().map(TableId)
    }

    fn live(&self, _: TableId) -> Mask {
        Mask(0)
    }

    fn field(&self, table: TableId, name: &str) -> Option<BoundField> {
        let i = fields(table_of(table)?)
            .iter()
            .position(|&(n, _)| n == name)?;
        Some(BoundField {
            col: Col(u16::try_from(i).ok()?),
            kind: FieldKind::Code,
            validity: None,
        })
    }

    fn code(&self, table: TableId, field: Col, literal: &str) -> Option<u32> {
        let t = self.tables;
        match Self::lane_of(table, field)? {
            Lane::TypeDomain | Lane::AttributeDomain | Lane::VerbDomain => t.domain_id(literal),
            Lane::TypeAttrType | Lane::AllowedSrc | Lane::AllowedDst => {
                t.type_id(literal).map(|id| u32::from(id.0))
            }
            Lane::TypeAttrAttr => t.attribute_id(literal).map(|id| u32::from(id.0)),
            Lane::AllowedVerb => t.verb_id(literal).map(|id| u32::from(id.0)),
            Lane::TypeAttrFlag => match literal {
                "mandatory" => Some(Flag::Mandatory as u32),
                "optional" => Some(Flag::Optional as u32),
                "indexed" => Some(Flag::Indexed as u32),
                _ => None,
            },
        }
    }
}

impl CodeWitness for OgitBinder<'_> {
    /// The OGIT tree the codes are ordinals of.
    type Witness = TreeId;

    fn code_witness(&self, table: TableId, col: Col) -> Option<TreeId> {
        Self::lane_of(table, col)?;
        let tree = self.tables.tree();
        tree.commit().is_some().then(|| tree.clone())
    }

    fn admit_code(&self, table: TableId, col: Col, held: &TreeId) -> Result<(), BindError> {
        Self::lane_of(table, col).ok_or(BindError::StaleCodeSpace)?;
        let tree = self.tables.tree();
        if tree.commit().is_some() && tree == held {
            Ok(())
        } else {
            Err(BindError::StaleCodeSpace)
        }
    }
}
