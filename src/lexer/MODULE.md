# src/lexer/

## Purpose
Turns Windjammer source text into tokens (identifiers, numbers with suffixes, strings with interpolation).

## Inputs/Outputs
In: `&str`. Out: `Vec<Token>` or `Vec<TokenWithLocation>`.

## Invariants
- Pure and total: never panics on input; unknown characters and malformed literals become `Token::Invalid(message)` (reported with line:column by the parser). Property test: `tests/lexer_parser_never_panic_test.rs`.
- Integer literal typing is one function (`number_lexing::int_token`); spec in `docs/design/lexical-grammar.md`.
- Imports nothing above the syntax layer (one known exception, below).

## Owns
Token definitions and lexing rules.

## Never decides
Syntax structure, types, semantics.

## Allowed imports
Layer `syntax`. May import: every module in a lower layer (support). Known violations (tracked in docs/architecture-allowlist.toml, may only shrink): `type_classification`.
Contract: docs/MODULE_MAP.toml; enforced by tests/architecture.rs.

## Entry points
`Lexer::new(input)`, `tokenize()`, `tokenize_with_locations()` (src/lexer/mod.rs).

## Tests
tests/lexer_tests.rs; inline tests in src/lexer/mod.rs.
Run: `cargo test --release --test all <filter>`; architecture rules: `cargo test --release --test all architecture`.

## Pipe tokens
`|>` is `PipeOp`, `|>?` (no space) is `PipeTry`. `next_token_with_location` skips blanks before
recording the position, so token columns are 1-based and start at the token itself.
