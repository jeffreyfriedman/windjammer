# src/codegen/rust/

## Purpose
The Rust backend: expression, statement, item, type and call-site generation split into ~90 files around `CodeGenerator` (generator.rs).

## Inputs/Outputs
In: `Program` + analyzer output + registry. Out: a Rust source string.

## Invariants
- Largest and most heuristic part of the compiler: `function_generation_prepare.rs`, `ir_call_site.rs`, `generator.rs` exceed the 2,000-line limit (ratcheted in docs/architecture-allowlist.toml).
- Functions over 300 lines are allow-listed and may not grow.
- New code must not add ownership/clone/width decisions here; put them in the IR solvers.
- Definitions are found by identity, never by name (C3): an item inside an inline module takes its analysis by `AnalyzedFunction::is_analysis_of`, its IR function by name plus `module_path`, and refreshes only its module-qualified registry key (`current_module_path`).
- Integer widths are converging on the numeric solver (C4, ADR-0028): `solver_widths.rs` is the only reader of the solver's width facts (`literal_widths`, `solver_local_widths`, `index_source_widths`, `usize_conversions`); literal suffixes, let widths, binary operands the solver agrees on and NUM-USIZE-TO-INT conversions come from it, and the legacy width deciders (binary peer logic, let paints, branch/return casts) run only where it answers `None`.
- Clone insertion is converging on the semantic layer (convergence plan C6): `move_at_last_use.rs` publishes the liveness fact (`ir::liveness`) to every reuse-clone query, so the last use of an owned binding always moves; "is it read again?" questions go to `read_after_current_statement` (the same fact), never to a statement scan.

## Owns
Rust text emission: `expression_generation`, `statement` files, `function_generation_*`, `data_structure_generation`, `method_receiver_resolution` (module function vs value method), `untyped_let_value` (string-literal arms of an unannotated `let`), `struct_field_coercion` (if-arm text tails for String fields), `list_literal_context` (ADR-0014 list-vs-array decision), `call_expression_generation/`, `ir_call_site`, `signature_promotion`.

## Never decides
Cross-function ownership policy, numeric inference, module resolution.

## Allowed imports
Layer `backend`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `compiler`, `linter`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`CodeGenerator::new` / `generate_program` (generator.rs); `backend.rs` implements `CodegenBackend`.

## Tests
tests/codegen_*_test.rs and tests/*bug_*_test.rs regression files.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

## Source maps
`source_map_align.rs` builds the `.wj` -> `.rs` line map by aligning the two texts (functions by name, statements level by level when the shapes agree); written as `<file>.rs.map` when `CompilerConfig::emit_source_maps` (`wj build --source-maps` / `--check`). The generator itself no longer tracks lines.
`durable_lowering`: wraps a `durable fn` body in `durable::run` and each step-site call in `durable::step` (sites come from `CompilerConfig::durable_steps`).
