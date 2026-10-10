//! The OGIT tables HIRO validates against, and the Ontology cache.
//!
//! [`OgitTables`] reads an OGIT tree once, through OGAR's own reader
//! ([`parse_file`], [`parse_verb`]), into five tables. Every row is addressed
//! by a `u16`, and every column is a `u32` lane lent as a slice
//! ([`OgitTables::lane`]), so a mask executor can read it without a copy.
//!
//! | Table | One row per | Lanes | Row order |
//! |---|---|---|---|
//! | [`Table::Type`] | entity type, SGO core included | domain | domain, name |
//! | [`Table::Attribute`] | declared attribute | domain | domain, name |
//! | [`Table::TypeAttr`] | attribute a type declares | type, attr, flag | type, flag, TTL list position |
//! | [`Table::Verb`] | verb | domain | domain, name |
//! | [`Table::Allowed`] | `ogit:allowed` tuple | src, verb, dst | src, verb, dst |
//!
//! - **Ids are row ordinals**, so the same OGIT tree always gives the same ids.
//! - **A table past 65,536 rows is refused** ([`LoadError::TooManyRows`])
//!   rather than given a wider index.
//! - **Names stay on this side.** The tables keep each table's names for
//!   lookup; no lane holds a name.
//! - **Nothing is dropped silently.** Files the reader rejects, names declared
//!   twice and references to undeclared names are reported
//!   ([`OgitTables::rejected`], [`OgitTables::duplicates`],
//!   [`OgitTables::unresolved`]).
//!
//! With the `ontology-cache` feature, `load_registry` is the Ontology cache
//! itself: lance-graph-ontology's `OntologyRegistry`, loaded from the same tree
//! and used as it is. It holds identity only, and only for names with a
//! domain; these tables hold the rest. The plan is bardioc's
//! `ONTOLOGY_CACHE_INTEGRATION_PLAN.md`.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ogar_from_schema::sgo::parse_verb;
use ogar_from_schema::ttl::parse_file;
use ogar_from_schema::{EntityDecl, TtlDeclaration};

/// The most rows a `u16` row address can reach.
pub const MAX_ROWS: usize = 1 << 16;

/// One of the five tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Table {
    /// Entity types, the SGO core included.
    Type,
    /// Declared attributes.
    Attribute,
    /// The attributes each type declares.
    TypeAttr,
    /// Verbs.
    Verb,
    /// `ogit:allowed` tuples.
    Allowed,
}

/// A column, lent as `&[u32]` by [`OgitTables::lane`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lane {
    /// [`Table::Type`]: the type's domain.
    TypeDomain,
    /// [`Table::Attribute`]: the attribute's domain.
    AttributeDomain,
    /// [`Table::Verb`]: the verb's domain.
    VerbDomain,
    /// [`Table::TypeAttr`]: the declaring type.
    TypeAttrType,
    /// [`Table::TypeAttr`]: the declared attribute.
    TypeAttrAttr,
    /// [`Table::TypeAttr`]: the [`Flag`].
    TypeAttrFlag,
    /// [`Table::Allowed`]: the source type.
    AllowedSrc,
    /// [`Table::Allowed`]: the verb.
    AllowedVerb,
    /// [`Table::Allowed`]: the target type.
    AllowedDst,
}

impl Lane {
    /// The table the lane belongs to.
    #[must_use]
    pub fn table(self) -> Table {
        match self {
            Lane::TypeDomain => Table::Type,
            Lane::AttributeDomain => Table::Attribute,
            Lane::VerbDomain => Table::Verb,
            Lane::TypeAttrType | Lane::TypeAttrAttr | Lane::TypeAttrFlag => Table::TypeAttr,
            Lane::AllowedSrc | Lane::AllowedVerb | Lane::AllowedDst => Table::Allowed,
        }
    }
}

/// How a type declares an attribute: the value in [`Lane::TypeAttrFlag`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Flag {
    /// `ogit:mandatory-attributes`.
    Mandatory = 0,
    /// `ogit:optional-attributes`.
    Optional = 1,
    /// `ogit:indexed-attributes`.
    Indexed = 2,
}

/// A row of [`Table::Type`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u16);

