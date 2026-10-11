# Automated invariant gates

Every class of defect that was once found by inspection has a gate that fails for the whole class.
A gate has three parts: an **independent oracle** (not derived from the code under test), a
**corpus** (every `.wj` in the repo plus generated sources), and **mutation self-tests** (the gate is
run against deliberately broken input and must reject it). Known violations live in ratchet files
that may only shrink. Rule: a bug found by inspection lands with a class-level gate (AGENTS.md,
hardening rule 12).

Run all span/diagnostic gates: `cargo test --release --test span_oracle_suite`
(`SPAN_ORACLE_VERBOSE=1` prints one example per violation class; `SPAN_ORACLE_BLESS=1` tightens
ratchet files, never loosens them).

## Position convention (one definition, used everywhere)

| Quantity | Convention |
|---|---|
| line | 1-based; a line ends at `\n` only (a `\r` before it is an ordinary column) |
| column | 1-based, counted in Unicode scalar values (not bytes, not UTF-16 units); a tab is one column |
| byte offset | 0-based, half-open `[start, end)`, always on a char boundary |
| LSP position | 0-based line, `character` in UTF-16 code units (`byte_to_lsp_position`, `lsp_position_to_byte`) |
| token / node span | starts at the first character of the token itself (never at preceding whitespace or comments) |

This matches `docs/diagnostics-schema.json` (v2) of `wj check --json`. The definition and the validators
live in `src/error/span_check.rs`; the tests recompute everything with a naive independent
implementation (`tests/common/span_oracle/naive.rs`) and cross-check.

## Gates

| # | Gate | Oracle | Corpus | Mutation self-tests | Ratchet |
|---|---|---|---|---|---|
| 1 | **Lexer spans** (`lexer_gate.rs`): byte range slices to exactly the token's lexeme, line/col recomputed from the byte offset (start and end), tokens ordered and non-overlapping, gaps contain only whitespace/plain comments (lossless: tokens + trivia reproduce the file), `Eof` is the empty range at the end, no panics | `lexeme.rs` (spelling of every token, exhaustive `match` over `Token`), `naive.rs` | all repo `.wj`, ~3000 generated sources (unicode, tabs, CRLF, BOM, comments, interpolation, raw strings, number-suffix backtracking), hand-written edge cases | 14 mutants (column/line/byte off by one, end wrong, mid-char, overlap, dropped token, wrong kind, missing Eof, byte-vs-char column) | `lexer_panics.txt` (empty) |
| 2 | **AST spans** (`ast_gate.rs`): every node has a span; range valid, on char boundaries, non-empty; line/col consistent; inside its parent; siblings ordered; first and last token legal for the node kind; slice re-parses to the same expression kind and consumes all tokens | `ast_oracle.rs`, `naive.rs` | all repo `.wj` that parse | 14 mutants over a hand-built AST + span table | `ast_location_classes.txt` (counts per rule/kind), `ast_parse_failures.txt` |
| 3 | **Diagnostic location oracle** (`diag_gate.rs`): every diagnostic is declared by an inline `//~` marker with exact line, column, length (when the producer has one), severity, code and message substring; no unannotated diagnostic, no unmatched marker, no diagnostic without a real location | marker files in `tests/diagnostics_oracle/*.wj` | the annotated corpus + every repo `.wj` | 10 mutants of `compare`, marker grammar tests | `unannotated_diagnostics.txt` (repo files that emit diagnostics without markers) |
| 4 | **Assertion layer + `--json` validation** (`src/error/span_check.rs`, `misc_gate.rs`): `SourceLocation::new` and the parser's diagnostic emitters panic in strict mode (debug builds, tests, `WJ_STRICT_SPANS=1`) on a zero line/column or reversed range; `validate_report_spans(report, source_of)` validates every span of a `wj check --json` report (missing fields, byte/char column confusion, start after end, past EOF, mid-char, secondary spans, fix edits) | `validate_span` | synthetic reports | 7 bad-span classes | none |
| 5a | **LSP / UTF-16** (`misc_gate.rs`): byte -> (line, UTF-16) -> byte round-trips at every char boundary of 300 repo files; independent UTF-16 length oracle; lexer columns convert consistently | `naive::utf16_len` | repo `.wj` | split-surrogate and past-EOL cases | none |
| 5b | **Source maps** (`sourcemap_gate.rs`): every `.rs.map` mapping names an existing Rust line, a real `.wj` token start, is monotonic, and the first word of the statement occurs on the mapped Rust line | lexer token starts + text of both files | compiled `examples/verified` programs + a CRLF/unicode program | 5 mutants | `sourcemap_compile_failures.txt` |
| 5c | **Formatter edits** (`misc_gate.rs`): applying `document_edits` yields `format_source`'s output, edits are ordered and in range, formatting preserves the token stream | token stream | formatter fixtures, verified examples, CRLF/unicode/multi-line-string cases | n/a | none |
| 6a | **Every parse error is located** (`misc_gate.rs`): ~900 fuzzed broken programs; every parse error carries `file:line:col` and that position exists in the mutated source; no parser panics | `diag_oracle::location_in_message`, `naive::offset_of` | 150 small repo programs x 6 mutations | n/a (vacuity check: >100 errors) | `parser_panics_on_broken_input.txt` |
| 6b | **Multi-line constructs and CRLF** (`misc_gate.rs`): the line of a token after multi-line strings, raw strings, comments and interpolations is exact for `\n` and `\r\n` | line count of the text | generated | covered by gate 1 mutants | none |
| 6c | **Legacy AST locations** (`misc_gate.rs`): reads of the inference-identity `Expression::location()` in diagnostic-producing code (`linter`, `resolve`, `cli`, `ide_analysis`, `error`) may only shrink | text scan | `src/` | n/a | `legacy_ast_location_reads.txt` |

Other automated invariant gates in the compiler (not span related) are listed in
`docs/TESTING_STRATEGY.md` and `tests/architecture.rs` (module layering, file-size ratchets).

## Writing a new diagnostic test

Add `tests/diagnostics_oracle/<name>.wj` with markers, never an ad-hoc `assert!(stderr.contains(":3"))`:

```
fn main() {
    let v = lookup().unwrap()
//~@13 ERROR W0002 explicit .unwrap() call
    println!("{}", x)
//~ ^^^^^^^^^^^^^^^^^ WARN W0008
}
```

See the module comment of `tests/common/span_oracle/diag_oracle.rs` for the full marker grammar
(`//~ ^^^` underline in the same character columns, `//~@col[+len]`, `//~@eof`).

## What the span log is, and what it is not

`Expression::location()` / `Statement::location()` are *not* spans: they are the position of the token
after the node and are shared by nested nodes (the numeric inference passes key on them). Diagnostics,
lints and source maps use the parser's span log (`Parser::enable_span_log`, `span_snapshot`;
`src/parser/span_log.rs`). Migrating the remaining readers (gate 6c) is tracked by the ratchet.
