# Automated invariant gates

Every class of defect that was once found by inspection has a gate that fails for the whole class. A gate has an
**independent oracle** (not derived from the code under test), a **corpus** (repo programs plus generated ones),
**mutation self-tests** (the gate is fed deliberately broken input and must reject it) and, where the product is not
there yet, a **ratchet** file that may only shrink. All gates below run in `cargo test --release --test all` and in
`scripts/merge_gate.sh` (mandatory before a merge, see AGENTS.md).

## Promise, differential and capability gates

Why these exist: bugs were found only when someone asked ("does the eject command exist?", "is `wj check` really checking?").
Tests verified what implementers chose to test, mostly positive cases, never the product's promises; modes were
never compared with each other; capabilities existed in code but not in the CLI; two drivers drifted.

| Gate | Test | Oracle | Corpus | Self-tests | Ratchet |
|---|---|---|---|---|---|
| **Promise registry** | `promise_registry_test.rs` | the clap tree of `wj` (every subcommand and long flag), `docs/reference/errors.md` (every code), quoted README sentences; each needs a positive AND a negative acceptance case that passes through the installed binary (sandbox: binary alone, empty PATH, private HOME and TMPDIR). Which promises a case covers is read back from clap's parse of its arguments and from its expected output, never declared | `tests/promises/*.toml` (~150 cases) | always-exit-0 mutant binary must fail every negative case; audit rejects missing negatives, short flags map to long names, a new clap flag is detected, a loose ratchet is detected | `docs/promises.toml` `max_pending` (pending promises), `xfail` cases document known bugs and fail when fixed |
| Docs mention only real commands | same file | clap tree | inline/fenced `wj` commands in README, WHY, agent guide, llms.txt, AGENTS.md | unknown command/flag detected | none |
| Every documented code is explainable | same file | `wj explain CODE` exits 0 and names the code | `docs/reference/errors.md` | n/a | none |
| **Dead capabilities** | `capability_reachability_test.rs` | name-based call graph over `syn` ASTs from the entry points `wj`, `wj-lint`, the LSP and MCP servers (the legacy `windjammer` binary is not an entry point); test-only callers never count | all of `src/`, LSP and MCP crates | hand-labelled programs (methods need their type reached, trait-impl methods need the type, private items need their module, `include!` files, test-only use is dead) | `docs/reachability-allowlist.toml` (`[dead_files]` exact set with reasons, `[dead_pub_fns]` per-file counts); intended library API: `[library_api]` in `docs/MODULE_MAP.toml` |
| **Single driver** | `single_driver_parity_test.rs` | clap trees of `wj` and the legacy `windjammer` | both drivers | missing-subcommand detection | `LEGACY_ONLY` (today: `eject`) |
| **check vs build** | `check_build_differential_test.rs` | the build pipeline (`wj build --no-cargo` + `rustc --emit=metadata`) and labelled mutation operators (each operator's output MUST be rejected) | 16 operators x 3 seeds over a seeded base program, controls, the interpreter corpus | classification table, generator determinism, every operator changes and marks the program | `tests/check_vs_build_known.toml`: `check-miss.<op>`, `check-false-error.<op>`, `pipeline-accepts-invalid.<op>` counts, exact (CHECK_DIFF_BLESS=1 after a fix) |
| **Interpreter vs check** | `interpreter_check_parity_test.rs` | `wj check` (the compiler front end): `wj run --interpret` refuses, before running anything, every program `wj check` rejects, and never refuses one it accepts | the labelled mutation operators of `common/check_diff_gen.rs` (each MUST be rejected) at 2 seeds, controls, `tests/interpreter_corpus` | permissive/over-strict/crashed-for-another-reason outcomes are each detected | none (any disagreement fails) |
| Interpreter vs compiled | `interpreter_differential_test.rs` (existing) | Python reference models + compiled Rust | `tests/interpreter_corpus` | print-faking program, source mutants | none |
| All backends vs independent oracle | `backend_conformance_test.rs` (existing, default suite) | known-answer vectors, Python models, generator evaluator | `tests/conformance_programs`, generated programs | injected backend bugs | `docs/backends/` support manifest |
| **Spec on every backend** | `spec_backends_test.rs` (the in-process interpreter run is `spec_conformance_test.rs`) | `// expect:` lines written from the language spec | `tests/spec/**` (lands with wt/lang-spec); interpreter, Rust, JS by default, + Go, TS with `WJ_SPEC_FULL=1` | WRONG vs REJECTED classification | `tests/spec_backend_status.tsv`; WRONG (ran, wrong output) is reported first and separately from REJECTED; `// xfail: Q-NNN` must exist in `docs/spec/OPEN_QUESTIONS.md` |
| **Codegen determinism** | `codegen_determinism_test.rs` | the output of the same build in another process (fresh hash seeds) with another rayon thread count | every verified example and conformance program, 3 runs (6 runs x 4 backends with `WJ_DETERMINISM_FULL=1`) | divergence detector | `tests/determinism_known.txt` (cause required) |
| Hash iteration in codegen paths | same file (`common/hash_iteration_lint.rs`) | text lint: iteration over `HashMap`/`HashSet` names in `src/codegen`, `src/ir`, `src/analyzer` | those trees | labelled snippet | `tests/determinism_hash_iteration.toml` (per-file counts, DETERMINISM_BLESS=1 to tighten) |
| **fmt / fix preserve behavior** | `fmt_fix_semantics_test.rs` | the interpreter run of the untouched program | interpreter corpus at 2 seeds + verified examples; for fix, each program is first rewritten into the deprecated surface (`println!`, `{}` templates, `vec![]`) | `unfix` produces the legacy surface | `tests/fmt_fix_known.txt` |
| **eject == build** (scaffold) | `eject_equivalence_test.rs` | `wj build --no-cargo` output, compared modulo comments | verified examples | comment-insensitive comparison | `EJECT_AVAILABLE` flips when wt/eject lands |
| Docs examples == interpreter | `docs_truth_test.rs` (existing) | interpreter output recorded in the docs | `sh,verify` fences | see docs/CONTRIBUTING-docs.md | docs-truth baselines |

### Heavy, opt-in tools

- **Coverage** (`scripts/coverage.sh`, cargo-llvm-cov): line/branch coverage of `src/` per top-level module by a test
  scope (default: the user-facing acceptance suites); baseline in the coverage JSON the script writes under `docs/generated/`, a module may not drop
  by more than 0.5 points. The report lists the biggest uncovered user-facing files. The installed-binary sandbox
  forwards `LLVM_PROFILE_FILE`, so lines run by the `wj` subprocess count.
- **Mutation testing** (`scripts/mutants.sh`, cargo-mutants): mutates the CLI, test runner, linter and checker files
  and runs the acceptance suites against each mutant; survivors are recorded in a text file the script writes under `docs/generated/`.
  Nightly / CI-optional (each mutant rebuilds the compiler).

### Adding to a gate

- New `wj` flag or subcommand: add a `[[promise]]` to `docs/promises.toml` and a positive and a negative case in
  `tests/promises/<command>.toml`. A known bug is an `xfail = "BUG: ..."` case, not a missing test.
- New module: make it reachable from `wj` (or the LSP/MCP server), or list it under `[library_api]`.
- A nondeterministic build: fix it (BTreeMap/IndexMap or sort); `tests/determinism_known.txt` only shrinks.

## Span and diagnostic-location gates

Every class of defect that was once found by inspection has a gate that fails for the whole class.
A gate has three parts: an **independent oracle** (not derived from the code under test), a
**corpus** (every `.wj` in the repo plus generated sources), and **mutation self-tests** (the gate is
run against deliberately broken input and must reject it). Known violations live in ratchet files
that may only shrink. Rule: a bug found by inspection lands with a class-level gate (AGENTS.md,
hardening rule 12).

Run all span/diagnostic gates: `cargo test --release --test span_oracle_suite`
(`SPAN_ORACLE_VERBOSE=1` prints one example per violation class; `SPAN_ORACLE_BLESS=1` tightens
ratchet files, never loosens them; `SPAN_ORACLE_BLESS=force` re-baselines after a merge brought in code or
corpora from other branches, and the diff is reviewed).

### Position convention (one definition, used everywhere)

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

### Gates

| # | Gate | Oracle | Corpus | Mutation self-tests | Ratchet |
|---|---|---|---|---|---|
| 1 | **Lexer spans** (`lexer_gate.rs`): byte range slices to exactly the token's lexeme, line/col recomputed from the byte offset (start and end), tokens ordered and non-overlapping, gaps contain only whitespace/plain comments (lossless: tokens + trivia reproduce the file), `Eof` is the empty range at the end, no panics | `lexeme.rs` (spelling of every token, exhaustive `match` over `Token`), `naive.rs` | all repo `.wj`, ~3000 generated sources (unicode, tabs, CRLF, BOM, comments, interpolation, raw strings, number-suffix backtracking), hand-written edge cases | 14 mutants (column/line/byte off by one, end wrong, mid-char, overlap, dropped token, wrong kind, missing Eof, byte-vs-char column) | `lexer_panics.txt` (empty) |
| 2 | **AST spans** (`ast_gate.rs`): every node has a span; range valid, on char boundaries, non-empty; line/col consistent; inside its parent; siblings ordered; first and last token legal for the node kind; slice re-parses to the same expression kind and consumes all tokens | `ast_oracle.rs`, `naive.rs` | all repo `.wj` that parse | 14 mutants over a hand-built AST + span table | `ast_location_classes.txt` (counts per rule/kind), `ast_parse_failures.txt` |
| 3 | **Diagnostic location oracle** (`diag_gate.rs`): every diagnostic is declared by an inline `//~` marker with exact line, column, length (when the producer has one), severity, code and message substring; no unannotated diagnostic, no unmatched marker, no diagnostic without a real location. Two producers: `wj check` (`check::check_sources`, all `WJxxxx` errors and lints, exact length) over `tests/diagnostics_oracle/check/` (drafted from `tests/diagnostics/cases` with `generate_check_markers`, then frozen), and the legacy parser + `RustLeakageLinter` path (`wj lint`) over `tests/diagnostics_oracle/` | marker files; the `//~` comments themselves are also a regression test (a diagnostic must not jump into a comment that mentions the name) | the annotated corpora + every repo `.wj` (also `validate_report_spans` on the check report of each) | 10 mutants of `compare`, marker grammar tests, strict-mode `Diagnostic::at` panics | `unannotated_check_diagnostics.txt`, `unannotated_diagnostics.txt` (file + codes), `check_invalid_spans.txt` |
| 4 | **Assertion layer + `--json` validation** (`src/error/span_check.rs`, `misc_gate.rs`): `SourceLocation::new` and the parser's diagnostic emitters panic in strict mode (debug builds, tests that call `set_strict(true)`) on a zero line/column or reversed range; `validate_report_spans(report, source_of)` validates every span of a `wj check --json` report (missing fields, byte/char column confusion, start after end, past EOF, mid-char, secondary spans, fix edits) | `validate_span` | synthetic reports | 7 bad-span classes | none |
| 5a | **LSP / UTF-16** (`misc_gate.rs`): byte -> (line, UTF-16) -> byte round-trips at every char boundary of 300 repo files; independent UTF-16 length oracle; lexer columns convert consistently | `naive::utf16_len` | repo `.wj` | split-surrogate and past-EOL cases | none |
| 5b | **Source maps** (`sourcemap_gate.rs`; maps from `wj build --source-maps`, `codegen/rust/source_map_align.rs`): every `.rs.map` mapping names an existing Rust line, a `.wj` line with code (a column, when present, is a token start), and the first word of the statement occurs on the mapped Rust line | lexer token starts + text of both files | compiled `examples/verified` programs + a CRLF/unicode program | 5 mutants | `sourcemap_compile_failures.txt` |
| 5c | **Formatter edits** (`misc_gate.rs`): applying `document_edits` yields `format_source`'s output, edits are ordered and in range, formatting preserves the token stream | token stream | formatter fixtures, verified examples, CRLF/unicode/multi-line-string cases | n/a | none |
| 6a | **Every parse error is located** (`misc_gate.rs`): ~900 fuzzed broken programs; every parse error carries `file:line:col` and that position exists in the mutated source; no parser panics | `diag_oracle::location_in_message`, `naive::offset_of` | 150 small repo programs x 6 mutations | n/a (vacuity check: >100 errors) | `parser_panics_on_broken_input.txt` |
| 6b | **Multi-line constructs and CRLF** (`misc_gate.rs`): the line of a token after multi-line strings, raw strings, comments and interpolations is exact for `\n` and `\r\n` | line count of the text | generated | covered by gate 1 mutants | none |
| 6c | **Legacy AST locations** (`misc_gate.rs`): reads of the inference-identity `Expression::location()` in diagnostic-producing code (`linter`, `resolve`, `cli`, `ide_analysis`, `error`) may only shrink | text scan | `src/` | n/a | `legacy_ast_location_reads.txt` |

Other automated invariant gates in the compiler (not span related) are listed in
`docs/TESTING_STRATEGY.md` and `tests/architecture.rs` (module layering, file-size ratchets).

### Writing a new diagnostic test

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

### What the span log is, and what it is not

`Expression::location()` / `Statement::location()` are *not* spans: they are the position of the token
after the node and are shared by nested nodes (the numeric inference passes key on them). Diagnostics,
lints and source maps use the parser's span log (`Parser::enable_span_log`, `span_snapshot`;
`src/parser/span_log.rs`). Migrating the remaining readers (gate 6c) is tracked by the ratchet.
