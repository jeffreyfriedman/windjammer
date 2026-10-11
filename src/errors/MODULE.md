# src/errors/

## Purpose
Rust-style error reporting helpers (mutability errors, formatted diagnostics).

## Inputs/Outputs
In: error data + source. Out: rendered messages.

## Invariants
- Messages are user-facing; keep stable.

## Owns
Diagnostic rendering for specific error classes.

## Never decides
Detection of errors.

## Allowed imports
Layer `support`. May import: nothing. Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `analyzer`, `parser`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`errors/mod.rs`, `errors/mutability.rs`.

## Tests
Inline unit tests.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.