/// A row of [`Table::Attribute`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttrId(pub u16);

/// A row of [`Table::Verb`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VerbId(pub u16);

/// A reference a type makes to a name the tree does not declare.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unresolved {
    /// The table the reference would have been a row of.
    pub table: Table,
    /// The type that makes it.
    pub from: String,
    /// The name, as written.
    pub name: String,
}

/// Why [`OgitTables::load`] failed.
#[derive(Debug)]
pub enum LoadError {
    /// The tree could not be read.
    Io(io::Error),
    /// A table would need more rows than a `u16` can address.
    TooManyRows {
        /// The table.
        table: Table,
        /// The rows it would need.
        rows: usize,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "the OGIT tree does not read: {e}"),
            LoadError::TooManyRows { table, rows } => write!(
                f,
                "{table:?} would need {rows} rows, past the {MAX_ROWS} a u16 address reaches"
            ),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io(e) => Some(e),
            LoadError::TooManyRows { .. } => None,
        }
    }
}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> Self {
        LoadError::Io(e)
    }
}

/// One table's names, in row order, and the one index over them: the rows
/// sorted by name, a permutation.
#[derive(Debug, Default, PartialEq, Eq)]
struct Names {
    names: Vec<String>,
    by_name: Vec<u16>,
}

impl Names {
    fn new(names: Vec<String>) -> Self {
        let mut by_name: Vec<u16> = (0..names.len()).map(row).collect();
        by_name.sort_by(|&a, &b| names[usize::from(a)].cmp(&names[usize::from(b)]));
        Self { names, by_name }
    }

    fn find(&self, name: &str) -> Option<u16> {
        self.by_name
            .binary_search_by(|&r| self.names[usize::from(r)].as_str().cmp(name))
            .ok()
            .map(|i| self.by_name[i])
    }

    fn get(&self, r: u16) -> Option<&str> {
        self.names.get(usize::from(r)).map(String::as_str)
    }
}

/// The five OGIT tables, read once from an OGIT tree.
#[derive(Debug, PartialEq, Eq)]
pub struct OgitTables {
    domains: Vec<String>,
    types: Names,
    attributes: Names,
    verbs: Names,
    type_domain: Vec<u32>,
    attribute_domain: Vec<u32>,
    verb_domain: Vec<u32>,
    type_attr_type: Vec<u32>,
    type_attr_attr: Vec<u32>,
    type_attr_flag: Vec<u32>,
    allowed_src: Vec<u32>,
    allowed_verb: Vec<u32>,
    allowed_dst: Vec<u32>,
    rejected: Vec<String>,
    duplicates: Vec<String>,
    unresolved: Vec<Unresolved>,
}

