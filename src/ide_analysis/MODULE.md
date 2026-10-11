# src/ide_analysis/

## Purpose
IDE-facing analysis: parse, ownership and type inference without codegen, for LSP/MCP.

## Inputs/Outputs
In: source text + options. Out: `IdeAnalysisResult` (diagnostics, hover info).

## Invariants
- Must never invoke codegen or cargo.

## Owns
IDE query results.

## Never decides
Compilation output.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend) and peers `linter`, `metadata`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`analyze_source`, `analyze_source_at_point` (src/ide_analysis/mod.rs).

## Tests
tests/ide_analysis_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.
