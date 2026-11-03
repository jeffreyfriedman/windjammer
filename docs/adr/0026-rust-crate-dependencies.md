---
status: proposed
last_verified: 0827f5fa427a
---
# ADR-0026: First-class Rust crate dependencies (wj.toml, signature surfaces, effects)

- **Status:** Proposed (minimum implemented; generator implemented, see "Generator")
- **Date:** 2026-11-02
- **Related:** [ADR-0012](0012-rust-interop-boundary.md) (interop boundary, option A), [ADR-0021](0021-effects-and-capabilities.md) §11 (externs and crates),
  [ADR-0022](0022-interpret-by-default.md) (interpret by default), [ADR-0023](0023-interpreter-rust-crate-bridge.md) (host bridge),
  `src/package/rust_surface.rs`, `src/package/rust_surface_gen/`, `src/check/rust_calls.rs`, `src/check/rust_surface_health.rs`, `src/compiler/rust_surface_metadata.rs`, `src/ir/effect_facts/crates.rs`, `src/config.rs`, `src/cargo_toml/dependency_management.rs`, `src/compiler/compilation_pipeline.rs`,
  `src/check/project.rs`, `src/ir/effect_facts/analysis.rs`, `src/interpreter/preflight.rs`, `src/ejector.rs`

## Context
What exists today (survey, `next/v0.50.0` at `cd2d3eb9c11c`):

| Piece | Where | State |
|---|---|---|
| `[dependencies]` in `wj.toml` | `config::DependencySpec` (simple version or table: `version`, `features`, `path`, `git`, `branch`, `package`) | Parsed; every entry is forwarded into the generated `Cargo.toml` by `cargo_toml::dependency_management::dep_spec_to_cargo_line_with_base`, so `strsim = "0.11"` already reaches Cargo. |
| Rust vs Windjammer dependency | `package::graph::classify` | Registry entries are Rust crates; a `path`/`git` dependency is Windjammer when it has `wj.toml` or `lib.wj`. |
| `wj check` on `strsim::levenshtein(a, b)` | `check::project::resolve_phase` | The head is a manifest dependency, so the call is accepted without looking at it: wrong names, arity and argument types all pass. |
| `wj build` on the same call | `codegen::rust::generator::report_missing_boundary_signature` | The emitted `main.rs` starts with `compile_error!("missing boundary signature for `strsim::levenshtein`")`: the ownership pipeline has no signature, so it fails closed. Arguments are emitted owned (`levenshtein(a, b)` where Rust wants `&str`). |
| `extern fn` | parser, `codegen` (`extern "C"` block), host bridge (ADR-0023) | C ABI only. Calling a Rust crate means hand-writing a C shim (`ffi.rs`) per function. |
| `std::http`, `std::db`, ... | `std/*.wj` signatures, implementation in `windjammer-runtime`, signatures recovered by `stdlib_scanner` from the runtime's Rust source | The model to copy: a Windjammer-typed signature file in front of Rust code. Only works for the bundled runtime. |
| Effects of a crate call | `ir::effect_facts::analysis` (`Res::Unresolved(External)` goes to the std sink table) | **None.** A crate call is invisible to the capability model: a program that is denied `net` can call `reqwest` freely. |
| Interpreter | `interpreter::preflight` | Reports `deps.rust_crate` and `wj run` falls back to compiled; the message printed the import (`strsim`), not the call. |
| `wj eject` | `ejector` | Reuses the build's `Cargo.toml`, so crate lines survive ejection. |

So the two missing parts are a **declared signature surface** for a crate (what the checker checks and the ownership
pipeline reads) and the **authority** of a crate call. Everything else is plumbing that exists.

## Options
### A. Signature surfaces as Windjammer interface files (`rust/<crate>.wji`)
The project declares the part of a crate it uses in a body-less Windjammer file, typed in Windjammer types, with the
ADR-0021 effect decorators:

```windjammer
// rust/strsim.wji: the surface of the strsim crate this project uses
@pure
pub fn levenshtein(a: string, b: string) -> usize
@pure
pub fn jaro(a: string, b: string) -> f64
```

