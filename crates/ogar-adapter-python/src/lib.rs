//! `ogar-adapter-python` — materializes the OGAR capability surface as a
//! self-contained generated Python module for foreign consumers.
//!
//! Companion to `ogar-adapter-csharp`; together they give a non-Rust
//! consumer the same "hot-plug" plug-and-play pattern documented in
//! `.claude/knowledge/hotplug-consumer-migration.md` — without linking
//! against `ogar-vocab` at all. [`emit_python`] renders ONE Python file
//! (compile-time artifact, zero runtime serialization) carrying:
//!
//! 1. **`CLASS_IDS`** — every [`ogar_vocab::class_ids::ALL`] entry, plus
//!    `compose_render_classid` / `concept_of` / `app_of` (the canon-high
//!    classid composition helpers mirroring [`ogar_vocab::app`]).
//! 2. **Domain action tables** — every
//!    [`ogar_vocab::capability_registry::domain_tables`] entry
//!    generically (so a future domain lands on regeneration with zero
//!    code change here), plus the FULL OCR specs from
//!    [`ogar_vocab::ocr_actions::ocr_actions`] (capability, subject
//!    concept + classid, typed params with mandatory flags, produces).
//! 3. **`HotPlug` + `resolve_hotplug`** — a pure Python mirror of
//!    [`ogar_vocab::capability_registry::resolve_hotplug`], same five
//!    drift arms (`UnknownClassid` / `NoCapabilitiesFor` /
//!    `UnexpectedConsumer` / `Uncovered` / `Undeclared`), same check
//!    order.
//! 4. **`Facet`** — the V3 4+12 content-blind facet read side: 16 bytes
//!    -> `(classid u32 LE, six (lo, hi) byte pairs)`. See OGAR
//!    `CLAUDE.md` P0 "THE CANONICAL GUID" / `E-V3-FACET-4-PLUS-12`, and
//!    `ruff_spo_address::Facet` (the producer-side Rust twin this
//!    mirrors byte-for-byte: `facet_classid(4 LE) | 6 x (lo:hi)`).
//!
//! Output is deterministic: the same `ogar-vocab` state always renders
//! the same bytes (stable ordering throughout the emitted tables). See
//! [`ground_truth`] for the canonical verification-dump format used to
//! prove parity against the live `ogar-vocab` data (shared with
//! `ogar-adapter-csharp`'s own verification loop via a `[dev-dependencies]`
//! path dependency on this crate — ONE comparison function, not two).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ogar_vocab::capability_registry::domain_tables;
use ogar_vocab::class_ids;
use ogar_vocab::ocr_actions::ocr_actions;

// ─────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────

/// Render the self-contained generated Python module.
///
/// `module_name` is documentation-only (embedded in the header
/// docstring) — Python module identity comes from the file's name /
/// placement on `sys.path`, not from any content inside the file.
///
/// # Determinism
///
/// Calling this twice against the same `ogar-vocab` state yields
/// byte-identical output — every table is emitted in the source data's
/// own stable order ([`ogar_vocab::class_ids::ALL`]'s declaration
/// order, [`ocr_actions`]'s table order, [`domain_tables`]'s
/// registration order), with no hashmap-iteration nondeterminism.
#[must_use]
pub fn emit_python(module_name: &str) -> String {
    let mut out = PY_TEMPLATE.replace("@@MODULE_NAME@@", module_name);
    out = out.replace("@@CLASS_IDS@@", &py_class_ids_block());
    out = out.replace("@@OCR_ACTIONS@@", &py_ocr_actions_block());
    out = out.replace(
        "@@OCR_EXPECTED_EXECUTORS@@",
        &py_str_tuple(ogar_vocab::ocr_actions::OCR_EXPECTED_EXECUTORS),
    );
    out = out.replace(
        "@@OCR_SUBJECT_CLASSIDS@@",
        &py_hex_tuple(ogar_vocab::ocr_actions::OCR_SUBJECT_CLASSIDS),
    );
    out = out.replace("@@DOMAIN_TABLES@@", &py_domain_tables_block());
    out
}

