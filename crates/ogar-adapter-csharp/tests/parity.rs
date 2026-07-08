//! C# compile+run parity loop.
//!
//! Scaffolds a minimal `net8.0` console project under `/tmp` (NO
//! `PackageReference` — pure BCL, so no NuGet network is ever touched),
//! drops in the emitted class library plus a tiny `Program.cs`, runs
//! `dotnet build -v q` then `dotnet run`, and compares stdout against the
//! live `ogar_vocab` ground truth via `ogar_adapter_python::ground_truth`
//! (reused here as a `[dev-dependencies]` path dependency — the SAME
//! comparison function that verifies the Python emission). A second test
//! proves the facet decoder specifically: Rust builds a known 16-byte
//! facet by hand, C# decodes those exact bytes via
//! `BinaryPrimitives`/`Span<byte>`, and the fields must match.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const NAMESPACE: &str = "Ogar.CapabilitySurface";

const CSPROJ: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>disable</Nullable>
    <InvariantGlobalization>true</InvariantGlobalization>
  </PropertyGroup>
</Project>
"#;

/// A unique temp dir under `/tmp` (or `$TMPDIR`) for one test run.
fn unique_tmp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!(
        "ogar-adapter-csharp-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir under /tmp");
    dir
}

fn dotnet(home: &Path) -> Command {
    let mut cmd = Command::new("dotnet");
    cmd.env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
        .env("DOTNET_NOLOGO", "1")
        // Isolate the dotnet temp/home tree per invocation. The flaky
        // failure is the .NET runtime's cross-process NAMED-MUTEX shared
        // memory, which NuGet's first-run MigrationRunner opens. On Linux
        // that shm lives at `$TMPDIR/.dotnet/shm` (falling back to
        // `/tmp/.dotnet/shm` when TMPDIR is unset) — NOT under HOME or
        // DOTNET_CLI_HOME. Two concurrent `dotnet` processes (the two
        // parity tests run on separate threads in one binary; CI also runs
        // jobs in parallel) then race on the one shared `.../shm/session<N>`
        // path: one loses with `mkdir(...) == EEXIST` or `stat(...) ==
        // ENOENT` in NuGet.Common.Migrations.MigrationRunner — an
        // intermittent hard failure with nothing to do with the code under
        // test. **`TMPDIR` is the controlling variable** (it relocates the
        // shm dir); HOME/DOTNET_CLI_HOME/NUGET_PACKAGES are additionally
        // isolated as hygiene. Each test owns its own `home`, so the shm
        // dirs never collide.
        .env("TMPDIR", home)
        .env("HOME", home)
        .env("DOTNET_CLI_HOME", home)
        .env("NUGET_PACKAGES", home.join("nuget"));
    cmd
}

/// Whether the `dotnet` SDK can be spawned. When it cannot, the parity
/// tests skip with a printed notice instead of a hard panic — a bare
/// sandbox (no .NET SDK) should not report a false failure for what is a
/// missing-toolchain condition, not a code defect.
///
/// Under CI (`CI` env var set, as GitHub Actions does) a missing `dotnet`
/// is instead a HARD failure: the C# parity loop is the whole point of
/// this crate's coverage, so a runner without the SDK must break loudly
/// rather than silently skip. CI images therefore MUST provision the
/// `net8.0` SDK (pure BCL, no NuGet — see the module doc).
fn dotnet_available() -> bool {
    let probe_home = unique_tmp_dir("probe-home");
    let present = dotnet(&probe_home).arg("--version").output().is_ok();
    let _ = fs::remove_dir_all(&probe_home);
    assert!(
        present || std::env::var_os("CI").is_none(),
        "`dotnet` not found but `CI` is set — the C# parity loop requires the \
         net8.0 SDK on CI runners; provision it or unset CI to skip locally",
    );
    present
}

/// Write the csproj + the emitted class library into `dir`. The caller
/// supplies `program_cs` (the tiny entry point) since the two tests need
/// different `Main` bodies.
fn scaffold_project(dir: &Path, program_cs: &str) {
    fs::write(dir.join("parity.csproj"), CSPROJ).expect("write csproj");
    fs::write(
        dir.join("OgarCapabilitySurface.cs"),
        ogar_adapter_csharp::emit_csharp(NAMESPACE),
    )
    .expect("write emitted C# class library");
    fs::write(dir.join("Program.cs"), program_cs).expect("write Program.cs");
}

