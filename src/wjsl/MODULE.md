# wjsl (Windjammer Shader Language front end + the single WGSL printer)

## Purpose
The ONE shader front end and ONE WGSL printer. Everything that emits WGSL
(RFC-syntax `.wjsl`, block-syntax `shader X { }`, and the Windjammer-source
WGSL backend in `codegen/wgsl`) prints types/structs/bindings via `wgsl_printer`.

## Pipeline (public API)
`parse(src) -> ShaderModule` -> `check(&module, src)` -> `emit_wgsl(module, src) -> String`.
- `transpile_wjsl(src)` runs all three; `transpile_wjsl_with_includes(src, dir)` resolves
  `use "p"` / `#include "p"` first (`resolve_includes`).
- `ShaderDiagnostics::new(src).enrich(err)` adds WJSL context/hints to any stage's error.
- Block syntax: `compile_legacy_shader(src)` (= `parse_block_shader` + `check_block_shader`
  + `emit_block_shader`); `check_uniform_match` validates host/shader uniform types.

## Modules
lexer, parser, ast: tokens and AST. type_checker + expression_/statement_type_checking +
shader_type_rules: semantic checks. codegen: module-level layout and function bodies.
wgsl_printer: pure type/struct/binding printing (shared). block_syntax: compact block-syntax
front end lowering into the same AST. includes: include resolution. diagnostics: errors/hints.

## Invariants
- Output is pinned by `tests/wgsl_golden_test.rs` (goldens in `tests/golden/wgsl/`).
- No other module may print WGSL types itself; add printing to `wgsl_printer`.
- Never decides: host-side layout (see `codegen/wgsl`).

## Imports
`anyhow`, std only; `codegen/wgsl` depends on `wjsl`, never the reverse.

## Callers
`wj shader-compile` (block syntax), `wj validate-wjsl`, `wj build` (.wjsl files),
`windjammer-runtime-host` (`transpile_wjsl[_with_includes]`).

## Testing
`cargo test --release --test all wgsl_golden wjsl shader wgsl`
