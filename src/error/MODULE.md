# src/error/

## Purpose
Compiler error types: `SourceLocation`, rich `CompileError`s with context and fix suggestions.

## Inputs/Outputs
In: messages/locations. Out: error values and suggestions (`suggest_fix`).

## Invariants
- Leaf-level support: should not depend on pipeline stages.

## Owns
Error data model and suggestions.

## Never decides
Detection of errors.

## Allowed imports
Layer `support`. May import: nothing. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`error/mod.rs`, `suggestions.rs`, `context_builder.rs`.

## Tests
tests/error_messages_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.