The compiler checks every `strsim::*` call against it, feeds it to the ownership pipeline as the crate's metadata,
and takes the call's effects from the decorators. The `.wji` extension keeps the file out of the module walker (an
interface is not a module of the program) while the parser, formatter rules and diagnostics stay the same.
Pros: one syntax, reviewable, works offline, deterministic, can be generated. Cons: someone writes or generates it.

### B. Read the crate's rustdoc JSON at build time (superseded as a generator source, see "Generator")
Pros: no hand-written file. Cons: needs a nightly toolchain and a crate download at check time, makes `wj check`
slow and network-dependent, and Rust signatures (generics, lifetimes, traits) do not map to Windjammer types
automatically. Better as a **generator** for A (a proposed `rust-surface` subcommand) than as the source of truth.

### C. Trust the call (today's behaviour, minus `compile_error!`)
Emit the call verbatim and let rustc judge. Pros: zero work. Cons: errors arrive as rustc text about generated code,
ownership is guessed (the boundary-signature failure exists precisely to forbid guessing), and the call stays
outside the capability model.

### D. Inline Rust (`rust { ... }`)
Rejected by ADR-0012 and ADR-0014: leaks Rust into the surface, defeats checking and every non-Rust backend.

## Recommendation: A, with B as a later generator
1. **Declaring a crate.** A Rust crate is an ordinary `[dependencies]` entry, in any form Cargo accepts that
   `DependencySpec` models (`strsim = "0.11"`, `{ version, features }`, `path`, `git`). It is forwarded to the
   generated `Cargo.toml` unchanged. No `rust = true` marker: registry entries are already Rust by classification.
2. **Signature surface.** `rust/<crate>.wji` next to `wj.toml` (crate name with `-` written `_`). Rules:
   - items are body-less `fn` declarations (functions with a body are rejected: the implementation is Rust);
   - types are Windjammer types; `string` parameters are passed borrowed (Rust `&str`), returns are owned; explicit
     widths (`usize`, `u32`, `f32`) mean exactly that Rust type;
   - each function may carry `@pure` or `@uses(...)`.
   The surface becomes crate metadata (`<project>/.wj-cache/rust-surfaces/<crate>/metadata.json`, the format
   path dependencies already publish), so every build mode reads it through the existing external-metadata path.
3. **Checking (`wj check`, and `wj build` before code generation).** For each call `crate::f(..)` (or `f(..)`
   imported with `use crate::f`) into a crate that has a surface:
   - `WJ0050` *not declared in the surface*: ``function `strsim::levenstein` is not declared in the signature surface of Rust crate `strsim` `` with a closest-name suggestion and the surface path;
   - `WJ0013` wrong argument count (same code and wording as for Windjammer functions);
   - `WJ0003` a literal argument of the wrong kind (same rule as for Windjammer functions).
   A malformed surface (function with a body, unparsable file) is `WJ0050` at the surface file.
   Calls into a crate **without** a surface are unchanged in this release (they already fail in the build with the
   boundary-signature error); `wj check` reporting them is the next step (see "Next").
4. **Effects: default-deny, declared grants (ADR-0021 §11 applied to crates).** A crate call is a sink of unknown
   authority. Its effect is:
   - the surface's `@uses(...)` set, or nothing for `@pure` (trusted claims, as for `extern fn`; ADR-0021 records them
     in the capability lock per crate version);
   - otherwise `ffi(<crate>)`, which only `[capabilities] ffi = ["<crate>"]` grants. That includes undeclared
     surface functions and **calls into crates with no surface**, which closes today's hole where crate calls had no
     effect at all.
   Under enforcement (a `[capabilities]` table, a lock file, `--sandbox`) an ungranted `ffi(<crate>)` is `WJ0044`
   and the help names the crate entry to add. Without a `[capabilities]` table nothing changes for compiled builds.
   We do not offer "allow all crates": the point of the model is that a crate's authority is visible.