// ─────────────────────────────────────────────────────────────────────
// Dynamic block builders — each pulls straight from live `ogar_vocab`
// data, never a hand-transcribed copy.
// ─────────────────────────────────────────────────────────────────────

fn py_class_ids_block() -> String {
    let mut s = String::new();
    for &(name, id) in class_ids::ALL {
        s.push_str(&format!("    {name:?}: 0x{id:04X},\n"));
    }
    s
}

fn py_ocr_actions_block() -> String {
    let mut s = String::new();
    for spec in ocr_actions() {
        let subject_concept_name = spec
            .def
            .object_class
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
        let subject_classid =
            ogar_vocab::canonical_concept_id(&subject_concept_name).unwrap_or_default();
        let params: Vec<String> = spec
            .params
            .iter()
            .map(|p| format!("ActionParam({:?}, {})", p.name, py_bool(p.mandatory)))
            .collect();
        let produces: Vec<String> = spec.produces.iter().map(|p| format!("{p:?}")).collect();
        s.push_str(&format!(
            "    OcrActionSpec(\n        capability={:?},\n        subject_concept={subject_concept_name:?},\n        subject_classid=0x{subject_classid:04X},\n        params={},\n        produces={},\n    ),\n",
            spec.def.predicate,
            py_tuple(&params),
            py_tuple(&produces),
        ));
    }
    s
}

fn py_domain_tables_block() -> String {
    let mut s = String::new();
    for table in domain_tables() {
        let execs: Vec<String> = table
            .expected_executors
            .iter()
            .map(|e| format!("{e:?}"))
            .collect();
        let entries: Vec<String> = (table.entries)()
            .iter()
            .map(|(cap, id)| format!("({cap:?}, 0x{id:04X})"))
            .collect();
        s.push_str(&format!(
            "    DomainTable(\n        domain={:?},\n        expected_executors={},\n        entries={},\n    ),\n",
            table.domain,
            py_tuple(&execs),
            py_tuple(&entries),
        ));
    }
    s
}

/// Build a Python tuple literal from already-formatted element strings.
/// `()` for zero elements; `(x,)` / `(x, y,)` otherwise — the trailing
/// comma is always syntactically valid in Python and sidesteps the
/// single-element-tuple gotcha (`("x")` is a plain string, not a tuple).
fn py_tuple(elems: &[String]) -> String {
    if elems.is_empty() {
        "()".to_string()
    } else {
        format!("({},)", elems.join(", "))
    }
}

fn py_str_tuple(items: &[&str]) -> String {
    let elems: Vec<String> = items.iter().map(|s| format!("{s:?}")).collect();
    py_tuple(&elems)
}

fn py_hex_tuple(items: &[u16]) -> String {
    let elems: Vec<String> = items.iter().map(|id| format!("0x{id:04X}")).collect();
    py_tuple(&elems)
}

fn py_bool(b: bool) -> &'static str {
    if b { "True" } else { "False" }
}

// ─────────────────────────────────────────────────────────────────────
// The static Python template. `@@…@@` markers are the data seams filled
// in by `emit_python` above.
// ─────────────────────────────────────────────────────────────────────

const PY_TEMPLATE: &str = r##""""GENERATED by ogar-adapter-python -- do not edit; regenerate from ogar-vocab.

Module: @@MODULE_NAME@@

Materializes the OGAR capability surface for a foreign Python consumer:
canonical class ids, the domain action tables (OCR capability specs
today; any future `domain_tables()` entry lands here automatically on
regeneration), hot-plug resolution (`resolve_hotplug` mirror, same five
drift arms), and the V3 4+12 facet decoder (classid(4 LE) + 12-byte
payload as six (lo, hi) rails).