impl OgitTables {
    /// Reads the OGIT tree at `root`, the directory holding `NTO/` and `SGO/`.
    ///
    /// A file declares what its directory says: `entities/`, `verbs/` or
    /// `attributes/`, in any letter case. Other `.ttl` files are not read.
    ///
    /// # Errors
    /// [`LoadError::Io`] if the tree does not read, and
    /// [`LoadError::TooManyRows`] if a table would pass [`MAX_ROWS`].
    pub fn load(root: &Path) -> Result<Self, LoadError> {
        let mut entities: Vec<EntityDecl> = Vec::new();
        let mut attributes: Vec<String> = Vec::new();
        let mut verbs: Vec<String> = Vec::new();
        let mut rejected = Vec::new();
        for path in ttl_files(root)? {
            let Some(kind) = kind_of(&path) else {
                continue;
            };
            let src = fs::read_to_string(&path)?;
            let accepted = match kind {
                Kind::Verb => match parse_verb(&src) {
                    Some(v) => {
                        verbs.push(v.curie);
                        true
                    }
                    None => false,
                },
                Kind::Entity | Kind::Attribute => match (kind, parse_file(&src)) {
                    (Kind::Entity, Some(TtlDeclaration::Entity(e))) => {
                        entities.push(e);
                        true
                    }
                    (Kind::Attribute, Some(TtlDeclaration::DatatypeAttribute(a))) => {
                        attributes.push(a.curie);
                        true
                    }
                    _ => false,
                },
            };
            if !accepted {
                rejected.push(relative(root, &path));
            }
        }

        // Sorted by (domain, name). The sort is stable, so a name declared
        // twice keeps the first file in path order.
        let mut duplicates = Vec::new();
        entities.sort_by(|a, b| order(&a.curie).cmp(&order(&b.curie)));
        entities.dedup_by(|later, kept| {
            let twice = later.curie == kept.curie;
            if twice {
                duplicates.push(later.curie.clone());
            }
            twice
        });
        let attributes = sorted_unique(attributes, &mut duplicates);
        let verbs = sorted_unique(verbs, &mut duplicates);
        check(Table::Type, entities.len())?;
        check(Table::Attribute, attributes.len())?;
        check(Table::Verb, verbs.len())?;

        let domains: Vec<String> = entities
            .iter()
            .map(|e| e.curie.as_str())
            .chain(attributes.iter().map(String::as_str))
            .chain(verbs.iter().map(String::as_str))
            .map(domain_of)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let type_domain = domain_lane(&domains, entities.iter().map(|e| e.curie.as_str()));
        let attribute_domain = domain_lane(&domains, attributes.iter().map(String::as_str));
        let verb_domain = domain_lane(&domains, verbs.iter().map(String::as_str));
        let types = Names::new(entities.iter().map(|e| e.curie.clone()).collect());
        let attributes = Names::new(attributes);
        let verbs = Names::new(verbs);

        let mut unresolved = Vec::new();
        let (mut type_attr_type, mut type_attr_attr, mut type_attr_flag) =
            (Vec::new(), Vec::new(), Vec::new());
        let mut allowed: Vec<(u32, u32, u32)> = Vec::new();
        for (t, e) in entities.iter().enumerate() {
            let t = u32::from(row(t));
            let lists = [
                (Flag::Mandatory, &e.mandatory_attributes),
                (Flag::Optional, &e.optional_attributes),
                (Flag::Indexed, &e.indexed_attributes),
            ];
            for (flag, list) in lists {
                for name in list {
                    match attributes.find(name) {
                        Some(a) => {
                            type_attr_type.push(t);
                            type_attr_attr.push(u32::from(a));
                            type_attr_flag.push(flag as u32);
                        }
                        None => unresolved.push(Unresolved {
                            table: Table::TypeAttr,
                            from: e.curie.clone(),
                            name: name.clone(),
                        }),
                    }
                }
            }
            for (verb, target) in &e.allowed {
                let v = verbs.find(verb);
                let d = types.find(target);
                for (found, name) in [(v.is_some(), verb), (d.is_some(), target)] {
                    if !found {
                        unresolved.push(Unresolved {
                            table: Table::Allowed,
                            from: e.curie.clone(),
                            name: name.clone(),
                        });
                    }
                }
                if let (Some(v), Some(d)) = (v, d) {
                    allowed.push((t, u32::from(v), u32::from(d)));
                }
            }
        }
        allowed.sort_unstable();
        allowed.dedup();
        check(Table::TypeAttr, type_attr_type.len())?;
        check(Table::Allowed, allowed.len())?;

        Ok(OgitTables {
            domains,
            types,
            attributes,
            verbs,
            type_domain,
            attribute_domain,
            verb_domain,
            type_attr_type,
            type_attr_attr,
            type_attr_flag,
            allowed_src: allowed.iter().map(|a| a.0).collect(),
            allowed_verb: allowed.iter().map(|a| a.1).collect(),
            allowed_dst: allowed.iter().map(|a| a.2).collect(),
            rejected,
            duplicates,
            unresolved,
        })
    }

    /// How many rows `table` has.
    #[must_use]
    pub fn rows(&self, table: Table) -> usize {
        match table {
            Table::Type => self.types.names.len(),
            Table::Attribute => self.attributes.names.len(),
            Table::TypeAttr => self.type_attr_type.len(),
            Table::Verb => self.verbs.names.len(),
            Table::Allowed => self.allowed_src.len(),
        }
    }

