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

## Pipe operator
`pipe_parser.rs` is the single desugaring site for `|>` and `|>?` (spec:
`docs/design/pipe-operator.md`). Backends see plain calls. Misuse diagnostics carry the
1-based `file:line:col` of the offending token (`pipe_error`); tests: `tests/pipe_*_test.rs`.

## Diagnostics and recovery
- `Parser::parse` first reports every `Token::Invalid` (lexical error) as `Lexical error: ... (at line L, column C)`,
  then parses. Parse errors always carry `line L, column C` (`with_position` adds it when a call site forgot).
- Recovery: a failed statement is recorded and the parser resynchronises (`recover_to_statement_boundary`),
  a failed item resynchronises at the next item keyword; up to 25 errors per file are reported, one per line.
  Recovery is only on inside `parse()`; `rewind(mark)` also drops errors recorded inside a speculative parse.
- Statement-boundary (newline) rules: `docs/design/statement-boundaries.md`.
- Match/`if` scrutinees reuse `parse_postfix_expression`: only the primary differs (no struct literal).
- `self::f()` is `Call` on identifier `self::f`; `self.f()` is `MethodCall`. Keep them distinct.
- Tests: `tests/parser_gaps_test.rs`, `tests/lexer_parser_never_panic_test.rs`, `tests/parser_grammar_roundtrip_test.rs`.

## Spans and locations
`Expression::location()` / `Statement::location()` are the position of the token *after* a node and are shared by nested nodes: they are the identity key of the numeric inference passes, **not** spans. Never use them for diagnostics. True spans come from the span log: `Parser::enable_span_log()` before `parse()`, then `expr_span`, `stmt_span`, or `span_snapshot()` (cheap clone, used by the linter and the source map). Spans are recorded in `span_log.rs`: the start is the first token of the innermost parse entry (`parse_expression`, `parse_binary_expression`, `parse_primary_expression`, `parse_match_value*`, `parse_statement`) that allocated the node, the end is the end of the last token consumed. String-interpolation errors carry `file:line:col:` of the failing token inside the interpolation (`Parser::locate_error`). String-interpolation bodies are lexed with `Lexer::with_origin` so their nodes have real file positions. Gate: tests/span_oracle_suite.rs (docs/testing/INVARIANTS.md).
