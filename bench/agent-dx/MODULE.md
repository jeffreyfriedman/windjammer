# bench/agent-dx

Self-contained benchmark that measures how well LLM coding agents do in Windjammer compared with Rust, Go,
Python and TypeScript, using identical tasks and one language-neutral oracle. Nothing here is compiled into the
`wj` binary; it is a Python (stdlib only) harness plus data. Read `README.md` for usage and methodology limits.

## Layout

| Path | Purpose |
|------|---------|
| `tasks/<id>/spec.md` | Natural-language spec shown to the agent (60 tasks, `t001`..`t060`). |
| `tasks/<id>/task.json` | Title and tier. |
| `tasks/<id>/samples.json` | Visible examples (a subset of the hidden vectors); shown to the agent. |
| `tasks/<id>/oracle/ref.py` | HIDDEN. Independent Python reference: `SAMPLES`, `EDGE`, `rand(rng)`, `solve(case)`. |
| `tasks/<id>/oracle/vectors.json` | HIDDEN. Generated (seed = crc32 of the task id): samples + edge cases + 30 random cases. Committed so runs are reproducible; a test checks they equal a fresh regeneration. |
| `solutions/{wj,rust,go,python,ts}/<id>/main.*` | Reference solutions (baseline matrix). Go/Python/Rust/TS were written from `spec.md` only (never from `ref.py`). |
| `agentdx/` | The harness package, one concern per file (below). |
| `agents/` | Agent adapters (`claude_code.sh`, `replay_example.sh`). |
| `transcripts/` | Recorded agent runs for deterministic replay (`schema: 1`). |
| `results/` | Generated baseline JSON and the markdown report. |
| `tests/` | Harness self-tests (unittest): oracle cross-checks, mutation checks, driver, metrics, replay. |

## Harness modules (`agentdx/`)

| File | Responsibility |
|------|----------------|
| `paths.py` | Layout constants, language list, solution file naming. |
| `tasks.py` | Load tasks/samples/vectors; render the agent prompt (spec + samples only). |
| `gen.py` | Generate `samples.json` and `vectors.json` from `oracle/ref.py`. |
| `compare.py` | The grading rule (trailing whitespace and trailing blank lines ignored, exit code compared). |
| `toolchains.py` | Detect toolchains; `announce` prints loud `SKIP` lines for missing ones. |
| `runners.py` | Per-language `build` + `run`: python, go, rust (`rustc`), ts (`deno check`/`tsc` + `node`), `wj-interp` (`wj run --interpret`), `wj` (`wj build --no-cargo` + `cargo build`). |
| `diagnostics.py` | Map compiler/runtime messages to a common category taxonomy. |
| `grade.py` | Build once, run every vector, return a result dict (`pass` / `fail` / `build_error`). |
| `baseline.py` | Grade the reference solutions into the language x task matrix. |
| `workspace.py` | Agent workspace (PROMPT.md, starter file, `./check`); never contains oracle data. |
| `driver.py` | Agent-in-the-loop trial runner: `ShellAgent` (pluggable command template), iteration loop, transcript. |
| `metrics.py` | pass@1, pass within k iterations, unbiased pass@k estimator, iterations-to-green, first-error-to-fix distance, categories, tokens, wall time. |
| `replay.py` | Deterministic replay of recorded transcripts (re-grade, or metrics-only with no toolchains). |
| `report.py` | Markdown + JSON report. |
| `cli.py` | `python3 -m agentdx <toolchains|gen|baseline|run|replay|report|check-workspace>`. |

## Invariants

- The oracle (`tasks/*/oracle/`) is never copied into an agent workspace (`tests/test_driver.py` asserts it).
- Every task is graded by the same vectors in every language; there is no per-language test code.
- Visible examples are a strict subset of the hidden vectors, which are at least 15 cases larger.
- Missing toolchains are skipped loudly (stderr `SKIP`), never silently counted as failures or passes.
- Adding a task: create `tasks/tNNN_name/{task.json,spec.md,oracle/ref.py}`, run `python3 -m agentdx gen --tasks tNNN`,
  add solutions, run `python3 -m unittest tests.test_oracle` (cross-language check) before committing.
