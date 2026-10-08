# SurrealDB core IAM — harvest onto the classid RBAC keystone

Source: `AdaWorldAPI/surrealdb` at `ce1b04a`, `surrealdb/core/src/iam/`.
Target: `docs/CLASSID-RBAC-KEYSTONE-SPEC.md` (the four axes, `ClassRbac`,
`authorize_scoped`). Profile: `auth_surrealdb` (`0x0B05`), is-a `auth_store`.

## Why SurrealDB

The keystone's stage 2 (row scope) has no scope-bearing reference proven yet
(§10: "the keystone stays CONJECTURE as a whole until a scope-bearing reference
is green"). SurrealDB's authorization is small, deterministic, and scoped by a
**nested** level, so it is a usable reference for axis 3.

## What SurrealDB decides

About 600 of the module's 10.6k lines are authorization; the rest
(`signin.rs`, `signup.rs`, `verify.rs`, `jwks.rs`, `token.rs`) is
authentication and belongs to the membrane.

| SurrealDB | file | meaning |
|---|---|---|
| `Level::{No, Root, Namespace(ns), Database(ns, db), Record(ns, db, ac)}` | `entities/resources/level.rs` | where an actor or a resource sits; `sublevel_of` is containment, every level contains itself |
| `Role::{Viewer, Editor, Owner}` | `entities/roles.rs` | predefined; custom roles are a declared TODO upstream |
| `Action::{View, Edit}` | `entities/action.rs` | `SELECT`/`LIVE`/`SHOW` → View; writes and `ACCESS` → Edit |
| `ResourceKind` (21 kinds incl. `Config(_)`, `Actor`) | `entities/resources/resource.rs` | what is acted on |
| `Actor { res: Resource, roles }` | `entities/resources/actor.rs` | an actor is a resource of kind `Actor` at a level |
| `is_allowed_check` | `mod.rs` | the whole decision |
| claims `ID`, `RL`, `NS`, `DB`, `AC` | `token.rs` | subject, roles, namespace, database, access method |

The decision:

- **View**: allowed iff the resource's level lies inside the actor's level. No
  role is required.
- **Edit with `Owner`**: allowed iff the resource's level lies inside the
  actor's level.
- **Edit with `Editor`**: as for Owner, but only for 12 kinds — Namespace,
  Database, Record, Table, Document, Option, Function, Analyzer, Parameter,
  Event, Field, Index. Any, Module, Model, Access, Config, Api, Bucket,
  Sequence and Actor stay Owner-only.
- **Edit otherwise**: denied.

`AuthLimit` / `Actor::new_limited` narrows an actor for a sub-operation: the
level moves down when the limit lies inside it, roles above the limit's role
are dropped, and an emptied role set becomes the limit's role or `Viewer`.

## Mapping onto the four axes

| axis | SurrealDB | OGAR |
|---|---|---|
| 1 class-grant (verb × class) | Action × ResourceKind, per role | `ClassGrant { target_classid, op_mask }` per role; View → `READ`, Edit → `WRITE` |
| 2 role hierarchy | Owner ⊇ Editor ⊇ Viewer only through `has_*_role` helpers; `is_allowed_check` itself tests roles individually | explicit grants per role (the kernel does not fold `roles_reaching`) |
| 3 row scope | the actor's level; containment by `sublevel_of` | `ScopeSpec.path` (`ScopePath`, lance-graph contract): Root = `ROOT`, `Namespace(a)` = `[a]`, `Database(a, x)` = `[a, x]`, `Record(a, x, ac)` = `[a, x, ac]`, segments interned; containment = `ScopeSpec::admits` |
| 4 field projection | none | full mask |

Roleless actors: a record-level user holds no roles yet may View. It maps to an
explicit role granted View, as data in this profile, so the kernel keeps
"no roles ⇒ deny".

## What landed

- `auth_surrealdb` (`0x0B05`) in `ogar-vocab` + `ogar-class-view`; lance-graph
  mirror row and `AuthProvider::SurrealDb` (grammar `ID` / `RL` / `NS`).
- `contract::rbac::ScopePath`, `ScopeSpec.path`, `ScopeSpec::admits`, path-aware
  `intersect` (lance-graph).
- `GrantSource::scope_of` in `ogar-rbac`, so a source supplies axis-3 scope.

## OPEN — what blocks the bit-for-bit equivalence probe

1. **Resource kinds have no classids.** Grants key on codebook concepts, and the
   codebook has no database-catalog concepts (namespace, table, field, index,
   function, …). The Editor rule depends on the kind, so a probe needs either
   minted catalog concepts or another way to carry the kind.
2. **Scope is bound to a membership, the trait binds it to a role.**
   `ClassRbac::row_scope(role, class)` has no actor, and `RoleId` is
   `&'static str`. A SurrealDB actor holds "Owner at namespace `a`": the scope
   belongs to the (actor, role, level) membership. Expressing that today needs
   one role id per (role, level) pair created at runtime, which `&'static str`
   does not allow without leaking. Zitadel organisations and any org-scoped
   role meet the same limit.
3. **`Level::No`** (anonymous) is contained by every level as a resource and
   contains only itself as an actor. No `ScopePath` has that property; it is
   left outside the probe.
4. **`AuthLimit` narrowing** is not modelled yet.
