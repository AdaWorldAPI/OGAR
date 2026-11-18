//! The Java OGIT validator's structural checks over the OGIT tables, behind
//! the `quack` feature.
//!
//! [`Validator`] answers what `EntityValidator.validate` answers before its
//! authority check (`services/java-ogit/.../entity/EntityValidator.java`,
//! lines 251-284), on the same tables [`OgitBinder`](super::OgitBinder) binds:
//!
//! | Check | Java | Here |
//! |---|---|---|
//! | `UNKNOWN_TYPE` | `:253-258` the type is a declared entity type | a lookup in the type spellings, built once (plan Q1) |
//! | `MISSING_MANDATORY_ATTRIBUTE` | `:261-271` the type's own mandatory attributes are present and non-blank | one mask-risc program over `type_attr` (plan Q3) |
//! | `UNKNOWN_PROPERTY` | `:275-284` every key is a declared attribute or a system key | a lookup in the attribute spellings, built once (plan Q2) |
//!
//! The Java authority check (`:290`, the bundled Scala validator) has no
//! counterpart here (plan R5).
//!
//! # Semantics kept from the Java
//!
//! - **Order.** The type first, then the mandatory attributes, then the keys.
//!   Java stops at the first violation; [`Validator::validate`] reports every
//!   one, in that order, so its first element is Java's verdict.
//!   An unknown type is reported alone, as Java returns at once.
//! - **Mandatory means the type's own list**, in TTL list order, not an
//!   inherited one (`:318-322`). A value that is absent, null or blank counts
//!   as missing (`:263-265`): [`Validator::validate_entries`] takes whether
//!   each value is non-blank, and [`Validator::validate`] treats every value
//!   as non-blank.
//! - **A key is known if the ontology declares it anywhere**, not only for
//!   this type: Java's known set is every declared attribute plus every
//!   member of every mandatory/optional/indexed list (`:275`,
//!   `OntologyLoader.java:271-308`). So `UNKNOWN_PROPERTY` is a lookup in the
//!   whole attribute table, not in the type's own lists.
//! - **System keys are ignored** (`OntologyLoader.java:117-128`):
//!   [`SYSTEM_KEYS`], ten keys, `ogit/_id` and `ogit/_type` among them.
//!
//! # Spellings
//!
//! Writes name attributes in the JanusGraph key form, `uriToJgKey` of the
//! attribute's URI (`OntologyLoader.java:318-333`): the URI without scheme
//! and host, so `ogit:name` is `ogit/name` and `ogit.MARS.Machine:class` is
//! `ogit/MARS/Machine/class`. Types arrive as `ogit/Auth/Account` or as the
//! full URI (`OgitEntity.java:75-79`, `jgKeyToUri`). The validator also
//! accepts the CURIE as the tables spell it and, for the SGO core, the
//! fixture form `ogit.Person`. Every spelling is resolved through an index
//! built once, so a call hashes the key it is given and allocates nothing.
//!
//! # One tree
//!
//! A [`Validator`] is made from one set of tables and keeps their
//! [`OgitWitness`] (tree commit and name-to-id digest, the binder's own) and
//! row counts. [`Validator::validate`] takes the tables again, so that it
//! lends their lanes to the executor instead of copying them, and refuses
//! tables of any other tree, or of the same commit with other names or sizes,
//! with [`ValidatorError::StaleTables`]. Tables whose commit cannot be read
//! cannot be vouched for and give [`ValidatorError::Unwitnessed`]: the check
//! fails closed, as the binder does. The type's row range is also checked
//! against the supplied tables' `type` lane on every call (O(1), see
//! [`TypeRows`]), because an uncommitted edit can move rows between types
//! and keep the commit, the digest and every row count: such tables give
//! [`ValidatorError::StaleTables`] too.
//!
//! # Zero copy and allocation
//!
//! Q3 is a program over `type_attr` lowered once from quack's `Filter`s
//! (no Quack frontend addition: the plan's R7 methods do not exist, so the
//! filters are built by hand, as the plan's P2b fallback says) and kept as a
//! mask-risc `CompiledProgram`:
//!
//! ```text
//! type ∈ TYPE  and  flag = mandatory  and  attr ∉ PRESENT
//! ```
//!
//! `TYPE` and `PRESENT` are caller-owned parameter planes over the `type` and
//! `attribute` tables, one bit set per call and cleared after; they are the
//! resident foreign planes `Filter::Semijoin` requires. The program runs over
//! one type's rows, not the whole table: `type_attr` is written type by type,
//! and [`TypeRows`] (counted once per tree with Quack's `count_by`) gives each
//! type's `[start, end)` as the executor's row extent. The `TYPE` semijoin
//! stays in the program, so a run over a wider extent gives the same rows. The
//! three `type_attr` lanes are lent from the tables as `LaneRef::U32`. The scratch, the planes
//! and the result mask are allocated when the validator is made. A call that
//! finds no violation allocates nothing; one that does allocates its result
//! vector.

