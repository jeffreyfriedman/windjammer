# Go / JavaScript / WASM backend assessment

Method: read the code, generated output for a 98-program corpus (`tests/conformance_programs`) with the compiler built from
this branch, and executed it (`go 1.26.1`, `node 24.17`; `tsc` is not installed, TypeScript was run through Node type
stripping only). "Before" numbers use the pre-change compiler on the first 35 hand-written programs.

## Verified facts (all true at the start)

| claim | evidence |
|---|---|
| Go/JS computed `prepare_ir_context` and discarded it | `go/generator/mod.rs:41`, `javascript/mod.rs:64` (`let _ir = ...`) |
| `encode_go_call_argument` / `encode_js_call_argument` had no callers | only definitions + a unit test; `git grep` over `src tests` |
| only json and subprocess runtime shims | `std/runtime/{go,js}` held `wj_json.*`, `wj_subprocess.*` only |
| tiny Go/JS conformance | 8 inline programs each (`conformance_{go,js}_test.rs`) + 6 in `integration_wj`, which silently returned `None` when `go` was missing |
| Python/Node/C targets not buildable | `CompilationTarget` has them (`lib.rs`), `wj build --target` accepts rust, go, javascript, wasm, wgsl only |

Further findings (before): Go mapped `int`/`i32` to `int64`, `Result<T,E>` to `T` (error type dropped), tuples to their first
element, `Option` to a pointer, unsupported expressions to `/* unsupported expression */`, `contains` to
`false /* TODO */`; emission was string templating over the untyped AST. JS used `/` for integer division, no BigInt, `===`
everywhere, and an auto-run guard `import.meta.url === file://${argv[1]}` that silently runs nothing when the path has a
symlink (macOS `/var` -> `/private/var`). `--minify` collapsed spaces inside string literals and truncated a template
literal at `//` (output was a `SyntaxError`); `--v8-optimize` was a no-op; `--source-maps` wrote no map;
`module_federation`, `web_workers`, `code_splitter`, `differential_loading` were not referenced by the compiler at all.
`wj run --target go` went through the Rust/cargo path. `wasm.rs` is the Rust generator with an empty signature registry
(`// TODO: Pass analyzed functions`) plus an HTML harness; nothing executes WASM in tests. `ir/target_encodings.rs` Go/JS arms
(mutex wrappers, `Object.freeze`) were never used by an emitter.

## Before: executed results (35 programs)

| backend | OK | wrong output | build/run failure |
|---|---|---|---|
| Go | 10 | 1 | 24 (22 rejected by `go vet`) |
| JS (main invoked explicitly) | 19 | 3 | 13 |
| Rust | all that were run | | |

## After

Architecture: `codegen/portable` (typed IR: exact integer widths, inference variables, ownership-driven `Copy`, flattening
of expression control flow, `Unsupported` errors instead of placeholder code) + `go/emit` + `javascript/emit` (JS and TS) +
runtimes `std/runtime/{go,js}/wj_core.*`, `wj_std.*`. `wj build --target go|js|ts` writes a ready-to-run directory.

| corpus (98 programs, seeds 1-5 for seeded ones) | pass | listed unsupported | fail |
|---|---|---|---|
| Rust | 75 | 23 (Rust-target codegen defects, `docs/known-issues/rust-*`) | 0 |
| Go | 98 | 0 | 0 |
| JS | 98 | 0 | 0 |

Plus 200 generated differential programs and 5 metamorphic relations on Go and JS against an independent evaluator, and
a 9-mutant self-test with 100% kill rate (`docs/backends/CONFORMANCE.md`).

## Feature support matrix (before -> after)

W = WORKS, P = PARTIAL, B = BROKEN, A = ABSENT. Rust column is the corpus result; TS = JS emitter with annotations.

