# src/interpreter/

## Purpose
Tree-walking interpreter (Windjammerscript) for `wj run --interpret`: evaluates the AST directly without codegen.

## Inputs/Outputs
In: `Program`. Out: a `Value` or error string; can capture stdout.

## Invariants
- Same source must behave like the compiled program for the supported subset.
- No analysis stage runs before it.

## Owns
Environment, values, expression/statement evaluation.

## Never decides
Ownership analysis, codegen.

## Allowed imports
Layer `tooling`. May import: every module in a lower layer (support < syntax < resolve < types < ownership < ir < backend). No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`Interpreter::new()` / `new_capturing()`, `run(&program)` (src/interpreter/mod.rs, engine.rs).

## Tests
tests/interpreter_test.rs, interpreter_advanced_test.rs, interpreter_bugs_test.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.
