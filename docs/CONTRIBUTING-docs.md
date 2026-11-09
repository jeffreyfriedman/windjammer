---
status: current
last_verified: 04c23fa8fcb0
---
# Keeping documentation true

Docs are verified against the code, not trusted. `tools/docs_truth/` holds the checks; `wj-docs` runs them
and `tests/docs_truth_test.rs` runs the same code under `cargo test` (so `make test` includes it).
Commands: `scripts/docs_gen.sh` (regenerate), `--check` (drift only), `--verify` (everything), `make docs-check`.

## 1. Generated files (never hand-edit)
`docs/reference/*.md` and `docs/generated/metrics.json` carry a `GENERATED FILE` header. Sources of truth:

| File | Source in code |
|------|----------------|
| `reference/cli.md` | clap definitions, `src/bin/wj_cli_args.rs` |
| `reference/keywords.md` | `src/lexer/keywords.rs` (pinned to the lexer by tests); operators derived by lexing |
| `reference/intrinsics.md` | `src/intrinsics/mod.rs` |
| `reference/errors.md` | `src/error_codes.rs`, `src/error_catalog.rs`, `src/linter/codes.rs` |
| `reference/targets.md` | `tests/conformance_programs/SUPPORT.toml` + corpus |
| `reference/stdlib.md` | scan of `std/**/*.wj` |
| `generated/metrics.json` | computed counts (see below) |
| `/llms.txt`, `/llms-full.txt` | `tools/docs_truth/gen_llms.rs`: index and inline bundle of the agent guide, CLI/targets/keywords references, MCP tool manifest and known gaps (a missing source fails the check) |

Adding a generator: write `fn(&Ctx) -> String`, register it in `tools/docs_truth/genfiles.rs`. Anything else
under `docs/reference/` fails the check.

## 2. Regions in hand-written docs
`<!-- wj:gen name=NAME [key=value] -->` ... `<!-- wj:end -->` is rewritten by `scripts/docs_gen.sh`.
Names: `cli_commands`, `targets_summary`, `adr_index`, `lint_range`, `metric key=K`, plus every file generator (`cli`, `errors`, ...).

## 3. Executable examples
Every ```` ```windjammer ```` / ```` ```wj ```` fence in Markdown and rustdoc must parse (a fragment is retried as a `fn main` body).
Annotations: `wj,ignore` | `wj,check` (analyzer) | `wj,run` | `wj,should_fail` | `wj,should_fail=WJ0007` (error carries the code) |
`wj,should_fail=W0002` (that lint fires). A `// expect: line` comment runs the fence in the interpreter and compares stdout.
```` ```sh,verify ```` fences run in a temp dir with `wj` on PATH: `$ cmd` lines are commands, other lines must appear in the output, in order.

Documents listed under `[verify] required` in `docs/docs-truth.toml` (the README quickstart) must run every shell fence that
invokes `wj` as `sh,verify` (or mark it `ignore`). A `$ cmd <<'EOF'` here-document creates the input files of a fence.
CI runs them with the freshly built `wj` (`WJ_BIN`, job `docs-truth`).

### Proposals
A document whose front matter says `status: proposal` describes the future. Its `wj` commands/flags, error codes and Windjammer fences are not
checked against the compiler (they name things that do not exist yet) but it must carry a banner within 12 lines of the front matter:
`> **Proposal: not implemented.** ...`. Everything under `docs/proposals/` must declare a status. A proposal that ships becomes
`accepted`/`current` and its fences and commands are checked like any other document.

## 3b. Grammar and interpreter registry
* `docs/windjammer.ebnf` is cross-checked against the real lexer and parser (`tools/docs_truth/ebnf.rs`): every quoted terminal lexes as one token,
  every lexer keyword appears (or is listed with a reason under `[ebnf] unlisted_keywords`), and sentences derived from the grammar (every alternative of every
  production) are parsed by the real parser. Known disagreements are `[ebnf] divergences`, `"production#alt" = reason`; an entry the parser now accepts fails.
* `src/interpreter/features.rs` lists what the interpreter supports and rejects, each with a probe program that `tests/interpreter_features_test.rs` runs.
  `docs/INTERPRETER.md` is generated from it (`interpreter_supported`, `interpreter_unsupported` regions).
* `docs/errors/errors.md` is the output of `wj docs -f markdown` and a registered generated file; its order is sorted so two runs are identical.
* `examples/EXAMPLES.toml` assigns every example and project template (`templates/`) a mode (`run`, `test`, `check`); `tests/examples_gate_test.rs` fails on an unlisted example.

## 4. Claims
Never type a statistic. Use `<!-- wj:gen name=metric key=conformance.programs -->`. Computed keys are in `metrics.json`;
numbers that need a CI run go in `docs/generated/measured.json` as `{"key": {"value": .., "commit": .., "command": ..}}`
written by the CI job that measured them. The lint flags `<N> tests|programs|keywords|intrinsics|subcommands|error codes|lint rules`
and `N / M` ratios on conformance/test lines.

## 5. References
Checked in every `.md`: relative links and `#anchors`, backticked `src/...`-style paths, `ADR-NNNN`, `WJ####`/`W####` codes,
`wj <subcommand> --flag` (against the real clap tree), and rule ids. Rule-id namespaces are pluggable in
`docs/docs-truth.toml` (`[[rule_ids]] prefix = "SPEC-"`, `defs = "docs/spec/**/*.md"`); inactive until the defining files exist.
A line saying something "does not exist" / "removed" is exempt from the `wj` command check.