use std::collections::HashMap;
use std::fmt;

use lance_graph_mask_risc::{
    CompiledProgram, Foreign, ForeignPlane as ForeignMask, LaneRef, Out, Planes, Scratch,
    execute_compiled, words_for,
};
use lance_graph_quack::bind::{Binder, CodeWitness, TableId};
use lance_graph_quack::{Agg, Cmp, Col, Filter, ForeignPlane, Query, lower};

use super::{AttrId, Flag, OgitTables, OgitWitness, Table, TypeId, TypeRows};

/// The keys HIRO injects on every vertex and the ontology does not declare:
/// `OntologyLoader.SYSTEM_PROPERTY_KEYS`, `OntologyLoader.java:117-128`.
/// [`Validator::validate`] never reports one as an unknown property.
pub const SYSTEM_KEYS: [&str; 10] = [
    "ogit/_id",
    "ogit/_type",
    "ogit/_creator",
    "ogit/_modified-on",
    "ogit/_created-on",
    "ogit/_owner",
    "ogit/_organization",
    "ogit/_is-deleted",
    "ogit/_v",
    "ogit/_v-id",
];

/// Whether `key` is one of the [`SYSTEM_KEYS`].
#[must_use]
pub fn is_system_key(key: &str) -> bool {
    SYSTEM_KEYS.contains(&key)
}

/// One structural violation, with the name it is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Violation<'k> {
    /// `UNKNOWN_TYPE`: the type, as it was given.
    UnknownType(&'k str),
    /// `MISSING_MANDATORY_ATTRIBUTE`: the attribute the type requires.
    /// [`Validator::key`] names it in the JanusGraph form.
    MissingMandatoryAttribute(AttrId),
    /// `UNKNOWN_PROPERTY`: the key, as it was given.
    UnknownProperty(&'k str),
}

impl Violation<'_> {
    /// The Java validator's error code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Violation::UnknownType(_) => "UNKNOWN_TYPE",
            Violation::MissingMandatoryAttribute(_) => "MISSING_MANDATORY_ATTRIBUTE",
            Violation::UnknownProperty(_) => "UNKNOWN_PROPERTY",
        }
    }
}

/// Why a [`Validator`] could not be made or could not run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidatorError {
    /// The tables' tree commit could not be read, so nothing can vouch that
    /// the ids the validator holds mean what the tables mean.
    Unwitnessed,
    /// The tables are not the ones the validator was made from: another tree,
    /// the same commit with another name-to-id mapping, or other row counts.
    StaleTables,
    /// The program did not lower or the scratch could not be sized.
    Prepare(String),
    /// The executor refused a call.
    Execute(String),
}

