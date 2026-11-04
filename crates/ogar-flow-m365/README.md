# ogar-flow-m365

PowerShell-shaped command labels for administrators, executed as typed
`ogar-loco` calls. This is the first slice of the flow arc: one read-only
reporting pipeline over a synthetic directory.

```text
  "Get-MgUser … | Get-IamLicenseEvidence | Select-Object … | Export-Report"
       │  frontend::compile                       (strings stop here)
       ▼
  label ─► adjacency::Operation ─► call byte ─► ogar_loco::Program (one Quads body)
                                                      │
  VerifiedLogin ─► ReportAuthority ─► ReportScope     │  ogar_loco::Interpreter::run
                                          │           ▼
                                          └──► FlowDialect ─► DirectoryObservations
                                                      │
                                                      ▼
                                            Report ─► to_json
```

## John Doe 101

```sh
cargo run -p ogar-flow-m365 --features fixture --example john_doe_101
```

```powershell
Get-MgUser -UserId "john.doe@example.test" |
    Get-IamLicenseEvidence |
    Select-Object DisplayName, Department, Licenses |
    Export-Report -Format JSON
```

This is a declarative input, not PowerShell. It compiles to one
`ogar-loco` function body of four calls (`0x90 0x91 0x94 0x95`), which the
unchanged `ogar_loco::Interpreter` runs under the flow dialect. Output, as
produced by the example:

```json
{"format":"ogar-flow-m365/report.v1","requested":"john.doe@example.test",
 "scope":{"tenant":32343,"policy_version":1,"fields":["ObjectId","UserPrincipalName","DisplayName","Department","AccountEnabled","Licenses","Memberships","MailboxAvailable"]},
 "source":{"id":"fixture:john-doe-101","observed_version":7},
 "rows":[{"DisplayName":"John Doe","Department":"Operations","Licenses":["M365 E3","Teams Phone"]}],
 "trace":[{"label":"Get-MgUser","operation":"query_user","fn":"0x90"},
          {"label":"Get-IamLicenseEvidence","operation":"read_license_evidence","fn":"0x91"},
          {"label":"Select-Object","operation":"select_fields","fn":"0x94"},
          {"label":"Export-Report","operation":"export_report","fn":"0x95"}]}
```

(Line breaks added; the real output is one line with fixed key order.)

## The adjacency table

A label is metadata; the operation it names is authoritative. Aliases are
several labels naming one operation row.

| labels | operation | effect | capability | call byte |
|---|---|---|---|---|
| `Get-MgUser`, `Get-AzureADUser` | `query_user` | read | `ReadIdentity` | `0x90` |
| `Get-IamLicenseEvidence` | `read_license_evidence` | read | `ReadLicenses` | `0x91` |
| `Get-IamMembership` | `read_membership` | read | `ReadMemberships` | `0x92` |
| `Where-Object`, `where`, `?` | `filter_rows` | read | field permission | `0x93` |
| `Select-Object`, `select` | `select_fields` | read | field permission | `0x94` |
| `Export-Report` | `export_report` | export | `Export` | `0x95` |
| `Set-MgUser`, `Update-MgUser`, `Remove-MgUser`, `New-MgGroupMember`, `Set-MgUserLicense` | mutations | mutation | — | none: refused |
| `Invoke-Expression`, `iex` | `evaluate_expression` | execution | — | none: refused |

Any other label is unknown and refused.

`Where-Object` admits `<Field> -eq|-ne <value>` on scalar fields and
`<Field> -contains <value>` on list fields. Its comparison byte is the
shared-core `ogar-loco` index (`EQ`, `NEQ`, `LIST_CONTAINS`), not a second
operator table. `Select-Object` packs its fields as a 24-bit
`lance_graph_contract::class_view::FieldMask` into the call's three value
bytes.

Refused syntax: script blocks, variables, subexpressions, statement
separators, redirection, escapes, splatting, call operators, unknown
parameters (`-RunAs`, `-Credential`, …), and any pipeline that does not
start with one source and end in `Export-Report`.

## Trust order

1. The actor is the canonical user of an `ogar_auth::federation::VerifiedLogin`.
   `run_flow` accepts nothing else, so the login boundary is checked by the
   compiler.
2. A `ReportAuthority` maps that user to a `ReportScope`: one tenant, a
   capability set, a field mask, a policy version. No scope means
   `FlowError::NoScope` and the host is never queried.