5. **Interpreter.** A crate call cannot run interpreted unless a host-bridge shim exists (ADR-0023). The preflight
   reports it before `main` runs, and the exact diagnostic is:

   ```text
   not interpreted yet: call into Rust crate dependency `strsim` (`strsim::levenshtein`) [deps.rust_crate]
   ```

   at the call's location. `wj run` prints it on the one-line fallback notice and runs compiled;
   `wj run --interpret` exits 2 with it and the help `run it compiled with `wj run --compiled``. The message names
   the full call path (it used to print only the import).
6. **`wj eject`.** Ejection runs the exact build, so the crate's `Cargo.toml` line and the borrow-correct call sites
   are in the ejected project; the `.wji` files are not copied (an ejected project is plain Rust and calls the crate
   directly). Nothing crate-specific is added to the ejector.

## Generator (`wj rust-surface`, implemented)
Hand-writing surfaces was the cost of option A. The generator removes it:

- **Commands.** `wj rust-surface <crate> [--fn NAME ...|--all-pub] [--write]` prints the surface and refreshes the
  cache copy; `--write` writes an editable copy to `rust/<crate>.wji`. `wj build` generates a surface for every Rust
  crate dependency without a hand-written one, with one `note:` line per crate it (re)generates and nothing when the
  cache is fresh. `wj check` never generates: it reads only project files, stays offline and fast, and reports a call
  into a crate with no surface as `W0014` with the exact commands (`wj rust-surface strsim`, and the `--write --fn ..`
  line that pins only the called functions). `wj build` generating is safe because the output is a pure function of
  the locked crate source and the generator; it never downloads (a crate that is not in the local registry gets a
  note naming `wj rust-surface <crate>`, which asks Cargo).
- **Source of truth: the crate source, parsed with `syn`** (option b of the brief). Evidence:
  rustdoc JSON needs `-Zunstable-options`, which stable rustc rejects (rustc 1.98.0: "the option `Z` is only accepted
  on the nightly compiler"; nightly 1.100.0 of 2026-09-01 documents strsim 0.11.1 in 2.2 s as `format_version` 61); the
  format version changes between nightlies, so a parser pinned to one version breaks for users on another. The crate source is what
  Cargo already unpacked for the locked version (`~/.cargo/registry/src/*/<crate>-<version>`, `vendor/`, or a `path`
  dependency), parsing it costs milliseconds, works on stable and offline, and `syn` is already a dependency. Its
  limits are handled explicitly rather than guessed: `#[cfg]` is evaluated for the enabled features (defaults expanded
  plus `wj.toml` `features`); target-dependent predicates (`unix`, `target_os`) make the function *skipped* with the
  predicate as the reason; `pub use` re-exports from the crate's own modules are followed (inline modules, `m.rs`,
  `m/mod.rs`, `#[path]`), re-exports from other crates are skipped; functions a macro defines are invisible and the
  surface says how many top-level macro invocations exist. rustdoc JSON stays a possible upgrade for crates whose
  API is mostly macro-generated; it is not needed for the first rungs.
- **What maps.** Non-generic free functions over scalars (`i8`..`u64`, `isize`, `usize`, `f32`, `f64`, `bool`,
  `char`), `&str` (as `string`), `String` returns, `&T`/`&mut T` of mappable `T` (`&[T]` as `&Vec<T>`), `Vec`,
  `Option`, `Result` and tuples of owned mappable types, and crate type aliases that resolve to those. Everything
  else is listed at the end of the surface as `// skipped <name>: <reason>`: generic parameters, `impl Trait`
  parameters, borrowed returns, owned `String` parameters (surface `string` parameters are `&str`), crate-defined
  types, `async`/`unsafe`/`#[doc(hidden)]`, target `cfg`, foreign re-exports. `WJ0050` on a skipped function quotes
  the reason. A skipped function is callable only through a hand-written declaration in a pinned copy.
- **Effects.** Generated functions carry no decorator, so calls are `ffi(<crate>)` (default-deny above), except for
  crate versions listed in the curated `src/package/rust_surface_gen/pure_crates.toml` (versioned; today `strsim`,
  `hex`), which become `@pure`. A listed crate still loses `@pure` when its source mentions `std::fs`, `std::net`,
  `std::process`, `std::env`, `std::time`, `extern "C"` or `libc::`. Nothing is inferred from names, and the
  generator never emits `@uses(..)`.
