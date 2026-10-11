# src/parser/

## Purpose
Recursive-descent parser and AST definitions (`parser/ast/`). Expression, statement, item, type and pattern parsing are split per file. The `Parser` struct itself lives in `src/parser_impl.rs`.

## Inputs/Outputs
In: tokens from `lexer`. Out: arena-allocated `Program<'ast>`.

## Invariants
- AST types are the shared vocabulary of every later stage; changing them touches everything.
- Parsing never consults analysis results.

## Owns
AST node definitions and syntax rules; parse-time error messages.

## Never decides
Ownership, types, name resolution.

## Allowed imports
Layer `syntax`. May import: every module in a lower layer (support) and peers `lexer`, `parser_impl`. No known violations.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`Parser::new(tokens)` then `parse()` (src/parser_impl.rs); AST in `parser/ast/`.

## Tests
tests/parser_*_test.rs and tests/ast_builders_tests.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.
