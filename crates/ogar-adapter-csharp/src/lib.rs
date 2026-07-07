//! `ogar-adapter-csharp` — materializes the OGAR capability surface as a
//! self-contained generated C# class library for foreign consumers.
//!
//! Companion to `ogar-adapter-python`; together they give a non-Rust
//! consumer the same "hot-plug" plug-and-play pattern documented in
//! `.claude/knowledge/hotplug-consumer-migration.md` — without linking
//! against `ogar-vocab` at all. [`emit_csharp`] renders ONE `.cs` file
//! (compile-time artifact, zero runtime serialization) carrying:
//!
//! 1. **`ClassIds`** — every [`ogar_vocab::class_ids::ALL`] entry, plus
//!    `ComposeRenderClassid` / `ConceptOf` / `AppOf` (the canon-high
//!    classid composition helpers mirroring [`ogar_vocab::app`]).
//! 2. **Domain action tables** — every
//!    [`ogar_vocab::capability_registry::domain_tables`] entry
//!    generically (so a future domain lands on regeneration with zero
//!    code change here), plus the FULL OCR specs from
//!    [`ogar_vocab::ocr_actions::ocr_actions`] (capability, subject
//!    concept + classid, typed params with mandatory flags, produces).
//! 3. **`HotPlug` + `ResolveHotplug`** — a pure C# mirror of
//!    [`ogar_vocab::capability_registry::resolve_hotplug`], same five
//!    drift arms (`UnknownClassid` / `NoCapabilitiesFor` /
//!    `UnexpectedConsumer` / `Uncovered` / `Undeclared`), same check
//!    order.
//! 4. **`Facet`** — the V3 4+12 content-blind facet read side: 16 bytes
//!    -> `(classid u32 LE, six (lo, hi) byte pairs)`, decoded via
//!    `System.Buffers.Binary.BinaryPrimitives` over `Span<byte>` (pure
//!    BCL, no NuGet package). See OGAR `CLAUDE.md` P0 "THE CANONICAL
//!    GUID" / `E-V3-FACET-4-PLUS-12`, and `ruff_spo_address::Facet`
//!    (the producer-side Rust twin this mirrors byte-for-byte:
//!    `facet_classid(4 LE) | 6 x (lo:hi)`).
//!
//! Output is deterministic and self-contained: no `PackageReference`,
//! pure BCL, so the emitted file builds against `net8.0` offline.
//!
//! The emitted type is a plain library (`OgarCapabilitySurface` static
//! class + supporting records) with NO `Main` entry point — a
//! consumer's own `Program.cs` calls `OgarCapabilitySurface.Dump()` or
//! `ResolveHotplug(...)` directly, matching how `ogar-adapter-python`'s
//! module is meant to be imported, not executed standalone (though its
//! `if __name__ == "__main__":` guard is also provided for convenience).
//!
//! See `ogar_adapter_python::ground_truth` for the canonical
//! verification-dump format and the ONE comparison function
//! (`assert_dump_matches`) this crate's own tests reuse via a
//! `[dev-dependencies]` path dependency on `ogar-adapter-python`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use ogar_vocab::capability_registry::domain_tables;
use ogar_vocab::class_ids;
use ogar_vocab::ocr_actions::ocr_actions;

// ─────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────