- **Header and staleness.** Every generated file starts with the generator stamp (generator version and pure-list
  version), `// crate: <name> <version>` and `// source: sha256:<hash of the files read>`. The cache copy's first line
  is `// @generated by `wj rust-surface``; the `--write` copy has a different first line because it is source.
  `wj build` regenerates a cache copy whose version or stamp differs from what it would produce now. `wj check`
  reports `W0015` when any generated header (cache or pinned) names another version than `Cargo.lock`
  (`build/Cargo.lock` or `Cargo.lock` next to `wj.toml`).
- **Precedence and repository hygiene.** `rust/<crate>.wji` is source and always wins over
  `.wj-cache/rust-surfaces/<crate>/surface.wji` (whole file, no merging: one file is authoritative and its effects
  are visible in one place). `--write` refuses to overwrite a hand-written file (one without the generator header).
  `scripts/tracked_generated_gate.wj` fails when a repository tracks anything under `.wj-cache/` or a cache surface.
- **Determinism.** Output is sorted by function name and depends only on the crate files, enabled features, the
  generator and the pure list: byte-identical across runs and thread counts (tested).
- **Tests.** `tests/rust_surface_generator.rs` (strsim against the hand-written oracle, hex's generics all skipped,
  determinism, precedence, `--write`, `W0014`, `W0015`, skipped-function `WJ0050`, an end-to-end `wj build` with no
  hand-written surface whose binary prints the real crate's output, the tracked-generated gate on this repository and
  on planted files); unit tests in `src/package/rust_surface_gen/` (type mapping, `cfg`, a fixture crate whose every
  public function is declared or skipped).

## Non-goals
- Generic crate APIs, traits and macros in a surface (`serde::Serialize`, `tokio::main`): wrap them in a concrete
  function in a Windjammer or Rust helper. Opaque crate types (`extern struct`) are the next rung, not this ADR.
- Non-Rust backends (Go, JS): a call into a surfaced Rust crate is a build error there, as `extern` is.
- Running crate code in the interpreter without a shim, or downloading crates during `wj check`.
- Verifying a surface against the real crate at check time (rustc does that at build time; a mismatch is a rustc
  error in the generated code, which is why surfaces should be generated where possible).
- Build scripts and proc macros: they run with the builder's authority (ADR-0021 §11).

## Next (ambition ladder)
1. Done: `wj check` warns (`W0014`) for calls into crates without a surface. Next: an error once `wj build` has had
   a chance to generate (today the build already fails closed on such calls).
2. Done: the generator (section "Generator"). Next on this rung: measure the 80% criterion on five popular small
   crates (in strsim 0.11.1 every `&str` function maps, while the `generic_*` family and `hamming` (crate error type) are skipped; hex maps none because its whole API is generic over `AsRef<[u8]>`,
   the case that motivates the next item), then monomorphise common bounds (`T: AsRef<str>`, `AsRef<[u8]>`,
   `impl Into<String>`) to `string` / `&Vec<u8>` and add an owned-string parameter form so `String` parameters map.
3. Host-bridge shims generated from surfaces (scalars and strings, ADR-0023's ABI), so surfaced crates run
   interpreted; acceptance: the strsim fixture prints the same output interpreted and compiled.
4. Opaque handles for crate types (`pub struct Regex` in a surface, methods as `fn(self: Regex, ..)`).

## Consequences
- "Use any Rust crate" becomes: add the dependency, describe the functions you call in `rust/<crate>.wji`, call them.
  No hand-edited `Cargo.toml`, no C shims.
- A crate call is never invisible to the capability model again.
- Tests: `tests/rust_crate_interop.rs` (Cargo line, surface checks, build stops before codegen, effects,
  interpreter diagnostic, and an end-to-end `wj build` of a real crate, `strsim`, whose binary prints the expected output).

## Decision needed from: project owner
Accept A (surface location `rust/<crate>.wji`, the `.wji` extension, default-deny `ffi(<crate>)` including
un-surfaced crates under enforcement) or amend.