3. Every domain call checks its capability before it reads. `get_user` asks
   the host only for permitted fields and drops any extra field a host
   returns. Filtering or selecting an unpermitted field is refused (a filter
   on it would leak it through the row count).
4. Host queries carry the scope's tenant, never one named in the pipeline. A
   user in another tenant looks exactly like a user that does not exist.

## Implemented, mocked, deferred

| piece | status |
|---|---|
| Label → operation adjacency, aliases, refusal of unknown and mutating labels | implemented |
| Flow `Vocabulary` (6 domain bytes), passes `ogar-loco` conformance | implemented |
| Lowering of the admitted subset to one `ogar-loco` body | implemented |
| Execution through `ogar_loco::Interpreter` with a flow `Dialect` | implemented |
| Verified-login entry (`VerifiedLogin`) | implemented, real `ogar-auth` type |
| Deterministic JSON report with scope, source, version and trace | implemented |
| Directory observations | **mocked**: `fixture::FixtureDirectory` (synthetic) |
| Report authority | **mocked**: `fixture::FixtureAuthority`, a test double, not security |
| Assertion verification | **mocked**: `fixture::FixtureVerifier`, driven through the real `establish` |
| Production authority via `ogar-rbac` (`OgarRbac::authorize_user`, field masks) | **deferred, gated** (below) |
| Observation adapter over `ogar-az` / Microsoft Graph | deferred (PR-FLOW-02) |
| Registering the vocabulary in a `VocabularyRegistry` under a body classid | **deferred, gated** (below) |
| CSV / other export formats, `-like` / `-match`, `ForEach-Object`, sorting | not implemented |

### The two gates

Both are the same missing piece: **no directory-user, license-assignment,
group-membership or report concept is minted in `ogar-vocab`.** This crate
mints nothing.

- **RBAC.** `OgarRbac::authorize_user(identity, class, op)` and its field
  masks are keyed by classid. Without a minted class for a directory user (and
  for license and membership facts), the production authority has nothing to
  be asked about. `ReportAuthority` is the seam it will implement.
- **Registry.** `VocabularyRegistry::plug` selects a vocabulary by the body
  node's concept id. The flow vocabulary is validated and run directly; it is
  not plugged into a registry, because that needs a body concept id (the
  `ogar-ro` `0x0306` and `ogar-dismech` `0x0333` precedent), which is an
  operator decision.

Every operation records its binding (`adjacency::ConceptBinding`):
`DirSchema` (the `ogar-dir-core` MS Graph user schema), `Generic`, or
`Unminted("…")` naming what is missing.

## The litmus test

`Get-MgUser` was added as a friendly label without changing anything in
`ogar-loco`: no new opcode, no change to the call ABI, the interpreter, the
conformance rules or `DOMAIN_FLOOR`. The only LOCO primitive missing for
production is outside LOCO: the concept ids above.

## Falsifiers

`cargo test -p ogar-flow-m365` runs 29 tests over synthetic data, F1–F10:
resolution and refusal (F1), alias equivalence (F2), no new concept and
shared-core comparison reuse (F3), read-only enforcement including
hand-built bodies (F4), actor isolation: no scope, missing capability,
foreign tenant, actor-choosing parameters, unverified assertions (F5), field
projection including a host that overshares (F6), determinism (F7), parity
with a direct typed reference (F8), `ogar-loco` conformance (F9), and refused
syntax (F10). Each guard was disabled once to confirm a test fails without
it.

## Dependency impact

`ogar-loco` is unchanged (it still pulls only `lance-graph-contract`). This
crate adds `ogar-auth` for the verified-login type, which brings its
cryptography stack (`ogar-encryption`, the ndarray `encryption` crate, hmac,
des, md-5, …): 97 unique crates in the normal-dependency tree. That is the
cost of taking `VerifiedLogin` rather than a bare actor string. A host that
already links `ogar-auth` pays nothing extra.

## Next seam: `report-rs`

A `report-rs` command palette would hold saved pipelines as text, call
`compile` once to show preview and validation errors, and call `run_flow`
with the session's `VerifiedLogin`, a production `ReportAuthority` and an
observation host. `Report` already carries what a provenance panel needs:
source id, observed version, scope, policy version, and the per-stage trace.
Report folds and large populations belong to `lance-graph-report`, behind
`DirectoryObservations`, not in this dialect.