    /// A column, lent without a copy: one value per row of its table.
    #[must_use]
    pub fn lane(&self, lane: Lane) -> &[u32] {
        match lane {
            Lane::TypeDomain => &self.type_domain,
            Lane::AttributeDomain => &self.attribute_domain,
            Lane::VerbDomain => &self.verb_domain,
            Lane::TypeAttrType => &self.type_attr_type,
            Lane::TypeAttrAttr => &self.type_attr_attr,
            Lane::TypeAttrFlag => &self.type_attr_flag,
            Lane::AllowedSrc => &self.allowed_src,
            Lane::AllowedVerb => &self.allowed_verb,
            Lane::AllowedDst => &self.allowed_dst,
        }
    }

    /// The domain names, sorted; a domain lane holds an index into this. The
    /// SGO core is the empty name.
    #[must_use]
    pub fn domains(&self) -> &[String] {
        &self.domains
    }

    /// A domain's index (`MARS`, or `""` for the SGO core).
    #[must_use]
    pub fn domain_id(&self, name: &str) -> Option<u32> {
        let i = self
            .domains
            .binary_search_by(|d| d.as_str().cmp(name))
            .ok()?;
        u32::try_from(i).ok()
    }

    /// A type by its CURIE (`ogit:Person`, `ogit.MARS:Machine`).
    #[must_use]
    pub fn type_id(&self, curie: &str) -> Option<TypeId> {
        self.types.find(curie).map(TypeId)
    }

    /// An attribute by its CURIE (`ogit:name`).
    #[must_use]
    pub fn attribute_id(&self, curie: &str) -> Option<AttrId> {
        self.attributes.find(curie).map(AttrId)
    }

    /// A verb by its CURIE (`ogit:dependsOn`).
    #[must_use]
    pub fn verb_id(&self, curie: &str) -> Option<VerbId> {
        self.verbs.find(curie).map(VerbId)
    }

    /// A type's CURIE.
    #[must_use]
    pub fn type_name(&self, id: TypeId) -> Option<&str> {
        self.types.get(id.0)
    }

    /// An attribute's CURIE.
    #[must_use]
    pub fn attribute_name(&self, id: AttrId) -> Option<&str> {
        self.attributes.get(id.0)
    }

    /// A verb's CURIE.
    #[must_use]
    pub fn verb_name(&self, id: VerbId) -> Option<&str> {
        self.verbs.get(id.0)
    }

    /// Files the reader did not accept, as paths inside the tree.
    #[must_use]
    pub fn rejected(&self) -> &[String] {
        &self.rejected
    }

    /// Names declared by more than one file; each keeps its first file.
    #[must_use]
    pub fn duplicates(&self) -> &[String] {
        &self.duplicates
    }

    /// References to names the tree does not declare. They have no row.
    #[must_use]
    pub fn unresolved(&self) -> &[Unresolved] {
        &self.unresolved
    }
}

/// The Ontology cache: lance-graph-ontology's `OntologyRegistry`, used as it
/// is.
#[cfg(feature = "ontology-cache")]
pub use lance_graph_ontology::OntologyRegistry;

/// Loads the OGIT tree at `root` into a new in-memory registry: the Ontology
/// cache.
///
/// It keeps only names with a domain (`ogit.<Domain>:<Name>`), so the SGO core
/// is not in it, and it keeps identity without attribute lists or
/// `ogit:allowed` tuples. [`OgitTables`] holds those.
///
/// # Errors
/// If the tree cannot be read, or yields nothing to load.
#[cfg(feature = "ontology-cache")]
pub fn load_registry(root: &Path) -> Result<OntologyRegistry, lance_graph_ontology::Error> {
    let registry = OntologyRegistry::new_in_memory();
    registry.hydrate_once_sync(root, &[])?;
    Ok(registry)
}

/// Every entity name `registry` holds, sorted and without repeats.
#[cfg(feature = "ontology-cache")]
#[must_use]
pub fn entity_curies(registry: &OntologyRegistry) -> Vec<String> {
    use lance_graph_ontology::namespace::SchemaKind;
    let mut out: Vec<String> = registry
        .namespace_names()
        .iter()
        .flat_map(|ns| registry.enumerate(ns))
        .filter(|r| r.kind == SchemaKind::Entity)
        .map(|r| r.ogit_uri.into_string())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// What a file's directory says it declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Entity,
    Verb,
    Attribute,
}

