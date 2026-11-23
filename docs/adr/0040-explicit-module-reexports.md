---
status: accepted
last_verified: b73a5629b
---
# ADR-0040: A module re-exports a name only through an explicit `pub use`

- **Status:** Accepted (owner decision, 2026-11-17)
- **Date:** 2026-11-17
- **Related:** [ADR-0030](0030-interpreter-rejects-what-rustc-rejects.md) (interpreter == compiled),
  [ADR-0039](0039-stdlib-one-way.md) (one correct way, no back-compat), `docs/design/visibility-lowering.md` (WJ0019),
  `src/resolve/MODULE.md`, `docs/DIAGNOSTICS.md`.

## Context

Today `pub mod http` inside `adapters/mod.wj` makes `crate::adapters::serve` the same function as
`crate::adapters::http::serve`: the resolver (`lookup_child_reexports`) and the Rust backend (`pub use self::http::*;`
for every child) both treat a module as re-exporting every public item of every public child. Rust does not.
A reader of `adapters/mod.wj` cannot see which names the module offers; two children that declare the same name make
the glob ambiguous (the backend works around it with selective re-exports); adding a public item to a child silently
widens the parent's API; and an agent that knows Rust writes `pub use http::serve` (which works) or `adapters::serve`
(which works only here). One way, as in Rust.

## Decision

A module exposes a name that is declared in another module only through an explicit `pub use` in that module:

```
// adapters/mod.wj
pub mod http
pub use http::serve        // the name `adapters::serve`
pub use codec::*           // a deliberate glob is an explicit re-export, and allowed
```

- `pub mod child` declares the child module `adapters::child`. It re-exports nothing.
- A path `adapters::serve` or an import `use crate::adapters::serve` resolves to an item declared in `adapters`
  itself, to a name `adapters` imports with `pub use`, or to a name a `pub use child::*` glob of `adapters` brings in.
  It does not resolve to an item that only a child of `adapters` declares.
- The interpreter, `wj check`, `wj build` and the generated Rust all follow the same rule (one resolver); the backend
  emits `pub use` lines only for the `pub use` lines the source has.
- Decided by declaration: the check compares what a path resolves to with what each module's scope declares. It never
  matches names or file layouts.

## Diagnostic

`WJ0194` (error, category module, front end): "`serve` is not re-exported by `adapters`; it is declared in
`adapters::http`". Primary span on the path or import that relies on the implicit rule. Help: "add `pub use http::serve`
to `adapters/mod.wj`, or name the item `adapters::http::serve`". The fix is attached as a machine-applicable edit that
inserts the `pub use` line after the module's last `pub mod` / `pub use` line (the module file is the one that declares
`pub mod http`). Several uses of one name produce one edit.

`wj check`, `wj build` and `wj run` stop on it, as they do for WJ0019.

## Interaction with WJ0019 (visibility)

The two rules compose and never overlap:

| Source | Result |
|---|---|
| `adapters::serve`, `serve` declared `pub` in `adapters::http`, no `pub use` | WJ0194 (not re-exported) |
| after the fix `pub use http::serve` | accepted |
| `pub use http::helper`, `helper` not `pub` | WJ0019 (a `pub use` of a private item), as today |
| `adapters::http::serve` | accepted, no re-export needed |
| a child declared `mod http` (not `pub`) and `pub use http::serve` in the parent | accepted; the parent is the child's ancestor and `serve` is `pub` |

A suggested `pub use` is only offered for an item that is itself visible from the parent (declared `pub`); for a private
item the diagnostic is WJ0019 and its fix (declare it `pub`) comes first.

## Migration

`wj fix <path>` adds the missing `pub use child::Name` lines. It resolves the project, groups the uses by re-exporting
module, inserts each missing line once, in sorted order after the module's existing `pub mod` / `pub use` lines, and
re-checks until nothing is left (so a name relied on through two levels gets a line in each module). It is idempotent: a
second run changes nothing, and `wj fix --check` fails while a line is missing. No compatibility flag, no deprecation
period (ADR-0039 policy): the standard library, the packages, the book and the ecosystem repositories are migrated with
one `wj fix`.

Files that only gain a glob through the old rule (`use crate::adapters::*` followed by a bare `serve()`) are reported by
the same check (the glob reaches `adapters`, which does not export `serve`) and fixed the same way.

## Consequences

- `lookup_child_reexports` and the backend's unconditional `pub use self::child::*;` for plain `pub mod` go away; the
  backend's per-module symbol-conflict workaround for wildcards becomes dead code and is deleted with them.
- A module's public surface is the list of its `pub use` lines plus its own `pub` items; docs and agent indexes can
  print it.
- Breaking for every program that relied on the implicit rule: one `wj fix` run. The `docs/std_surface.toml` and book
  examples are migrated in the same change.
- Gates: a corpus fixture that fails with WJ0194 and passes after the fix, a `wj fix` before/after fixture with an
  idempotence check, a mutation case (the implicit lookup restored makes the fixture pass the check, which the test
  forbids), and the interpreter/compiled agreement for the explicit form.

## Decision needed from: project owner

None; decided 2026-11-17.