Canon: OGAR CLAUDE.md P0 "THE CANONICAL GUID" + E-V3-FACET-4-PLUS-12
(classid: u32 = [hi u16 CANON concept][lo u16 APP render prefix]).
Pattern: .claude/knowledge/hotplug-consumer-migration.md.
"""

from __future__ import annotations

import struct
from dataclasses import dataclass


# ============================================================
# 1. CLASS_IDS -- canonical concept name -> u16 codebook id
# ============================================================

CLASS_IDS: dict[str, int] = {
@@CLASS_IDS@@}

_ID_TO_NAME: dict[int, str] = {v: k for k, v in CLASS_IDS.items()}


def compose_render_classid(concept: int, app: int) -> int:
    """Compose a full render classid: ``(concept << 16) | app``.

    Canon (OGAR CLAUDE.md P0 "THE CANONICAL GUID", operator-locked
    2026-07-02): ``classid: u32 = [hi u16 CANON concept][lo u16 APP /
    render prefix]``. Mirrors ``ogar_vocab::app::render_classid``
    (parameter order flipped to concept-first per this module's contract).
    """
    return ((concept & 0xFFFF) << 16) | (app & 0xFFFF)


def concept_of(classid: int) -> int:
    """The CANON concept half (high u16) of a full classid."""
    return (classid >> 16) & 0xFFFF


def app_of(classid: int) -> int:
    """The APP / render-prefix half (low u16) of a full classid."""
    return classid & 0xFFFF


# ============================================================
# 2. Domain action tables
# ============================================================


@dataclass(frozen=True)
class ActionParam:
    """One parameter of a capability's typed I/O signature."""

    name: str
    mandatory: bool


@dataclass(frozen=True)
class OcrActionSpec:
    """One OCR capability -- mirrors `ogar_vocab::ocr_actions::OcrActionSpec`."""

    capability: str
    subject_concept: str
    subject_classid: int
    params: tuple[ActionParam, ...]
    produces: tuple[str, ...]


OCR_ACTIONS: tuple[OcrActionSpec, ...] = (
@@OCR_ACTIONS@@)

OCR_EXPECTED_EXECUTORS: tuple[str, ...] = @@OCR_EXPECTED_EXECUTORS@@

OCR_SUBJECT_CLASSIDS: tuple[int, ...] = @@OCR_SUBJECT_CLASSIDS@@

# Every OCR capability name, in table order -- matches
# `ogar_vocab::ocr_actions::OCR_ACTION_NAMES`. Used below as the
# "everything covered" scenario for the hot-plug verification dump.
OCR_EXPECTED_COVERED_ALL: tuple[str, ...] = tuple(a.capability for a in OCR_ACTIONS)


@dataclass(frozen=True)
class DomainTable:
    """One authoritative domain action table -- the generic hot-plug join
    surface. Mirrors `ogar_vocab::capability_registry::DomainTable`.
    """

    domain: str
    expected_executors: tuple[str, ...]
    entries: tuple[tuple[str, int], ...]


DOMAIN_TABLES: tuple[DomainTable, ...] = (
@@DOMAIN_TABLES@@)


# ============================================================
# 3. HotPlug + resolve_hotplug
# ============================================================


@dataclass(frozen=True)
class HotPlug:
    """A consumer's hot-plug declaration. Mirrors
    `lance_graph_contract::hotplug::HotPlug`.
    """

    consumer: str
    classids: tuple[int, ...]
    covered: tuple[str, ...]


class HotplugDrift(Exception):
    """Base class for every `resolve_hotplug` failure arm."""


class UnknownClassid(HotplugDrift):
    """A hot-plugged classid is not minted in the codebook."""

    def __init__(self, classid: int) -> None:
        self.classid = classid
        super().__init__(f"hot-plugged classid 0x{classid:04X} is not minted")


class NoCapabilitiesFor(HotplugDrift):
    """A hot-plugged classid resolves to no declared capability."""

    def __init__(self, classid: int) -> None:
        self.classid = classid
        super().__init__(
            f"classid 0x{classid:04X} resolves to no declared capability"
        )


class UnexpectedConsumer(HotplugDrift):
    """The consumer is not an expected executor for a contributing table."""

    def __init__(self, consumer: str) -> None:
        self.consumer = consumer
        super().__init__(f"consumer `{consumer}` is not an expected executor")


class Uncovered(HotplugDrift):
    """A declared capability has no consumer arm."""

    def __init__(self, capability: str) -> None:
        self.capability = capability
        super().__init__(f"declared capability `{capability}` has no consumer arm")


class Undeclared(HotplugDrift):
    """The consumer covers a capability the authority does not declare."""

    def __init__(self, capability: str) -> None:
        self.capability = capability
        super().__init__(f"consumer covers `{capability}` which is not declared")


def resolve_hotplug(
    consumer: str,
    classids: list[int] | tuple[int, ...],
    covered: list[str] | tuple[str, ...],
) -> tuple[list[tuple[str, int]], list[str]]:
    """Pure Python mirror of
    `ogar_vocab::capability_registry::resolve_hotplug`.

    Checks, in order: every classid minted (`UnknownClassid`); per
    contributing domain table, the consumer is an expected executor
    (`UnexpectedConsumer`); every hot-plugged classid contributes at
    least one capability (`NoCapabilitiesFor`); coverage both directions
    (`Uncovered` / `Undeclared`).
    """
    classids = list(classids)
    covered = list(covered)

    concepts: list[tuple[str, int]] = []
    for cid in classids:
        name = _ID_TO_NAME.get(cid)
        if name is None:
            raise UnknownClassid(cid)
        concepts.append((name, cid))

    # BTreeMap-equivalent: distinct classids, ascending, count of
    # contributing (capability, subject) rows seen so far.
    unique_sorted = sorted(set(classids))
    contributed: dict[int, int] = {cid: 0 for cid in unique_sorted}
    capabilities: list[str] = []

    for table in DOMAIN_TABLES:
        table_contributes = False
        for cap, subject in table.entries:
            if subject in contributed:
                contributed[subject] += 1
                table_contributes = True
                capabilities.append(cap)
        if table_contributes and consumer not in table.expected_executors:
            raise UnexpectedConsumer(consumer)

    for cid in unique_sorted:
        if contributed[cid] == 0:
            raise NoCapabilitiesFor(cid)

    capabilities = sorted(set(capabilities))

    covered_set = set(covered)
    for cap in capabilities:
        if cap not in covered_set:
            raise Uncovered(cap)

    capabilities_set = set(capabilities)
    for cap in covered:
        if cap not in capabilities_set:
            raise Undeclared(cap)

    return concepts, capabilities


# ============================================================
# 4. Facet -- the V3 4+12 content-blind facet, read side
# ============================================================

TIERS = 6


@dataclass(frozen=True)
class Facet:
    """The V3 4+12 content-blind facet: ``classid(4 LE) + 6x(lo:hi)``
    rails, byte-identical to ``ruff_spo_address::Facet`` /
    ``lance_graph_contract::facet::FacetCascade``.

    Canon: OGAR CLAUDE.md ``E-V3-FACET-4-PLUS-12`` -- the 12-byte payload
    is a content-blind byte register the classid's ClassView projects;
    here it is read back as six ``(lo, hi)`` byte pairs (``u8:u8``,
    NEVER widened to u16/u24).
    """

    classid: int
    is_a_chain: tuple[int, ...]
    part_of_chain: tuple[int, ...]

    @staticmethod
    def from_bytes(data: bytes) -> "Facet":
        """Decode 16 bytes -> ``(classid u32 LE, six (lo, hi) pairs)``."""
        if len(data) != 16:
            raise ValueError(f"facet must be exactly 16 bytes, got {len(data)}")
        (classid,) = struct.unpack_from("<I", data, 0)
        is_a = tuple(data[4 + 2 * t] for t in range(TIERS))
        part_of = tuple(data[5 + 2 * t] for t in range(TIERS))
        return Facet(classid, is_a, part_of)

    def to_bytes(self) -> bytes:
        """Encode back to the 16-byte wire form (round-trip inverse)."""
        out = bytearray(16)
        struct.pack_into("<I", out, 0, self.classid)
        for t in range(TIERS):
            out[4 + 2 * t] = self.is_a_chain[t]
            out[5 + 2 * t] = self.part_of_chain[t]
        return bytes(out)


# ============================================================
# Verification dump -- the canonical text format shared with the
# ogar-adapter-csharp emission. See ogar_adapter_python::ground_truth
# on the Rust side for the ground-truth generator + comparator (the
# SAME function verifies both this module's dump() and the C#
# emission's Dump()).
# ============================================================


def dump() -> str:
    """Render the canonical verification dump (see module doc)."""
    lines: list[str] = []

    lines.append("CLASS_IDS")
    for name, cid in sorted(CLASS_IDS.items(), key=lambda kv: kv[0]):
        lines.append(f"{name}={cid:04X}")
    lines.append("END_CLASS_IDS")

    lines.append("OCR_ACTIONS")
    for a in OCR_ACTIONS:
        params = ",".join(f"{p.name}:{1 if p.mandatory else 0}" for p in a.params)
        produces = ",".join(a.produces)
        lines.append(
            f"{a.capability}|{a.subject_concept}|{a.subject_classid:04X}|{params}|{produces}"
        )
    lines.append("END_OCR_ACTIONS")

    lines.append("DOMAIN_TABLES")
    for t in DOMAIN_TABLES:
        execs = ",".join(t.expected_executors)
        entries = ",".join(f"{cap}:{cid:04X}" for cap, cid in t.entries)
        lines.append(f"{t.domain}|{execs}|{entries}")
    lines.append("END_DOMAIN_TABLES")

    lines.append("HOTPLUG_GREEN")
    concepts, capabilities = resolve_hotplug(
        "tesseract-ogar", OCR_SUBJECT_CLASSIDS, OCR_EXPECTED_COVERED_ALL
    )
    concepts_str = ",".join(f"{n}:{cid:04X}" for n, cid in concepts)
    lines.append(f"CONCEPTS={concepts_str}")
    lines.append(f"CAPABILITIES={','.join(capabilities)}")
    lines.append("END_HOTPLUG_GREEN")

    lines.append("HOTPLUG_DRIFT")
    try:
        resolve_hotplug("tesseract-ogar", [0xFFFE], [])
        raise AssertionError("expected UnknownClassid")
    except UnknownClassid as e:
        lines.append(f"UnknownClassid={e.classid:04X}")

    try:
        # `visit` (0x0906): the table-less Health concept — the stable
        # "no capabilities" probe now that `patient` carries the healthcare
        # table (whose executor guard would fire first).
        resolve_hotplug("tesseract-ogar", [CLASS_IDS["visit"]], [])
        raise AssertionError("expected NoCapabilitiesFor")
    except NoCapabilitiesFor as e:
        lines.append(f"NoCapabilitiesFor={e.classid:04X}")

    try:
        resolve_hotplug("stranger", OCR_SUBJECT_CLASSIDS, OCR_EXPECTED_COVERED_ALL)
        raise AssertionError("expected UnexpectedConsumer")
    except UnexpectedConsumer as e:
        lines.append(f"UnexpectedConsumer={e.consumer}")

    missing = list(OCR_EXPECTED_COVERED_ALL)
    missing.pop()
    try:
        resolve_hotplug("tesseract-ogar", OCR_SUBJECT_CLASSIDS, missing)
        raise AssertionError("expected Uncovered")
    except Uncovered as e:
        lines.append(f"Uncovered={e.capability}")

    extra = list(OCR_EXPECTED_COVERED_ALL) + ["does_not_exist"]
    try:
        resolve_hotplug("tesseract-ogar", OCR_SUBJECT_CLASSIDS, extra)
        raise AssertionError("expected Undeclared")
    except Undeclared as e:
        lines.append(f"Undeclared={e.capability}")
    lines.append("END_HOTPLUG_DRIFT")

    lines.append("FACET")
    demo = Facet(
        classid=compose_render_classid(CLASS_IDS["commercial_document"], 0x0002),
        is_a_chain=(1, 2, 3, 4, 5, 6),
        part_of_chain=(0x11, 0x12, 0x13, 0x14, 0x15, 0x16),
    )
    encoded = demo.to_bytes()
    decoded = Facet.from_bytes(encoded)
    assert decoded == demo, "facet round-trip must be lossless"
    lines.append(f"CLASSID={decoded.classid:08X}")
    lines.append("IS_A=" + ",".join(f"{b:02X}" for b in decoded.is_a_chain))
    lines.append("PART_OF=" + ",".join(f"{b:02X}" for b in decoded.part_of_chain))
    lines.append("END_FACET")

    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    print(dump())
"##;

// ─────────────────────────────────────────────────────────────────────
// Ground truth — shared by BOTH this crate's and `ogar-adapter-csharp`'s
// integration tests. Not `#[cfg(test)]`-gated: `ogar-adapter-csharp`
// links this crate as a plain `[dev-dependencies]` path dependency
// purely to reuse [`ground_truth::expected_dump`] and
// [`ground_truth::assert_dump_matches`] — ONE comparison function, not
// two independently-maintained ones.
// ─────────────────────────────────────────────────────────────────────

/// Ground-truth generation + comparison for the verification dump both
/// the emitted Python `dump()` and the emitted C# `Dump()` must
/// reproduce byte-for-byte.
pub mod ground_truth {
    use ogar_vocab::capability_registry::{HotplugDrift, domain_tables, resolve_hotplug};
    use ogar_vocab::class_ids;
    use ogar_vocab::ocr_actions::{self, ocr_actions};

    type HotplugOk = (Vec<(&'static str, u16)>, Vec<String>);

    /// The canonical text-dump format both the emitted Python `dump()`
    /// and the emitted C# `Dump()` must reproduce byte-for-byte. Built
    /// directly from live `ogar_vocab` calls — never hand-transcribed —
    /// so drift between the authority and either foreign emission is
    /// caught here, not silently accepted.
    #[must_use]
    pub fn expected_dump() -> String {
        let mut out = String::new();

        out.push_str("CLASS_IDS\n");
        let mut sorted: Vec<(&str, u16)> = class_ids::ALL.to_vec();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (name, id) in &sorted {
            out.push_str(&format!("{name}={id:04X}\n"));
        }
        out.push_str("END_CLASS_IDS\n");

        out.push_str("OCR_ACTIONS\n");
        for spec in ocr_actions() {
            let subject_concept = spec.def.object_class.rsplit('/').next().unwrap_or_default();
            let subject_classid =
                ogar_vocab::canonical_concept_id(subject_concept).unwrap_or_default();
            let params = spec
                .params
                .iter()
                .map(|p| format!("{}:{}", p.name, u8::from(p.mandatory)))
                .collect::<Vec<_>>()
                .join(",");
            let produces = spec.produces.join(",");
            out.push_str(&format!(
                "{}|{subject_concept}|{subject_classid:04X}|{params}|{produces}\n",
                spec.def.predicate,
            ));
        }
        out.push_str("END_OCR_ACTIONS\n");

        out.push_str("DOMAIN_TABLES\n");
        for table in domain_tables() {
            let execs = table.expected_executors.join(",");
            let entries = (table.entries)()
                .iter()
                .map(|(c, id)| format!("{c}:{id:04X}"))
                .collect::<Vec<_>>()
                .join(",");
            out.push_str(&format!("{}|{execs}|{entries}\n", table.domain));
        }
        out.push_str("END_DOMAIN_TABLES\n");

        let ocr_ids = ocr_actions::OCR_SUBJECT_CLASSIDS;
        let all_covered = ocr_actions::OCR_ACTION_NAMES;

        out.push_str("HOTPLUG_GREEN\n");
        let (concepts, capabilities) = resolve_hotplug("tesseract-ogar", ocr_ids, all_covered)
            .expect("ground truth green resolve_hotplug must succeed");
        let concepts_str = concepts
            .iter()
            .map(|(n, id)| format!("{n}:{id:04X}"))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&format!("CONCEPTS={concepts_str}\n"));
        out.push_str(&format!("CAPABILITIES={}\n", capabilities.join(",")));
        out.push_str("END_HOTPLUG_GREEN\n");

        out.push_str("HOTPLUG_DRIFT\n");
        let r1 = resolve_hotplug("tesseract-ogar", &[0xFFFEu16], &[]);
        out.push_str(&format!(
            "UnknownClassid={}\n",
            expect_drift_hex(r1, "UnknownClassid")
        ));
        // `visit` (0x0906) is the deliberately table-less Health concept
        // (excluded from HEALTHCARE_SUBJECT_CLASSIDS) — no OCR and no
        // healthcare capabilities, so it is the stable "no capabilities"
        // probe. `patient` USED to work here but now carries the
        // healthcare table, so its executor guard fires before the
        // NoCapabilitiesFor check (matches capability_registry's own probe).
        let r2 = resolve_hotplug("tesseract-ogar", &[class_ids::VISIT], &[]);
        out.push_str(&format!(
            "NoCapabilitiesFor={}\n",
            expect_drift_hex(r2, "NoCapabilitiesFor")
        ));
        let r3 = resolve_hotplug("stranger", ocr_ids, all_covered);
        out.push_str(&format!(
            "UnexpectedConsumer={}\n",
            expect_drift_str(r3, "UnexpectedConsumer")
        ));
        let mut missing = all_covered.to_vec();
        missing.pop();
        let r4 = resolve_hotplug("tesseract-ogar", ocr_ids, &missing);
        out.push_str(&format!(
            "Uncovered={}\n",
            expect_drift_str(r4, "Uncovered")
        ));
        let mut extra = all_covered.to_vec();
        extra.push("does_not_exist");
        let r5 = resolve_hotplug("tesseract-ogar", ocr_ids, &extra);
        out.push_str(&format!(
            "Undeclared={}\n",
            expect_drift_str(r5, "Undeclared")
        ));
        out.push_str("END_HOTPLUG_DRIFT\n");

        let classid = ogar_vocab::app::render_classid(0x0002, class_ids::COMMERCIAL_DOCUMENT);
        let is_a: [u8; 6] = [1, 2, 3, 4, 5, 6];
        let part_of: [u8; 6] = [0x11, 0x12, 0x13, 0x14, 0x15, 0x16];
        out.push_str("FACET\n");
        out.push_str(&format!("CLASSID={classid:08X}\n"));
        out.push_str(&format!(
            "IS_A={}\n",
            is_a.iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(",")
        ));
        out.push_str(&format!(
            "PART_OF={}\n",
            part_of
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(",")
        ));
        out.push_str("END_FACET\n");

        out
    }

    fn expect_drift_hex(r: Result<HotplugOk, HotplugDrift>, want: &str) -> String {
        match (&r, want) {
            (Err(HotplugDrift::UnknownClassid(id)), "UnknownClassid") => format!("{id:04X}"),
            (Err(HotplugDrift::NoCapabilitiesFor(id)), "NoCapabilitiesFor") => {
                format!("{id:04X}")
            }
            _ => panic!("expected {want} drift, got {r:?}"),
        }
    }

    fn expect_drift_str(r: Result<HotplugOk, HotplugDrift>, want: &str) -> String {
        match (&r, want) {
            (Err(HotplugDrift::UnexpectedConsumer(c)), "UnexpectedConsumer") => c.clone(),
            (Err(HotplugDrift::Uncovered(c)), "Uncovered") => c.clone(),
            (Err(HotplugDrift::Undeclared(c)), "Undeclared") => c.clone(),
            _ => panic!("expected {want} drift, got {r:?}"),
        }
    }

    /// Compare a foreign-emitted dump (Python stdout, C# stdout) against
    /// the live `ogar-vocab` ground truth ([`expected_dump`]). Panics
    /// with a line-level diff on the first mismatch. `source` names the
    /// caller (`"python"` / `"csharp"`) for a readable panic message.
    ///
    /// Shared by both `ogar-adapter-python`'s and `ogar-adapter-csharp`'s
    /// integration tests — the ONE comparison function referenced in
    /// both verification loops.
    pub fn assert_dump_matches(source: &str, actual: &str) {
        let expected = expected_dump();
        // Trim trailing whitespace/newlines before splitting: subprocess
        // stdout capture (and a caller using `Console.WriteLine` /
        // `print()` instead of the newline-free `Write` / `end=""` form)
        // routinely adds or drops exactly one trailing newline, which is
        // not a real content mismatch.
        let expected_lines: Vec<&str> = expected.trim_end().lines().collect();
        let actual_lines: Vec<&str> = actual.trim_end().lines().collect();
        let n = expected_lines.len().min(actual_lines.len());
        for i in 0..n {
            assert_eq!(
                expected_lines[i],
                actual_lines[i],
                "{source}: dump mismatch at line {}",
                i + 1
            );
        }
        assert_eq!(
            expected_lines.len(),
            actual_lines.len(),
            "{source}: dump line count mismatch\n--- expected ---\n{expected}\n--- actual ---\n{actual}",
        );
    }
}
