# formatter

Deterministic, idempotent formatter for Windjammer (`.wj`) source.

## Purpose
Re-flow whitespace of Windjammer source without changing program meaning and without
losing comments. The compiler lexer drops comments, so the formatter owns a lossless
tokenizer (`tokens`) instead of reusing it.

## Inputs / outputs
- `format_source(&str, &FormatOptions) -> Result<String, FormatError>`
- `document_edits` / `range_edits` -> minimal `LineEdit`s (editor integration)
- `comments(&str)` -> comment texts in order (test/diagnostic helper)
- `FormatOptions { max_width = 100, indent = 4 }`, read from `wj.toml [fmt]`

## Pipeline
`tokens` (lossless scan, trivia kept as `nl`/`space`) -> `normalize` (brace placement,
blank-line collapsing, trailing commas) -> `layout` (indent from bracket nesting and
continuation lines, `spacing` per token pair) -> `wrap` (over-long `(`/`[` lists go one
element per line, loop to fixpoint) -> `verify` (token stream equal modulo trailing commas,
else `FormatError::Unverified`; callers keep the original text).

## Invariants
- Never reorders or rewrites tokens; only whitespace and optional trailing commas change.
- Author line breaks are kept (except `{` on its own line / `else` after `}` joining and
  blank-line collapsing); lines are never joined otherwise.
- format(format(x)) == format(x); every comment survives, in order.

## Owns / never decides
Owns layout only. Never parses, type-checks, or inspects semantics; no IO, no globals,
no env vars. Callers (CLI, LSP) do all file access and config discovery.

## Allowed imports
`std`, `toml` (options parsing only). Nothing from the compiler pipeline.

## Entry points
`src/cli/fmt.rs` (`wj fmt`), `crates/windjammer-lsp/src/formatting.rs`.

## Testing
`cargo test --release --test all formatter_` (corpus idempotence/AST/comments, golden
fixtures in `tests/fixtures/formatter`, CLI), `cargo test -p windjammer-lsp formatting`.
