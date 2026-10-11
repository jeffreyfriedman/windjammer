---
status: current
last_verified: 203e43d114c9
---
# Lexical grammar decisions

## Never panic, always a diagnostic with a position
The lexer is total. Any character that starts no token, and any malformed literal (empty `0x`,
integer wider than 64 bits, unterminated string), becomes `Token::Invalid(message)`. `Parser::parse`
reports every `Invalid` token as `Lexical error: <message> (at line L, column C)` before parsing.
Property tests: `tests/lexer_parser_never_panic_test.rs` (arbitrary text, token soup, mutated
corpus programs).

## Attributes: `@decorator`, not `#[attribute]`
ADR-0014 removes Rust-isms from the surface. Windjammer already has decorators (`@derive(Clone)`,
`@test`, `@route("/x")`). `#[derive(Clone)]` is therefore **not** supported; `#` is an invalid
character whose diagnostic suggests the decorator spelling:

    Lexical error: unexpected character '#': Windjammer has no `#[...]` attributes; use a decorator,
    e.g. `@derive(Clone)` instead of `#[derive(Clone)]` ... (at line 1, column 1)

(`r#ident` raw identifiers and `r#"..."#` raw strings are unaffected.)

## Integer literals
One rule (`src/lexer/number_lexing.rs::int_token`), decimal/hex/binary/octal alike:

| literal | token |
|---|---|
| `<= i64::MAX`, no suffix | `IntLiteral(v)` |
| exactly `2^63`, no suffix | `IntLiteral(i64::MIN)` (bit pattern; pinned by `hex_literal_u64_test`) |
| `2^63 < v <= u64::MAX`, no suffix | `IntLiteralSuffixed(bits, "u64")`: FNV-1a / xorshift constants |
| explicit suffix | that suffix; payload is the two's-complement bit pattern (`u64` above `i64::MAX` is a negative `i64` payload, reinterpret with `as u64`) |
| `i*` suffix above `i64::MAX`, or any value above `u64::MAX` | `Token::Invalid` |

128-bit literals (`u128` constants above `u64::MAX`) need a wider `Literal` payload through the
whole pipeline (parser AST, Rust/Go/JS codegen, interpreter `i128` already exists). Until then they
are a diagnostic, not a panic. Decision for the owner: widen `Literal::IntSuffixed` to `i128`.

## Strings
`"${...}"` interpolates. `"${"` is a literal dollar-brace unless the nested string literal is
followed by `.` or `[` (`"${"abc".len()}"`). Inside `${...}` nested string and char literals are
skipped as units, so `}` or `"` inside them does not end the expression.