/// Render the self-contained generated C# class library.
///
/// `namespace` becomes the emitted file's `namespace` declaration (a
/// real, functional parameter — unlike `ogar-adapter-python`'s
/// `module_name`, C# source genuinely declares its own namespace).
///
/// # Determinism
///
/// Calling this twice against the same `ogar-vocab` state yields
/// byte-identical output — every table is emitted in the source data's
/// own stable order ([`ogar_vocab::class_ids::ALL`]'s declaration
/// order, [`ocr_actions`]'s table order, [`domain_tables`]'s
/// registration order), with no hashmap-iteration nondeterminism.
#[must_use]
pub fn emit_csharp(namespace: &str) -> String {
    let mut out = CS_TEMPLATE.replace("@@NAMESPACE@@", namespace);
    out = out.replace("@@CLASS_IDS@@", &cs_class_ids_block());
    out = out.replace("@@OCR_ACTIONS@@", &cs_ocr_actions_block());
    out = out.replace(
        "@@OCR_EXPECTED_EXECUTORS@@",
        &cs_str_array_inline(ogar_vocab::ocr_actions::OCR_EXPECTED_EXECUTORS),
    );
    out = out.replace(
        "@@OCR_SUBJECT_CLASSIDS@@",
        &cs_hex_array_inline(ogar_vocab::ocr_actions::OCR_SUBJECT_CLASSIDS),
    );
    out = out.replace("@@DOMAIN_TABLES@@", &cs_domain_tables_block());
    out
}

// ─────────────────────────────────────────────────────────────────────
// Dynamic block builders — each pulls straight from live `ogar_vocab`
// data, never a hand-transcribed copy.
// ─────────────────────────────────────────────────────────────────────

fn cs_class_ids_block() -> String {
    let mut s = String::new();
    for &(name, id) in class_ids::ALL {
        s.push_str(&format!("        {{ {name:?}, 0x{id:04X} }},\n"));
    }
    s
}

fn cs_ocr_actions_block() -> String {
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
            .map(|p| format!("new ActionParam({:?}, {})", p.name, cs_bool(p.mandatory)))
            .collect();
        let produces: Vec<String> = spec.produces.iter().map(|p| format!("{p:?}")).collect();
        s.push_str(&format!(
            "        new OcrActionSpec(\n            Capability: {:?},\n            SubjectConcept: {subject_concept_name:?},\n            SubjectClassId: 0x{subject_classid:04X},\n            Params: new ActionParam[] {{ {} }},\n            Produces: new string[] {{ {} }}),\n",
            spec.def.predicate,
            params.join(", "),
            produces.join(", "),
        ));
    }
    s
}

fn cs_domain_tables_block() -> String {
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
            "        new DomainTable(\n            Domain: {:?},\n            ExpectedExecutors: new string[] {{ {} }},\n            Entries: new (string Capability, int SubjectClassId)[] {{ {} }}),\n",
            table.domain,
            execs.join(", "),
            entries.join(", "),
        ));
    }
    s
}

