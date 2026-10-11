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
`changed_lines` since that commit. Malformed metadata fails; missing or old metadata only warns.

## Ratchets and exclusions
`docs/docs-truth.toml`: `[[exclude]]` entries need a reason; `[baseline.fences|links|claims]` hold per-file counts of
pre-existing problems. A file may not exceed its baseline; `scripts/docs_gen.sh --update-baseline` lowers it. Fix, do not raise.

## Live example
Number of programs in the conformance corpus (a generated region; edit the code, not this number):

<!-- wj:gen name=metric key=conformance.programs -->
119
<!-- wj:end -->
