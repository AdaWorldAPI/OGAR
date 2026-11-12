//! The OGIT tables HIRO validates against, and the Ontology cache.
//!
//! [`OgitTables`] reads an OGIT tree once, through OGAR's own reader
//! ([`parse_all`], [`parse_verb`]), into five tables. Every row is addressed
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
//!   A different tree can give the same id a different meaning, so an id kept
//!   outside this process must be stored with [`OgitTables::tree`], and read
//!   back only against tables whose tree is the same.
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
use ogar_from_schema::ttl::parse_all;
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

/// Which OGIT tree a set of tables was read from: the git commit its
/// checkout is at, when that can be read.
///
/// It names the commit, not the working tree: uncommitted edits to the files
/// do not change it. It is read from `.git` without running git, so a tree
/// that is not a checkout (an export, a tarball) has none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TreeId {
    commit: Option<String>,
}

impl TreeId {
    /// The tree at `root`, which is unknown rather than an error when its
    /// commit cannot be read.
    #[must_use]
    pub fn of(root: &Path) -> Self {
        Self {
            commit: checkout_commit(root),
        }
    }

    /// The checkout's commit as lowercase hex, if it was read.
    #[must_use]
    pub fn commit(&self) -> Option<&str> {
        self.commit.as_deref()
    }
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
    tree: TreeId,
}

impl OgitTables {
    /// Reads the OGIT tree at `root`, the directory holding `NTO/` and `SGO/`.
    ///
    /// A file declares what its directory says: `entities/`, `verbs/` or
    /// `attributes/`, in any letter case. Other `.ttl` files are not read.
    /// An `entities/` file's attribute declarations are read too; every
    /// subject in a file counts.
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
                Kind::Entity | Kind::Attribute => {
                    // A file may declare more than one subject; an entity
                    // file can carry its own attributes.
                    let mut any = false;
                    for decl in parse_all(&src) {
                        match (kind, decl) {
                            (Kind::Entity, TtlDeclaration::Entity(e)) => entities.push(e),
                            (_, TtlDeclaration::DatatypeAttribute(a)) => attributes.push(a.curie),
                            _ => continue,
                        }
                        any = true;
                    }
                    any
                }
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
            tree: TreeId::of(root),
        })
    }

    /// The tree these tables were read from. Ids are row ordinals of that
    /// tree: store it with any id kept outside this process.
    #[must_use]
    pub fn tree(&self) -> &TreeId {
        &self.tree
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

/// The commit a checkout at `root` is on, from `.git` alone: `HEAD`, then the
/// ref it names in the gitdir, the common dir (a worktree) or `packed-refs`.
/// `.git` may be a file pointing at the gitdir (a submodule or worktree).
fn checkout_commit(root: &Path) -> Option<String> {
    let dot_git = root.join(".git");
    let gitdir = if dot_git.is_file() {
        let text = fs::read_to_string(&dot_git).ok()?;
        root.join(text.trim().strip_prefix("gitdir:")?.trim())
    } else {
        dot_git
    };
    let text = fs::read_to_string(gitdir.join("HEAD")).ok()?;
    let text = text.trim();
    let Some(name) = text.strip_prefix("ref:").map(str::trim) else {
        return hex_commit(text);
    };
    // A worktree keeps its refs in the common dir its gitdir points at.
    let common = fs::read_to_string(gitdir.join("commondir"))
        .ok()
        .map(|c| gitdir.join(c.trim()));
    [Some(&gitdir), common.as_ref()]
        .into_iter()
        .flatten()
        .find_map(|dir| {
            let loose = fs::read_to_string(dir.join(name)).ok();
            let packed = || {
                let packed = fs::read_to_string(dir.join("packed-refs")).ok()?;
                packed.lines().find_map(|l| {
                    let (sha, r) = l.split_once(' ')?;
                    (r == name).then(|| sha.to_owned())
                })
            };
            hex_commit(loose.or_else(packed)?.trim())
        })
}

/// `s` as a commit id (40 or 64 hex digits), lowercased.
fn hex_commit(s: &str) -> Option<String> {
    ((s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| s.to_ascii_lowercase())
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

    /// A scratch directory, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("ogit-tree-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn write(&self, path: &str, text: &str) {
            let file = self.0.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, text).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const OTHER: &str = "fedcba9876543210fedcba9876543210fedcba98";

    #[test]
    fn a_detached_head_is_the_commit() {
        let s = Scratch::new("detached");
        s.write(".git/HEAD", &format!("{}\n", SHA.to_uppercase()));
        assert_eq!(TreeId::of(&s.0).commit(), Some(SHA));
    }

    #[test]
    fn a_head_ref_is_resolved_loose_then_packed() {
        let s = Scratch::new("ref");
        s.write(".git/HEAD", "ref: refs/heads/main\n");
        s.write(
            ".git/packed-refs",
            &format!("# pack-refs\n{OTHER} refs/heads/other\n{SHA} refs/heads/main\n"),
        );
        assert_eq!(TreeId::of(&s.0).commit(), Some(SHA));
        s.write(".git/refs/heads/main", &format!("{OTHER}\n"));
        assert_eq!(TreeId::of(&s.0).commit(), Some(OTHER), "a loose ref wins");
    }

    #[test]
    fn a_git_file_points_at_the_gitdir_and_a_worktree_at_its_common_dir() {
        let s = Scratch::new("gitfile");
        s.write("tree/.git", "gitdir: ../meta/worktrees/w\n");
        s.write("meta/worktrees/w/HEAD", "ref: refs/heads/topic\n");
        s.write("meta/worktrees/w/commondir", "../..\n");
        s.write("meta/refs/heads/topic", &format!("{SHA}\n"));
        assert_eq!(TreeId::of(&s.0.join("tree")).commit(), Some(SHA));
    }

    #[test]
    fn an_unreadable_commit_is_unknown_not_an_error() {
        let s = Scratch::new("unknown");
        assert_eq!(TreeId::of(&s.0), TreeId::default(), "no .git");
        s.write(".git/HEAD", "ref: refs/heads/gone\n");
        assert_eq!(TreeId::of(&s.0).commit(), None, "ref with no file");
        s.write(".git/HEAD", "not a commit\n");
        assert_eq!(TreeId::of(&s.0).commit(), None);
        s.write(".git/HEAD", &format!("{}zz\n", &SHA[..38]));
        assert_eq!(TreeId::of(&s.0).commit(), None, "right length, not hex");
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