fn dotnet_build(dir: &Path, home: &Path) {
    let build = dotnet(home)
        .current_dir(dir)
        .args(["build", "-v", "q"])
        .output()
        .expect("spawn dotnet build");
    assert!(
        build.status.success(),
        "dotnet build failed (exit {:?}):\nstdout: {}\nstderr: {}",
        build.status.code(),
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr),
    );
}

fn dotnet_run(dir: &Path, home: &Path) -> String {
    let run = dotnet(home)
        .current_dir(dir)
        .args(["run", "-v", "q", "--no-build"])
        .output()
        .expect("spawn dotnet run");
    assert!(
        run.status.success(),
        "dotnet run failed (exit {:?}):\nstdout: {}\nstderr: {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
    String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn emitted_library_builds_and_dump_matches_ground_truth() {
    if !dotnet_available() {
        eprintln!(
            "SKIP emitted_library_builds_and_dump_matches_ground_truth: \
             `dotnet` (net8.0 SDK) not found — C# parity loop needs it"
        );
        return;
    }
    let dir = unique_tmp_dir("build-dump");
    let home = unique_tmp_dir("build-dump-home");
    scaffold_project(
        &dir,
        &format!("using {NAMESPACE};\nConsole.Write(OgarCapabilitySurface.Dump());\n"),
    );

    dotnet_build(&dir, &home);
    let dump = dotnet_run(&dir, &home);

    // Compare the dump against the Rust-side ogar_vocab ground truth
    // exactly like the Python loop — the SAME comparison function
    // (ogar_adapter_python::ground_truth::assert_dump_matches), so the
    // two verification loops can never silently diverge on what
    // "correct" means.
    ogar_adapter_python::ground_truth::assert_dump_matches("csharp", &dump);

    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn facet_bytes_built_in_rust_decode_correctly_in_csharp() {
    if !dotnet_available() {
        eprintln!(
            "SKIP facet_bytes_built_in_rust_decode_correctly_in_csharp: \
             `dotnet` (net8.0 SDK) not found — C# parity loop needs it"
        );
        return;
    }
    let dir = unique_tmp_dir("facet");
    let home = unique_tmp_dir("facet-home");

    // Rust builds a known 16-byte facet BY HAND, straight from the
    // documented layout (facet.rs / ruff_spo_address::Facet doc):
    //   byte[0..4)    = classid (u32 LE)
    //   byte[4 + 2t]  = is_a_chain[t]   (lo)
    //   byte[5 + 2t]  = part_of_chain[t] (hi)
    // Independent of ogar-adapter-csharp's own `Facet` type — proves C#
    // decodes bytes Rust produced, not just bytes C# produced itself.
    let classid =
        ogar_vocab::app::render_classid(0x0002, ogar_vocab::class_ids::COMMERCIAL_DOCUMENT);
    let is_a: [u8; 6] = [0x21, 0x22, 0x23, 0x24, 0x25, 0x26];
    let part_of: [u8; 6] = [0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6];
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&classid.to_le_bytes());
    for t in 0..6 {
        bytes[4 + 2 * t] = is_a[t];
        bytes[5 + 2 * t] = part_of[t];
    }
    let byte_literals: Vec<String> = bytes.iter().map(|b| format!("0x{b:02X}")).collect();

    let program_cs = format!(
        "using {NAMESPACE};\n\
         byte[] data = new byte[] {{ {} }};\n\
         var f = Facet.FromBytes(data);\n\
         Console.WriteLine($\"{{f.ClassId:X8}}\");\n\
         Console.WriteLine(string.Join(\",\", f.IsAChain.Select(b => $\"{{b:X2}}\")));\n\
         Console.WriteLine(string.Join(\",\", f.PartOfChain.Select(b => $\"{{b:X2}}\")));\n",
        byte_literals.join(", "),
    );
    scaffold_project(&dir, &program_cs);

    dotnet_build(&dir, &home);
    let got = dotnet_run(&dir, &home);

    let expected = format!(
        "{classid:08X}\n{}\n{}",
        is_a.iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(","),
        part_of
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(","),
    );
    assert_eq!(
        got.trim_end(),
        expected.trim_end(),
        "C#-decoded facet fields do not match the Rust-built bytes"
    );

    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&home);
}
