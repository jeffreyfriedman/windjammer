# javascript (JavaScript and TypeScript backends)

## Purpose
Emit an ES-module project from the portable IR: `main.mjs` (or `main.ts`), `wj_core.mjs` (runtime), `package.json`.
One emitter serves both languages; `JavaScriptBackend::typescript()` adds type annotations (runs on Node >= 22.6 type stripping, or `tsc`).


## Inputs/Outputs
`Program` -> portable IR -> `emit::emit_module` -> `main.mjs`/`main.ts`, `wj_core.mjs`, `package.json`.

## Representation contract
- `i64/u64/isize/usize` = `BigInt`; `i8..i32/u8..u32` = Number kept in range by checked helpers (`$i32(a + b)`), panic on overflow;
  `wrapping_*` wrap explicitly. `f32` = `Math.fround`. `char` = one-code-point string.
- `Vec` = Array, `HashMap` = Map (integer/string/char/bool keys only), `HashSet` = Set, tuple = Array.
- `Option<T>` = `null | {v}`, `Result<T, E>` = `{ok, v}`; structs = classes with `clone()`; enums = base class + one class per variant (`$t` tag).
- Strings are UTF-16 in memory but `len()`, slicing and `find` use UTF-8 byte semantics (`$strlen`, `$strSlice`).
- Panics exit 101 (`$run`); output is buffered and flushed on exit.


## Intrinsics
Same as Go: lowered by the portable layer to `$print`/`$eprint`/`$panic`; format specs use `$pad`, `$fixed`, `$radix`, `$dbg*`.
The TypeScript flavour is exercised on the intrinsic corpus (`intrinsic_corpus_conforms_on_typescript`, Node type stripping).


## Invariants
All value-semantics copies come from `PK::Copy` (lowering); the emitter never aliases a struct on assignment.


## Owns
JS/TS spelling and the JS runtime (`wj_core.mjs`).

## Never decides
Typing or ownership (the portable layer does).


## Removed (see docs/backends/ASSESSMENT.md)
`minifier`, `polyfills`, `source_maps`, `v8_optimizer`, `module_federation`,
`web_workers`, `code_splitter`, `differential_loading`: they were text/regex scaffolding with no connection to program semantics.
`--minify/--polyfills/--v8-optimize/--source-maps` now fail loudly. `tree_shaker` (AST-level) is kept.


## Allowed imports
`crate::codegen` (portable IR, backend trait), `crate::parser`, `crate::ir`, `crate::test_utils`. Contract: docs/MODULE_MAP.toml.

## Entry points
`JavaScriptBackend::new()` / `::typescript()`, `emit::emit_module`. CLI: `wj build --target js|ts FILE -o DIR`; `cd DIR && node main.mjs`.


## Test
`cargo test --release --test all backend_conformance`; needs `node` (skips are loud).