## 6. Freshness
ADRs and design docs (`[freshness] tracked`) carry front matter `status:` and `last_verified: <commit>`.
`wj-docs stamp FILE` sets it to HEAD. A warning appears when the code paths a doc references changed by more than
`changed_lines` since that commit. Malformed metadata fails. With `[freshness] require = true` (set) missing front matter or `last_verified` fails too; an old
`last_verified` only warns. An ADR's front matter `status` must equal its `- **Status:**` line.

## 7. The book (`docs/book`)
The Windjammer Book is an mdBook (`docs/book/book.toml`, pages under `docs/book/src/`, order in `SUMMARY.md`). Its checks live in
`tools/docs_truth/book.rs` and `tests/book_test.rs`; the sample runner is `scripts/book/book_samples.wj` (interpreted Windjammer).
* Every ```` ```wj ```` fence is a whole program with a `fn main`. Lines starting with `~` are part of the program but hidden in the
  rendered page. `// expect: line` comments are the expected stdout. The runner runs each program interpreted and compiled and requires
  the two outputs to agree. `wj,test` fences run with `wj test --interpret`. `sh,verify` fences run like everywhere else.
* `wj,ignore` needs a `<!-- ignore: reason -->` line above it and is counted: `[book] max_ignored` in `docs/docs-truth.toml` may only
  shrink, as may `max_drafts` (unwritten chapters, written `[Title]()` in SUMMARY) and `max_uncovered_promises`.
* Every id in `docs/promises.toml` must be mentioned by a page: a command by `wj NAME`, a flag by `--flag`, a code by its number, a feature by
  a `<!-- promise: feature ID -->` comment on the page that teaches it. `wj-docs book-uncovered` lists what is missing.
* Pages under `docs/book/src/reference/` are generated (CLI, CLI guarantees from `promises.toml`, keywords, diagnostic codes, targets,
  standard library from `std/**/*.wj`, grammar from the parser-checked `docs/spec/grammar.ebnf`); regenerate with `scripts/docs_gen.sh`.
* Counts in prose come from `<!-- wj:gen name=metric key=book.examples -->` (keys `book.pages`, `book.drafts`, `book.examples`,
  `book.examples_ignored`, `book.promises_uncovered`) or the `book_stats` region, never from typing.
* Links to repository files use `https://github.com/jeffreyfriedman/windjammer/blob/main/<path>` and are checked against the tree.
* `wj-docs book-extract DIR` writes the samples; `wj run --interpret scripts/book/book_samples.wj -- DIR $(which wj) SCRATCH [--compiled]` runs them.
* CI builds and deploys the site (`.github/workflows/book.yml`); locally, `mdbook build docs/book` writes `target/book`.

## Ratchets and exclusions
`docs/docs-truth.toml`: `[[exclude]]` entries need a reason; `[baseline.fences|links|claims]` hold per-file counts of
pre-existing problems. A file may not exceed its baseline; `scripts/docs_gen.sh --update-baseline` lowers it and refuses to raise or add one. Fix, do not raise.

## Live example
Number of programs in the conformance corpus (a generated region; edit the code, not this number):

<!-- wj:gen name=metric key=conformance.programs -->
156
<!-- wj:end -->
