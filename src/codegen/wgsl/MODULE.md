# codegen/wgsl (Windjammer source -> WGSL backend)

## Purpose
`WgslBackend` (target `wgsl`, `wj build --target wgsl`): lowers Windjammer AST
(functions, structs, `extern let` bindings, decorators) to WGSL with GPU
validation and automatic std140-style struct padding.

## Inputs/Outputs
Input: parsed `Program`. Output: WGSL text. `shader_metadata` extracts bindings and
entry points back out of WGSL text (used by host crates).

## Invariants
- Type spelling goes through `WgslType::to_wgsl_string` -> `wjsl::wgsl_printer::type_to_wgsl`
  (the single printer); do not add a second type printer here.
- Uniform buffers convert u32 types to f32 (`to_uniform_safe_type`).

## Owns
Host-side struct layout/padding, uniform-safe types, GPU validation, `shader_metadata` extraction.

## Never decides
Shader syntax parsing/checking and WGSL type printing (that is `wjsl`).

## Allowed imports
`crate::parser`, `crate::parser_impl`, `crate::wjsl` (types + printer), `crate::codegen` (backend trait). Contract: docs/MODULE_MAP.toml.

## Entry points
`WgslBackend` (src/codegen/wgsl/mod.rs), `shader_metadata`.

## Test
`cargo test --release --test all wgsl` (golden: `tests/golden/wgsl/backend_*.wgsl`).
