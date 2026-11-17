# src/analyzer/

## Purpose
Ownership, borrow and mutability inference over the parsed AST. For every function it decides whether each parameter is owned, borrowed or mutably borrowed, whether `self` is `&self`/`&mut self`/`self`, and records optimization hints (clone, string, SIMD, defer-drop detectors).

## Inputs/Outputs
In: `parser::Program` (+ global/external `SignatureRegistry`). Out: `Vec<AnalyzedFunction>`, the `SignatureRegistry`, and trait/type info consumed by `ir` and `codegen`.

## Invariants
- `field_cloneability.rs`: a field type with no `Clone` (channel end, lock guard, file, trait object, or a struct holding one) moved out of `self` makes the receiver owned; Clone fields are read under `&self` and cloned.
- Analysis is read-only over the AST; results live in `AnalyzedFunction`/`SignatureRegistry`, never in the AST.
- Multi-pass compilation re-runs the analyzer with `new_with_copy_structs` and global signatures; results must be deterministic.
- `signature_registry.rs` is the only place that should look signatures up by name (`get_signature(&str)` is ratcheted).
- Identity (C3, docs/spec/c3-inventory.md): an inline-module item is registered under its module-qualified path (`a::take`, `a::Counter::new`) as well as the legacy bare key; `AnalyzedFunction::module_path` and `is_analysis_of` (shared arena body) identify a definition. A bare call inside `mod m` resolves to `m::f` first (`SignatureRegistry::scoped_key`); a bare call to a name the file imports with `use` resolves to that path (`parser::use_imports`, `AnalyzedFunction::imported_paths`, `SignatureRegistry::imported_free_function` in `signature_registry/identity.rs`), never to a same-named method. Suffix searches never pick the first of several homonyms.

## Owns
Parameter ownership inference (`parameter_analysis`, `borrow_analysis`, `passthrough_inference`), self receiver inference (`self_*`), mutation detection, signature registry, type/trait collection, optimization detectors (`optimization_detectors/`), usage tracking (`usage_tracking/`), the `std::arena` escape check and feature gate (`arena_escape.rs`, WJ0160/WJ0161, ADR-0025; also run by the interpreter loader), the structured-concurrency check (`task_structure.rs`, WJ0190, ADR-0004: every task is waited on; also run by the interpreter loader, reported with a fix by `check/tasks.rs`).

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