impl fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidatorError::Unwitnessed => {
                write!(
                    f,
                    "the OGIT tables' tree commit is unknown, so the validator cannot vouch for their ids"
                )
            }
            ValidatorError::StaleTables => {
                write!(f, "the tables are not those this validator was made from")
            }
            ValidatorError::Prepare(e) => write!(f, "the validator could not be prepared: {e}"),
            ValidatorError::Execute(e) => write!(f, "the validator program was refused: {e}"),
        }
    }
}

impl std::error::Error for ValidatorError {}

/// The row counts the resident planes and the result mask were sized for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sizes {
    types: usize,
    attributes: usize,
    type_attr: usize,
}

impl Sizes {
    fn of(tables: &OgitTables) -> Self {
        Self {
            types: tables.rows(Table::Type),
            attributes: tables.rows(Table::Attribute),
            type_attr: tables.rows(Table::TypeAttr),
        }
    }
}

/// The structural validator, prepared once per loaded [`OgitTables`]; see the
/// [module docs](self).
pub struct Validator {
    witness: OgitWitness,
    table: TableId,
    sizes: Sizes,
    /// Columns of `type_attr`, in the binder's order: `type`, `attr`, `flag`.
    cols: [Col; 3],
    program: CompiledProgram,
    scratch: Scratch<'static>,
    /// `TYPE`: one bit over `type`, set for the call.
    type_plane: Vec<u64>,
    /// `PRESENT`: a bit over `attribute` per non-blank key of the call.
    present_plane: Vec<u64>,
    /// The mask the program writes: the missing mandatory rows of `type_attr`.
    /// Only the rows of the type last run hold bits; see [`Validator::run`].
    missing: Vec<u64>,
    /// The rows of `type_attr` per type, counted once with `count_by`.
    rows: TypeRows,
    /// The row range the last run wrote into `missing`.
    dirty: std::ops::Range<usize>,
    types: HashMap<Box<str>, u16>,
    attributes: HashMap<Box<str>, u16>,
    /// Each attribute's JanusGraph key, by id.
    keys: Vec<Box<str>>,
}

impl fmt::Debug for Validator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Validator")
            .field("witness", &self.witness)
            .field("sizes", &self.sizes)
            .finish_non_exhaustive()
    }
}

/// `ogit.A.B:local` as the JanusGraph key `ogit/A/B/local`
/// (`OntologyLoader.uriToJgKey` of the CURIE's URI, `:318-333`); `None` for a
/// name whose prefix is not an OGIT one.
fn jg_key(curie: &str) -> Option<String> {
    let (prefix, local) = curie.split_once(':')?;
    let domain = match prefix {
        "ogit" => "",
        p => p.strip_prefix("ogit.")?,
    };
    let mut key = String::from("ogit/");
    for segment in domain.split('.').filter(|s| !s.is_empty()) {
        key.push_str(segment);
        key.push('/');
    }
    key.push_str(local);
    Some(key)
}

fn index(names: impl Iterator<Item = (u16, Vec<String>)>) -> HashMap<Box<str>, u16> {
    let mut map = HashMap::new();
    // Ids ascend, so on a clash the lowest id keeps the spelling.
    for (id, spellings) in names {
        for s in spellings {
            map.entry(s.into_boxed_str()).or_insert(id);
        }
    }
    map
}

fn rows_u16(n: usize) -> impl Iterator<Item = u16> {
    (0..n).filter_map(|i| u16::try_from(i).ok())
}

