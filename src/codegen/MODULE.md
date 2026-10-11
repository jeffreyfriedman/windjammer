# src/codegen/

## Purpose
Backends that print target code from the analyzed AST/IR: Rust (primary), JavaScript, Go, WGSL, WASM. Also the `CodegenBackend` trait and `generate(program, target, config)` dispatcher.

## Inputs/Outputs
In: `parser::Program`, analyzed functions, `SignatureRegistry`, optional `ir` data. Out: `CodegenOutput` (source text + extension) or a Rust `String`.

## Invariants
- Backends should be printers: ownership, clones and numeric widths are decided upstream. Today `rust/` still contains such decisions (e.g. `signature_promotion`, `ir_call_site`); that is the debt WP 7-9 remove.
- Output must compile under `rustc`; golden/fixture tests guard this.
- Go/JS backends do not yet consume the IR (WP 11).

## Owns
Text emission and formatting for every target; per-target helpers in `rust/`, `javascript/`, `go/`, `wgsl/`; `wasm.rs`, `wasm_component_generator.rs`.

## Never decides
Whether a parameter is borrowed/owned, whether to clone, integer/float width, name resolution, file IO, cargo invocation.

## Allowed imports
Layer `backend`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `compiler`, `linter`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`codegen::generate`, `generate_rust`, `generate_go`, `generate_javascript`, `generate_wasm` (src/codegen/mod.rs); `CodeGenerator::new(registry, target)` in `rust/generator.rs`; `create_backend` in `backend.rs`.

## Tests
tests/codegen_*_test.rs (~220 files) via `tests/common/test_utils.rs::compile_single`; backend-specific suites for wgsl/go/js.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.
