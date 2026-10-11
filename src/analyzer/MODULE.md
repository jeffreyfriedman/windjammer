# src/analyzer/

## Purpose
Ownership, borrow and mutability inference over the parsed AST. For every function it decides whether each parameter is owned, borrowed or mutably borrowed, whether `self` is `&self`/`&mut self`/`self`, and records optimization hints (clone, string, SIMD, defer-drop detectors).

## Inputs/Outputs
In: `parser::Program` (+ global/external `SignatureRegistry`). Out: `Vec<AnalyzedFunction>`, the `SignatureRegistry`, and trait/type info consumed by `ir` and `codegen`.

## Invariants
- Analysis is read-only over the AST; results live in `AnalyzedFunction`/`SignatureRegistry`, never in the AST.
- Multi-pass compilation re-runs the analyzer with `new_with_copy_structs` and global signatures; results must be deterministic.
- `signature_registry.rs` is the only place that should look signatures up by name (`get_signature(&str)` is ratcheted).

## Owns
Parameter ownership inference (`parameter_analysis`, `borrow_analysis`, `passthrough_inference`), self receiver inference (`self_*`), mutation detection, signature registry, type/trait collection, optimization detectors (`optimization_detectors/`), usage tracking (`usage_tracking/`).

## Never decides
Rust text generation, numeric widths (`type_inference`/`ir::numeric_solver`), call-site coercion, name resolution to DefIds (not yet built).

## Allowed imports
Layer `ownership`. May import: every module in a lower layer (support < syntax < resolve < types) and peers `auto_clone`. Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `codegen`, `ir`, `metadata`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`Analyzer::new()` / `Analyzer::new_with_copy_structs(set)`, then `analyze_program` / `analyze_program_with_global_signatures` (src/analyzer/mod.rs). Called from `compiler::compilation_pipeline`.

## Tests
tests/analyzer_*_test.rs (13 files), plus ownership cases in tests/codegen_*; inline unit tests.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.