impl Validator {
    /// Prepares the checks for `tables`: lowers the mandatory-attribute
    /// program once, sizes the planes and builds the spelling indexes.
    ///
    /// # Errors
    /// [`ValidatorError::Unwitnessed`] if the tables' commit is unknown;
    /// [`ValidatorError::Prepare`] if the program cannot be lowered.
    pub fn new(tables: &OgitTables) -> Result<Self, ValidatorError> {
        let binder = tables.binder();
        let table = binder
            .table("type_attr")
            .ok_or_else(|| ValidatorError::Prepare("no type_attr table".into()))?;
        let col = |name: &str| {
            binder
                .field(table, name)
                .map(|f| f.col)
                .ok_or_else(|| ValidatorError::Prepare(format!("no field {name}")))
        };
        let cols = [col("type")?, col("attr")?, col("flag")?];
        let witness = binder
            .code_witness(table, cols[0])
            .ok_or(ValidatorError::Unwitnessed)?;
        let mandatory = binder
            .code(table, cols[2], "mandatory")
            .ok_or_else(|| ValidatorError::Prepare("no mandatory flag".into()))?;
        debug_assert_eq!(mandatory, Flag::Mandatory as u32);

        // Q3: the rows are the missing mandatory attributes of the type in
        // plane 0, given the attributes present in plane 1.
        let query = Query {
            filter: Filter::and([
                Filter::semijoin(cols[0], ForeignPlane(0)),
                Filter::cmp(cols[2], Cmp::EqU32(mandatory)),
                Filter::negate(Filter::semijoin(cols[1], ForeignPlane(1))),
            ]),
            agg: Agg::Rows,
        };
        let program = lower(&query).map_err(|e| ValidatorError::Prepare(format!("{e:?}")))?;
        let sizes = Sizes::of(tables);
        let rows = TypeRows::new(tables, Table::TypeAttr).map_err(|e| match e {
            super::TypeRowsError::Unwitnessed => ValidatorError::Unwitnessed,
            e => ValidatorError::Prepare(e.to_string()),
        })?;
        let scratch = Scratch::for_program(&program, sizes.type_attr)
            .map_err(|e| ValidatorError::Prepare(format!("{e:?}")))?;

        let types = index(rows_u16(sizes.types).map(|i| {
            let curie = tables.type_name(TypeId(i)).unwrap_or_default();
            let mut s = vec![curie.to_owned()];
            if let Some(jg) = jg_key(curie) {
                s.push(format!("http://www.purl.org/{jg}"));
                s.push(jg);
            }
            // The fixture form of an SGO core type: `ogit:Person` is `ogit.Person`.
            if let Some(local) = curie.strip_prefix("ogit:") {
                s.push(format!("ogit.{local}"));
            }
            (i, s)
        }));
        let keys: Vec<Box<str>> = rows_u16(sizes.attributes)
            .map(|i| {
                let curie = tables.attribute_name(AttrId(i)).unwrap_or_default();
                jg_key(curie).unwrap_or_else(|| curie.to_owned()).into()
            })
            .collect();
        let attributes = index(rows_u16(sizes.attributes).map(|i| {
            let curie = tables.attribute_name(AttrId(i)).unwrap_or_default();
            (i, vec![curie.to_owned(), keys[usize::from(i)].to_string()])
        }));

        Ok(Self {
            witness,
            table,
            sizes,
            cols,
            program: CompiledProgram::new(program),
            scratch,
            type_plane: vec![0; words_for(sizes.types)],
            present_plane: vec![0; words_for(sizes.attributes)],
            missing: vec![0; words_for(sizes.type_attr)],
            rows,
            dirty: 0..0,
            types,
            attributes,
            keys,
        })
    }

    /// The compiled Q3 program, for a caller that wants to run it elsewhere
    /// (the reference oracle, in the tests).
    #[must_use]
    pub fn program(&self) -> &CompiledProgram {
        &self.program
    }

    /// An attribute's JanusGraph key, the form a reject message names; its
    /// CURIE for a name with no OGIT prefix.
    #[must_use]
    pub fn key(&self, id: AttrId) -> Option<&str> {
        self.keys.get(usize::from(id.0)).map(AsRef::as_ref)
    }

    /// A type, by any spelling a write uses.
    #[must_use]
    pub fn type_id(&self, spelling: &str) -> Option<TypeId> {
        self.types.get(spelling).map(|&i| TypeId(i))
    }

    /// An attribute, by its CURIE or its JanusGraph key.
    #[must_use]
    pub fn attribute_id(&self, spelling: &str) -> Option<AttrId> {
        self.attributes.get(spelling).map(|&i| AttrId(i))
    }