fn cs_str_array_inline(items: &[&str]) -> String {
    items
        .iter()
        .map(|s| format!("{s:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn cs_hex_array_inline(items: &[u16]) -> String {
    items
        .iter()
        .map(|id| format!("0x{id:04X}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn cs_bool(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

// ─────────────────────────────────────────────────────────────────────
// The static C# template. `@@…@@` markers are the data seams filled in
// by `emit_csharp` above.
// ─────────────────────────────────────────────────────────────────────

const CS_TEMPLATE: &str = r##"// GENERATED by ogar-adapter-csharp -- do not edit; regenerate from ogar-vocab.
//
// Materializes the OGAR capability surface for a foreign C# consumer:
// canonical class ids, the domain action tables (OCR capability specs
// today; any future `domain_tables()` entry lands here automatically on
// regeneration), hot-plug resolution (`ResolveHotplug` mirror, same five
// drift arms), and the V3 4+12 facet decoder (classid(4 LE) + 12-byte
// payload as six (lo, hi) rails).
//
// Canon: OGAR CLAUDE.md P0 "THE CANONICAL GUID" + E-V3-FACET-4-PLUS-12
// (classid: u32 = [hi u16 CANON concept][lo u16 APP render prefix]).
// Pattern: .claude/knowledge/hotplug-consumer-migration.md.

using System;
using System.Buffers.Binary;
using System.Collections.Generic;
using System.Linq;

namespace @@NAMESPACE@@;

/// <summary>One parameter of a capability's typed I/O signature.</summary>
public sealed record ActionParam(string Name, bool Mandatory);

/// <summary>
/// One OCR capability -- mirrors <c>ogar_vocab::ocr_actions::OcrActionSpec</c>.
/// </summary>
public sealed record OcrActionSpec(
    string Capability,
    string SubjectConcept,
    int SubjectClassId,
    ActionParam[] Params,
    string[] Produces);

/// <summary>
/// One authoritative domain action table -- the generic hot-plug join
/// surface. Mirrors <c>ogar_vocab::capability_registry::DomainTable</c>.
/// </summary>
public sealed record DomainTable(
    string Domain,
    string[] ExpectedExecutors,
    (string Capability, int SubjectClassId)[] Entries);

/// <summary>
/// A consumer's hot-plug declaration. Mirrors
/// <c>lance_graph_contract::hotplug::HotPlug</c>.
/// </summary>
public sealed record HotPlug(string Consumer, int[] ClassIds, string[] Covered);

/// <summary>Base class for every <see cref="OgarCapabilitySurface.ResolveHotplug"/> failure arm.</summary>
public abstract class HotplugDrift : Exception
{
    /// <summary>Construct with the formatted drift message.</summary>
    protected HotplugDrift(string message) : base(message) { }
}

/// <summary>A hot-plugged classid is not minted in the codebook.</summary>
public sealed class UnknownClassid : HotplugDrift
{
    /// <summary>The unminted classid.</summary>
    public int ClassId { get; }

    /// <summary>Construct from the offending classid.</summary>
    public UnknownClassid(int classId) : base($"hot-plugged classid 0x{classId:X4} is not minted")
        => ClassId = classId;
}

/// <summary>A hot-plugged classid resolves to no declared capability.</summary>
public sealed class NoCapabilitiesFor : HotplugDrift
{
    /// <summary>The classid with no declared capability.</summary>
    public int ClassId { get; }

    /// <summary>Construct from the offending classid.</summary>
    public NoCapabilitiesFor(int classId) : base($"classid 0x{classId:X4} resolves to no declared capability")
        => ClassId = classId;
}

/// <summary>The consumer is not an expected executor for a contributing table.</summary>
public sealed class UnexpectedConsumer : HotplugDrift
{
    /// <summary>The unexpected consumer name.</summary>
    public string Consumer { get; }

    /// <summary>Construct from the offending consumer name.</summary>
    public UnexpectedConsumer(string consumer) : base($"consumer `{consumer}` is not an expected executor")
        => Consumer = consumer;
}

/// <summary>A declared capability has no consumer arm.</summary>
public sealed class Uncovered : HotplugDrift
{
    /// <summary>The uncovered capability name.</summary>
    public string Capability { get; }

    /// <summary>Construct from the offending capability name.</summary>
    public Uncovered(string capability) : base($"declared capability `{capability}` has no consumer arm")
        => Capability = capability;
}

/// <summary>The consumer covers a capability the authority does not declare.</summary>
public sealed class Undeclared : HotplugDrift
{
    /// <summary>The undeclared capability name.</summary>
    public string Capability { get; }

    /// <summary>Construct from the offending capability name.</summary>
    public Undeclared(string capability) : base($"consumer covers `{capability}` which is not declared")
        => Capability = capability;
}

/// <summary>
/// The V3 4+12 content-blind facet: <c>classid(4 LE) + 6x(lo:hi)</c>
/// rails, byte-identical to <c>ruff_spo_address::Facet</c> /
/// <c>lance_graph_contract::facet::FacetCascade</c>.
///
/// Canon: OGAR CLAUDE.md <c>E-V3-FACET-4-PLUS-12</c> -- the 12-byte
/// payload is a content-blind byte register the classid's ClassView
/// projects; here it is read back as six <c>(lo, hi)</c> byte pairs
/// (<c>u8:u8</c>, NEVER widened to u16/u24).
/// </summary>
public sealed record Facet(uint ClassId, byte[] IsAChain, byte[] PartOfChain)
{
    /// <summary>The number of (part_of:is_a) cascade tiers (6).</summary>
    public const int Tiers = 6;

    /// <summary>Decode 16 bytes -> (classid u32 LE, six (lo, hi) pairs).</summary>
    public static Facet FromBytes(ReadOnlySpan<byte> data)
    {
        if (data.Length != 16)
            throw new ArgumentException($"facet must be exactly 16 bytes, got {data.Length}");
        uint classId = BinaryPrimitives.ReadUInt32LittleEndian(data.Slice(0, 4));
        var isA = new byte[Tiers];
        var partOf = new byte[Tiers];
        for (int t = 0; t < Tiers; t++)
        {
            isA[t] = data[4 + 2 * t];
            partOf[t] = data[5 + 2 * t];
        }
        return new Facet(classId, isA, partOf);
    }

    /// <summary>Encode back to the 16-byte wire form (round-trip inverse).</summary>
    public byte[] ToBytes()
    {
        var outBytes = new byte[16];
        BinaryPrimitives.WriteUInt32LittleEndian(outBytes.AsSpan(0, 4), ClassId);
        for (int t = 0; t < Tiers; t++)
        {
            outBytes[4 + 2 * t] = IsAChain[t];
            outBytes[5 + 2 * t] = PartOfChain[t];
        }
        return outBytes;
    }
}

/// <summary>
/// Materializes the OGAR capability surface: class ids, domain action
/// tables, hot-plug resolution, and the facet decoder. See the file
/// header for the full doc. No <c>Main</c> here by design -- a
/// consumer's own entry point calls into this static class.
/// </summary>
public static class OgarCapabilitySurface
{
    // ============================================================
    // 1. ClassIds -- canonical concept name -> u16 codebook id
    // ============================================================

    /// <summary>Every promoted OGAR concept, by canonical name.</summary>
    public static readonly IReadOnlyDictionary<string, int> ClassIds = new Dictionary<string, int>
    {
@@CLASS_IDS@@    };

    private static readonly IReadOnlyDictionary<int, string> IdToName =
        ClassIds.ToDictionary(kv => kv.Value, kv => kv.Key);

    /// <summary>
    /// Compose a full render classid: <c>(concept &lt;&lt; 16) | app</c>.
    /// Canon (OGAR CLAUDE.md P0 "THE CANONICAL GUID", operator-locked
    /// 2026-07-02): <c>classid: u32 = [hi u16 CANON concept][lo u16 APP
    /// / render prefix]</c>. Mirrors <c>ogar_vocab::app::render_classid</c>
    /// (parameter order flipped to concept-first per this module's contract).
    /// </summary>
    public static int ComposeRenderClassid(int concept, int app) =>
        ((concept & 0xFFFF) << 16) | (app & 0xFFFF);

    /// <summary>The CANON concept half (high u16) of a full classid.</summary>
    public static int ConceptOf(int classid) => (classid >> 16) & 0xFFFF;

    /// <summary>The APP / render-prefix half (low u16) of a full classid.</summary>
    public static int AppOf(int classid) => classid & 0xFFFF;

    // ============================================================
    // 2. Domain action tables
    // ============================================================

    /// <summary>The tesseract-rs OCR capability surface, FULL specs.</summary>
    public static readonly OcrActionSpec[] OcrActions =
    {
@@OCR_ACTIONS@@    };

    /// <summary>Executors the OCR table expects to register against it.</summary>
    public static readonly string[] OcrExpectedExecutors = { @@OCR_EXPECTED_EXECUTORS@@ };

    /// <summary>The distinct subject classids the OCR table binds.</summary>
    public static readonly int[] OcrSubjectClassIds = { @@OCR_SUBJECT_CLASSIDS@@ };

    /// <summary>
    /// Every OCR capability name, in table order -- matches
    /// <c>ogar_vocab::ocr_actions::OCR_ACTION_NAMES</c>. Used below as
    /// the "everything covered" scenario for the hot-plug verification
    /// dump.
    /// </summary>
    public static readonly string[] OcrExpectedCoveredAll =
        OcrActions.Select(a => a.Capability).ToArray();

    /// <summary>
    /// Every registered authoritative domain table -- the generic
    /// hot-plug join surface. Mirrors
    /// <c>ogar_vocab::capability_registry::domain_tables()</c>.
    /// </summary>
    public static readonly DomainTable[] DomainTables =
    {
@@DOMAIN_TABLES@@    };

    // ============================================================
    // 3. HotPlug + ResolveHotplug
    // ============================================================

    /// <summary>
    /// Pure C# mirror of
    /// <c>ogar_vocab::capability_registry::resolve_hotplug</c>.
    ///
    /// Checks, in order: every classid minted (<see cref="UnknownClassid"/>);
    /// per contributing domain table, the consumer is an expected
    /// executor (<see cref="UnexpectedConsumer"/>); every hot-plugged
    /// classid contributes at least one capability
    /// (<see cref="NoCapabilitiesFor"/>); coverage both directions
    /// (<see cref="Uncovered"/> / <see cref="Undeclared"/>).
    /// </summary>
    public static (List<(string Name, int ClassId)> Concepts, List<string> Capabilities) ResolveHotplug(
        string consumer, IReadOnlyList<int> classids, IReadOnlyList<string> covered)
    {
        var concepts = new List<(string, int)>();
        foreach (var cid in classids)
        {
            if (!IdToName.TryGetValue(cid, out var name))
                throw new UnknownClassid(cid);
            concepts.Add((name, cid));
        }

        // BTreeMap-equivalent: distinct classids, ascending, count of
        // contributing (capability, subject) rows seen so far.
        var uniqueSorted = classids.Distinct().OrderBy(x => x).ToList();
        var contributed = uniqueSorted.ToDictionary(id => id, _ => 0);
        var capabilities = new List<string>();

        foreach (var table in DomainTables)
        {
            bool tableContributes = false;
            foreach (var (cap, subject) in table.Entries)
            {
                if (contributed.ContainsKey(subject))
                {
                    contributed[subject]++;
                    tableContributes = true;
                    capabilities.Add(cap);
                }
            }
            if (tableContributes && !table.ExpectedExecutors.Contains(consumer))
                throw new UnexpectedConsumer(consumer);
        }

        foreach (var cid in uniqueSorted)
        {
            if (contributed[cid] == 0)
                throw new NoCapabilitiesFor(cid);
        }

        capabilities = capabilities.Distinct().OrderBy(c => c, StringComparer.Ordinal).ToList();

        var coveredSet = new HashSet<string>(covered);
        foreach (var cap in capabilities)
        {
            if (!coveredSet.Contains(cap))
                throw new Uncovered(cap);
        }

        var capabilitiesSet = new HashSet<string>(capabilities);
        foreach (var cap in covered)
        {
            if (!capabilitiesSet.Contains(cap))
                throw new Undeclared(cap);
        }

        return (concepts, capabilities);
    }

    // ============================================================
    // Verification dump -- the canonical text format shared with the
    // ogar-adapter-python emission. See ogar_adapter_python::ground_truth
    // on the Rust side for the ground-truth generator + comparator (the
    // SAME function verifies both this class's Dump() and the Python
    // emission's dump()).
    // ============================================================

    /// <summary>Render the canonical verification dump (see class doc).</summary>
    public static string Dump()
    {
        var lines = new List<string>();

        lines.Add("CLASS_IDS");
        foreach (var kv in ClassIds.OrderBy(kv => kv.Key, StringComparer.Ordinal))
            lines.Add($"{kv.Key}={kv.Value:X4}");
        lines.Add("END_CLASS_IDS");

        lines.Add("OCR_ACTIONS");
        foreach (var a in OcrActions)
        {
            var paramsStr = string.Join(",", a.Params.Select(p => $"{p.Name}:{(p.Mandatory ? 1 : 0)}"));
            var producesStr = string.Join(",", a.Produces);
            lines.Add($"{a.Capability}|{a.SubjectConcept}|{a.SubjectClassId:X4}|{paramsStr}|{producesStr}");
        }
        lines.Add("END_OCR_ACTIONS");

        lines.Add("DOMAIN_TABLES");
        foreach (var t in DomainTables)
        {
            var execs = string.Join(",", t.ExpectedExecutors);
            var entries = string.Join(",", t.Entries.Select(e => $"{e.Capability}:{e.SubjectClassId:X4}"));
            lines.Add($"{t.Domain}|{execs}|{entries}");
        }
        lines.Add("END_DOMAIN_TABLES");

        lines.Add("HOTPLUG_GREEN");
        var (concepts, capabilities) = ResolveHotplug("tesseract-ogar", OcrSubjectClassIds, OcrExpectedCoveredAll);
        lines.Add("CONCEPTS=" + string.Join(",", concepts.Select(c => $"{c.Name}:{c.ClassId:X4}")));
        lines.Add("CAPABILITIES=" + string.Join(",", capabilities));
        lines.Add("END_HOTPLUG_GREEN");

        lines.Add("HOTPLUG_DRIFT");
        try
        {
            ResolveHotplug("tesseract-ogar", new[] { 0xFFFE }, Array.Empty<string>());
            throw new Exception("expected UnknownClassid");
        }
        catch (UnknownClassid e) { lines.Add($"UnknownClassid={e.ClassId:X4}"); }

        try
        {
            ResolveHotplug("tesseract-ogar", new[] { ClassIds["patient"] }, Array.Empty<string>());
            throw new Exception("expected NoCapabilitiesFor");
        }
        catch (NoCapabilitiesFor e) { lines.Add($"NoCapabilitiesFor={e.ClassId:X4}"); }

        try
        {
            ResolveHotplug("stranger", OcrSubjectClassIds, OcrExpectedCoveredAll);
            throw new Exception("expected UnexpectedConsumer");
        }
        catch (UnexpectedConsumer e) { lines.Add($"UnexpectedConsumer={e.Consumer}"); }

        var missing = OcrExpectedCoveredAll.ToList();
        missing.RemoveAt(missing.Count - 1);
        try
        {
            ResolveHotplug("tesseract-ogar", OcrSubjectClassIds, missing);
            throw new Exception("expected Uncovered");
        }
        catch (Uncovered e) { lines.Add($"Uncovered={e.Capability}"); }

        var extra = OcrExpectedCoveredAll.ToList();
        extra.Add("does_not_exist");
        try
        {
            ResolveHotplug("tesseract-ogar", OcrSubjectClassIds, extra);
            throw new Exception("expected Undeclared");
        }
        catch (Undeclared e) { lines.Add($"Undeclared={e.Capability}"); }
        lines.Add("END_HOTPLUG_DRIFT");

        lines.Add("FACET");
        var demo = new Facet(
            (uint)ComposeRenderClassid(ClassIds["commercial_document"], 0x0002),
            new byte[] { 1, 2, 3, 4, 5, 6 },
            new byte[] { 0x11, 0x12, 0x13, 0x14, 0x15, 0x16 });
        var encoded = demo.ToBytes();
        var decoded = Facet.FromBytes(encoded);
        if (decoded.ClassId != demo.ClassId
            || !decoded.IsAChain.SequenceEqual(demo.IsAChain)
            || !decoded.PartOfChain.SequenceEqual(demo.PartOfChain))
            throw new Exception("facet round-trip must be lossless");
        lines.Add($"CLASSID={decoded.ClassId:X8}");
        lines.Add("IS_A=" + string.Join(",", decoded.IsAChain.Select(b => $"{b:X2}")));
        lines.Add("PART_OF=" + string.Join(",", decoded.PartOfChain.Select(b => $"{b:X2}")));
        lines.Add("END_FACET");

        return string.Join("\n", lines) + "\n";
    }
}
"##;
