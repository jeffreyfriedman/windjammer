# codegen/wgsl (Windjammer source -> WGSL backend)

## Purpose
`WgslBackend` (target `wgsl`, `wj build --target wgsl`): lowers Windjammer AST
(functions, structs, `extern let` bindings, decorators) to WGSL with GPU
validation and automatic std140-style struct padding.

## Inputs / outputs
Input: parsed `Program`. Output: WGSL text. `shader_metadata` extracts bindings and
entry points back out of WGSL text (used by host crates).

## Invariants
- Type spelling goes through `WgslType::to_wgsl_string` -> `wjsl::wgsl_printer::type_to_wgsl`
  (the single printer); do not add a second type printer here.
- Uniform buffers convert u32 types to f32 (`to_uniform_safe_type`).
- Never decides: shader syntax parsing/checking (that is `wjsl`).

## Imports
`crate::parser`, `crate::wjsl` (types + printer), `codegen::backend`.

## Testing
`cargo test --release --test all wgsl` (golden: `tests/golden/wgsl/backend_*.wgsl`).
