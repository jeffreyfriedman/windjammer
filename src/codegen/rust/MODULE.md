# src/codegen/rust/

## Purpose
The Rust backend: expression, statement, item, type and call-site generation split into ~90 files around `CodeGenerator` (generator.rs).

## Inputs/Outputs
In: `Program` + analyzer output + registry. Out: a Rust source string.

## Invariants
- Largest and most heuristic part of the compiler: `function_generation_prepare.rs`, `ir_call_site.rs`, `generator.rs` exceed the 2,000-line limit (ratcheted in docs/architecture-allowlist.toml).
- Functions over 300 lines are allow-listed and may not grow.
- New code must not add ownership/clone/width decisions here; put them in the IR solvers.
- Clone insertion is converging on the semantic layer (convergence plan C6): `move_at_last_use.rs` publishes the liveness fact (`ir::liveness`) to every reuse-clone query, so the last use of an owned binding always moves; "is it read again?" questions go to `read_after_current_statement` (the same fact), never to a statement scan.

## Owns
Rust text emission: `expression_generation`, `statement` files, `function_generation_*`, `data_structure_generation`, `call_expression_generation/`, `ir_call_site`, `signature_promotion`.

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