    /// The three `type_attr` lanes the program reads (`type`, `attr`,
    /// `flag`), lent from `tables` with no copy.
    #[must_use]
    pub fn lanes<'t>(&self, tables: &'t OgitTables) -> [&'t [u32]; 3] {
        let binder = tables.binder();
        self.cols.map(|c| binder.lane(self.table, c).unwrap_or(&[]))
    }

    /// The addresses of the buffers prepared at construction: the `TYPE` and
    /// `PRESENT` planes and the result mask. They never move for the life of
    /// the validator, so no call builds a plane.
    #[doc(hidden)]
    #[must_use]
    pub fn resident_addresses(&self) -> [usize; 3] {
        [
            self.type_plane.as_ptr() as usize,
            self.present_plane.as_ptr() as usize,
            self.missing.as_ptr() as usize,
        ]
    }

    fn admit(&self, tables: &OgitTables) -> Result<(), ValidatorError> {
        let binder = tables.binder();
        if Sizes::of(tables) != self.sizes
            || binder
                .admit_code(self.table, self.cols[0], &self.witness)
                .is_err()
        {
            return Err(ValidatorError::StaleTables);
        }
        Ok(())
    }

    fn set(plane: &mut [u64], bit: u16) {
        plane[usize::from(bit) / 64] |= 1 << (bit % 64);
    }

    /// Runs the program for the type in `TYPE` and the attributes in
    /// `PRESENT`, over the rows of `ty` alone; the missing rows land in
    /// `self.missing`.
    ///
    /// The type's rows are one contiguous extent, so the executor visits only
    /// them. A partial extent writes only its own bits and leaves the rest of
    /// the result as it was, so the words the previous run wrote are cleared
    /// first: the mask then holds this type's rows and nothing else, as a scan
    /// of the whole table gives.
    fn run(&mut self, tables: &OgitTables, ty: TypeId) -> Result<(), ValidatorError> {
        // Bound to the supplied tables' key lane, not only to their witness
        // and sizes: an edit can move rows between types and change neither.
        let extent = self
            .rows
            .range_in_lane(tables, ty)
            .map_err(|_| ValidatorError::StaleTables)?;
        let stale = self.dirty.clone();
        if !stale.is_empty() {
            self.missing[stale.start / 64..=(stale.end - 1) / 64].fill(0);
        }
        self.dirty = extent.clone();
        if extent.is_empty() {
            return Ok(());
        }
        let [ty, attr, flag] = self.lanes(tables);
        let lanes = [LaneRef::U32(ty), LaneRef::U32(attr), LaneRef::U32(flag)];
        let planes = Planes {
            n_rows: self.sizes.type_attr,
            masks: &[],
            lanes: &lanes,
        };
        let foreign = [
            ForeignMask {
                words: &self.type_plane,
                rows: self.sizes.types,
            },
            ForeignMask {
                words: &self.present_plane,
                rows: self.sizes.attributes,
            },
        ];
        let foreign = Foreign {
            planes: &foreign,
            lanes: &[],
        };
        execute_compiled(
            &self.program.compiled(),
            &planes,
            &foreign,
            &mut self.scratch,
            Out::Mask(&mut self.missing),
            extent,
        )
        .map(drop)
        .map_err(|e| ValidatorError::Execute(format!("{e:?}")))
    }

    /// The rows of `type_attr` that are mandatory attributes of `ty` missing
    /// from `present`, as the program's result mask: `words_for(rows)` words,
    /// bit `r` for row `r`. The mask is the validator's own buffer and is
    /// overwritten by the next call.
    ///
    /// # Errors
    /// [`ValidatorError::StaleTables`] for tables that are not the
    /// validator's; [`ValidatorError::Execute`] if the executor refuses.
    pub fn missing_mask(
        &mut self,
        tables: &OgitTables,
        ty: TypeId,
        present: &[AttrId],
    ) -> Result<&[u64], ValidatorError> {
        self.admit(tables)?;
        if usize::from(ty.0) >= self.sizes.types
            || present
                .iter()
                .any(|a| usize::from(a.0) >= self.sizes.attributes)
        {
            return Err(ValidatorError::StaleTables);
        }
        Self::set(&mut self.type_plane, ty.0);
        for a in present {
            Self::set(&mut self.present_plane, a.0);
        }
        let ran = self.run(tables, ty);
        self.type_plane.fill(0);
        self.present_plane.fill(0);
        ran.map(|()| self.missing.as_slice())
    }

    /// Validates a write of type `ty` carrying `keys`, every value non-blank.
    /// See [`Validator::validate_entries`].
    ///
    /// # Errors
    /// As [`Validator::validate_entries`].
    pub fn validate<'k>(
        &mut self,
        tables: &OgitTables,
        ty: &'k str,
        keys: &[&'k str],
    ) -> Result<Vec<Violation<'k>>, ValidatorError> {
        self.validate_inner(tables, ty, keys.iter().map(|&k| (k, true)))
    }

    /// Validates a write of type `ty` carrying `entries` in body order, each a
    /// key and whether its value is non-blank (a blank value counts as
    /// absent for the mandatory check, as in Java, `:263-265`).
    ///
    /// Returns every violation, in the Java validator's order: an unknown
    /// type alone, otherwise each missing mandatory attribute in the type's
    /// TTL list order, then each unknown key in body order. An empty vector
    /// accepts. The first element is the verdict the Java validator gives.
    ///
    /// # Errors
    /// [`ValidatorError::StaleTables`] if `tables` are not those the
    /// validator was made from; [`ValidatorError::Execute`] if the executor
    /// refuses.
    pub fn validate_entries<'k>(
        &mut self,
        tables: &OgitTables,
        ty: &'k str,
        entries: &[(&'k str, bool)],
    ) -> Result<Vec<Violation<'k>>, ValidatorError> {
        self.validate_inner(tables, ty, entries.iter().copied())
    }

    fn validate_inner<'k>(
        &mut self,
        tables: &OgitTables,
        ty: &'k str,
        entries: impl Iterator<Item = (&'k str, bool)> + Clone,
    ) -> Result<Vec<Violation<'k>>, ValidatorError> {
        self.admit(tables)?;
        let mut out = Vec::new();
        let Some(&t) = self.types.get(ty) else {
            out.push(Violation::UnknownType(ty));
            return Ok(out);
        };

        // Keys: a known one sets its PRESENT bit when its value is non-blank.
        let mut unknown = false;
        Self::set(&mut self.type_plane, t);
        for (key, non_blank) in entries.clone() {
            match self.attributes.get(key) {
                Some(&a) => {
                    if non_blank {
                        Self::set(&mut self.present_plane, a);
                    }
                }
                None => unknown |= !is_system_key(key),
            }
        }
        let ran = self.run(tables, TypeId(t));
        self.type_plane.fill(0);
        self.present_plane.fill(0);
        ran?;

        let attrs = self.lanes(tables)[1];
        // Only this type's rows were written: read just the words they touch.
        let touched = self.dirty.clone();
        let words = if touched.is_empty() {
            0..0
        } else {
            touched.start / 64..(touched.end - 1) / 64 + 1
        };
        for w in words {
            let mut bits = self.missing[w];
            while bits != 0 {
                let r = w * 64 + bits.trailing_zeros() as usize;
                bits &= bits - 1;
                if let Some(&a) = attrs.get(r)
                    && let Ok(a) = u16::try_from(a)
                {
                    out.push(Violation::MissingMandatoryAttribute(AttrId(a)));
                }
            }
        }
        if unknown {
            for (key, _) in entries {
                if !self.attributes.contains_key(key) && !is_system_key(key) {
                    out.push(Violation::UnknownProperty(key));
                }
            }
        }
        Ok(out)
    }
}