fn kind_of(path: &Path) -> Option<Kind> {
    let dir = path.parent()?.file_name()?.to_str()?;
    [
        ("entities", Kind::Entity),
        ("verbs", Kind::Verb),
        ("attributes", Kind::Attribute),
    ]
    .into_iter()
    .find(|(name, _)| dir.eq_ignore_ascii_case(name))
    .map(|(_, kind)| kind)
}

/// Every `.ttl` file under `root`, sorted, skipping hidden directories.
fn ttl_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            let hidden = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'));
            if hidden {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "ttl") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The domain of a CURIE: `MARS` for `ogit.MARS:Machine`, empty for the SGO
/// core (`ogit:Person`).
fn domain_of(curie: &str) -> &str {
    curie
        .strip_prefix("ogit.")
        .and_then(|rest| rest.split_once(':'))
        .map_or("", |(domain, _)| domain)
}

/// The row order of the type, attribute and verb tables.
fn order(curie: &str) -> (&str, &str) {
    (domain_of(curie), curie)
}

/// Each name's domain, as an index into the sorted `domains`.
fn domain_lane<'a>(domains: &[String], names: impl Iterator<Item = &'a str>) -> Vec<u32> {
    names
        .map(|n| {
            let i = domains
                .binary_search_by(|d| d.as_str().cmp(domain_of(n)))
                .expect("every domain was collected");
            u32::try_from(i).expect("fewer domains than names")
        })
        .collect()
}

/// `names` sorted by [`order`], each kept once; repeats go to `duplicates`.
fn sorted_unique(mut names: Vec<String>, duplicates: &mut Vec<String>) -> Vec<String> {
    names.sort_by(|a, b| order(a).cmp(&order(b)));
    names.dedup_by(|later, kept| {
        let twice = later == kept;
        if twice {
            duplicates.push(later.clone());
        }
        twice
    });
    names
}

fn check(table: Table, rows: usize) -> Result<(), LoadError> {
    if rows > MAX_ROWS {
        Err(LoadError::TooManyRows { table, rows })
    } else {
        Ok(())
    }
}

/// A row index as its `u16` address. Tables are checked against
/// [`MAX_ROWS`] before any row is addressed.
fn row(i: usize) -> u16 {
    u16::try_from(i).expect("rows are checked against MAX_ROWS first")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_past_the_u16_address_is_refused() {
        assert!(check(Table::Type, MAX_ROWS).is_ok());
        let err = check(Table::TypeAttr, MAX_ROWS + 1).unwrap_err();
        assert!(matches!(
            err,
            LoadError::TooManyRows {
                table: Table::TypeAttr,
                rows
            } if rows == MAX_ROWS + 1
        ));
        assert_eq!(row(MAX_ROWS - 1), u16::MAX);
    }

    #[test]
    fn domains_come_from_the_curie() {
        assert_eq!(domain_of("ogit.MARS:Machine"), "MARS");
        assert_eq!(domain_of("ogit.MARS.Application:class"), "MARS.Application");
        assert_eq!(domain_of("ogit:Person"), "");
        assert_eq!(domain_of("not-a-curie"), "");
    }

    #[test]
    fn directories_match_in_any_case() {
        assert_eq!(
            kind_of(Path::new("NTO/Credit/Entities/Contract.ttl")),
            Some(Kind::Entity)
        );
        assert_eq!(
            kind_of(Path::new("SGO/sgo/verbs/dependsOn.ttl")),
            Some(Kind::Verb)
        );
        assert_eq!(
            kind_of(Path::new("NTO/MARS/attributes/x.ttl")),
            Some(Kind::Attribute)
        );
        assert_eq!(kind_of(Path::new("ogit.ttl")), None);
    }

    #[test]
    fn names_are_found_through_the_sorted_index() {
        let n = Names::new(vec!["ogit.B:x".into(), "ogit:a".into(), "ogit.A:y".into()]);
        assert_eq!(n.find("ogit:a"), Some(1));
        assert_eq!(n.find("ogit.A:y"), Some(2));
        assert_eq!(n.find("ogit:zzz"), None);
        assert_eq!(n.get(0), Some("ogit.B:x"));
    }
}