| feature | Rust | Go | JS | TS | WASM |
|---|---|---|---|---|---|
| structs, methods | W | B -> W | W -> W | W | P (Rust path) |
| enums + match, guards, or-patterns | W | B -> W | P -> W | W | P |
| traits / default methods / bounded generics | W | P -> W (generic *methods* and trait objects over enums: unsupported error) | P -> W | W | P |
| generics (fns, structs) | W | B -> W | P -> W | W | P |
| closures (by-value capture snapshots) | P (returned capturing closure rejected) | B -> W | B -> W | W | P |
| Option / Result / `?` | P | B -> W | B -> W | W | P |
| strings, interpolation, UTF-8 byte semantics | W | B -> W | P -> W | W | P |
| Vec / HashMap / HashSet (int/string/char/bool keys) | W | B -> W | B -> W | W | P |
| iterators (eager arrays; no laziness/infinite) | P | B -> P | B -> P | P | P |
| value vs reference semantics of structs/Vec | W | B -> W | B -> W | W | ? |
| integer widths, overflow panics, wrapping/checked ops | W | B -> W | B -> W | W | ? |
| async / spawn / channels | W | A (rejected) | A (rejected) | A | P |
| modules / multi-file imports | W | A | A | A | P |
| decorators (`@test`, `@derive`...) | W | A (ignored) | A (ignored) | A | P |
| test runner (`@test`) | W | A | A | A | A |
| std: math, strings, fs, time | W | A -> W | A -> W | W | ? |
| std: json | W | A (shims exist, not wired) | A | A | ? |

## Semantic hazards (status after)

* Struct/array copy vs alias: fixed (copy inserted on owned use of a place; mutation self-test covers it). Closures snapshot
  captured `mut` locals like Rust `move` closures.
* Integer widths: Go uses exact widths; JS uses BigInt for 64-bit and range-checked Numbers otherwise; `usize` is `uint`/BigInt.
  Arithmetic overflow panics (Rust debug semantics) on all three; this is a documented contract, `--release` Rust would wrap.
* String indexing: byte offsets via `$strSlice`/`wjStrSlice` (panics off a char boundary). Remaining: `trim` uses each
  language's Unicode whitespace set (JS also trims U+FEFF).
* Map iteration order: unspecified everywhere; programs must sort keys (the corpus does). JS keeps insertion order, Go is random.
* Error handling: panics exit 101 on Go/JS; `Result` is a value on all targets; `?` desugars to an early return.
* Floats: Display/Debug follow Rust (no exponent in Display, `1.0` in Debug, `{:.N}` half-even via exact arithmetic in JS).
* Operator precedence: Windjammer's parser binds `&` looser than `==` (C-like), unlike Rust; `a & 1 == 1` is a type error.
* Recursion depth: JS ~10k frames, Go grows, Rust 8 MB; corpus checks depth 5000.

## Removed

`minifier`, `polyfills`, `source_maps`, `v8_optimizer`, `module_federation`, `web_workers`, `code_splitter`,
`differential_loading` (JS), the old AST-templating Go/JS generators, `ir_lowering.rs` in both backends, and the legacy tests
that asserted their output (`enhanced_javascript_tests.rs`, `conformance_{go,js}_test.rs`, `integration_backend_conformance_test.rs`
+ `tests/integration_wj`). `--minify/--polyfills/--v8-optimize/--source-maps` now fail with an explanatory error.

## Remaining gaps, ranked

1. Multi-file projects/imports on Go/JS/TS (single `.wj` or `main.wj` only).
2. Async/spawn/channels and decorators-as-semantics.
3. Rust target defects exposed by the corpus (23 programs; see `docs/known-issues/rust-*`).
4. Lazy iterators; user `Display` impls; `HashMap` with struct/tuple keys; generic Go methods; trait objects over enums.
5. `json`, `http`, and the Rust-only std modules (`std/README.md` tiers).
6. `tsc` type-checking of TS output (needs a `wj_core.d.mts`); WASM has no execution test.
