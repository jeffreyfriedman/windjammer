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

Adding a generator: write `fn(&Ctx) -> String`, register it in `tools/docs_truth/genfiles.rs`. Anything else
under `docs/reference/` fails the check.

## 2. Regions in hand-written docs
`<!-- wj:gen name=NAME [key=value] -->` ... `<!-- wj:end -->` is rewritten by `scripts/docs_gen.sh`.
Names: `cli_commands`, `adr_index`, `lint_range`, `metric key=K`, plus every file generator (`cli`, `errors`, ...).

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

## Ratchets and exclusions
`docs/docs-truth.toml`: `[[exclude]]` entries need a reason; `[baseline.fences|links|claims]` hold per-file counts of
pre-existing problems. A file may not exceed its baseline; `scripts/docs_gen.sh --update-baseline` lowers it and refuses to raise or add one. Fix, do not raise.

## Live example
Number of programs in the conformance corpus (a generated region; edit the code, not this number):

<!-- wj:gen name=metric key=conformance.programs -->
130
<!-- wj:end -->
