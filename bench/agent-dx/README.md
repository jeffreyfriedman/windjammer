# agent-dx: does an LLM agent do better in Windjammer than in Rust, Go, Python, TypeScript?

This benchmark exists to *measure* (and then keep improving) how well coding agents perform in Windjammer.
It gives every language the same 60 tasks, grades them with the same hidden test vectors, and records what
happens when an agent has to get there through a compiler/test feedback loop.

Pure Python 3 standard library; no network; no LLM calls unless you ask for them (see "Running with Claude Code").

```
python3 -m agentdx toolchains                       # what is installed (missing ones are SKIPPED loudly)
python3 -m agentdx gen                              # regenerate hidden vectors from the Python oracle
python3 -m agentdx baseline --json results/baseline.json   # grade the reference solutions in every language
python3 -m agentdx report --baseline results/baseline.json --transcripts transcripts --md results/report.md
python3 -m unittest discover -s tests               # harness self-tests (no LLM)
```

Run from `bench/agent-dx/`. Set `WJ_BIN=/path/to/wj` to choose the compiler (default: `wj` on PATH), and
`AGENTDX_TIMEOUT_SCALE=3` on a loaded machine. `AGENTDX_CARGO_TARGET` sets the shared cargo target dir used by
the compiled Windjammer path.

## The suite

60 tasks in 8 tiers (`tasks/<id>/spec.md`):

| Tier | Tasks | Examples |
|------|-------|----------|
| pure-functions | t001-t008 | gcd/lcm, sieve queries, Collatz, Roman numerals, base conversion, fast Fibonacci mod m |
| data-structures | t009-t017 | RPN stack machine, ring buffer, LRU cache, heap, BST with delete, union-find, linked list, matrices, interval merge |
| strings-parsing | t018-t027 | word frequencies, Caesar, RLE, anagram groups, CSV with quotes, expression parser, bracket matcher, templates, INI |
| error-handling | t028-t033 | checked division, parse reports, bank ledger, multi-error config validation, checked i32 arithmetic, optional chains |
| traits-generics | t034-t041 | trait objects, generic stack, multi-key sort, default methods, operator overloading, custom iterators, memoizer, enum tree |
| concurrency | t042-t047 | chunked parallel sum, channel pipeline, mutex histogram, ordered parallel map, producer/consumer, dependency scheduler |
| file-cli | t048-t055 | wc, grep, cat -n, file transform, CSV columns, flag parser, directory tree, log summary |
| stateful-apps | t056-t060 | todo manager, inventory, vending machine, Game of Life, key-value store with transactions |

### Language-neutral contract

Every solution is one program (`main.wj`, `main.rs`, `main.go`, `main.py`, `main.ts`) that reads standard input,
command-line arguments and files in its current directory, and writes standard output and an exit status.
A *case* is `{stdin, args, files}`; the grader creates a fresh temp directory, materializes `files`, runs the program,
and compares stdout (trailing whitespace per line and trailing blank lines ignored) and the exit code.
Because the contract is only text, the identical vectors grade all languages.

### The hidden oracle

- `tasks/<id>/oracle/ref.py` is an independent Python implementation (`solve`) plus a case generator: hand-written
  `SAMPLES` and `EDGE` cases and `rand(rng)` seeded randomized cases (seed = crc32 of the task id, 30 cases).
- `oracle/vectors.json` (about 33-46 cases per task) is generated from it and committed; `samples.json` (2 cases) is the
  only part shown to agents. A test regenerates the vectors and fails if the committed file differs.
- The oracle is verified against independent implementations in other languages: the Python, Go, Rust and TypeScript
  reference solutions were written from the spec text alone by separate authors (never from `ref.py`) and all of them
  reproduce the vectors (`tests/test_oracle.py`). Where authors guessed an unspecified detail, the vectors were the
  tie-break and the spec was tightened.

"Hidden" is a convention, not a sandbox: the oracle is in this repository, so the agent workspace never contains it
(asserted by tests) but an agent with unrestricted file access could read it. Run agents in a directory-restricted
sandbox if that matters (see limits).

## Per-language runners (`agentdx/runners.py`)

| Lang id | Build | Run |
|---------|-------|-----|
| `python` | `py_compile` | `python3 -I main.py` |
| `go` | `go build` | binary |
| `rust` | `rustc --edition 2021 -C opt-level=1 --error-format=json` | binary |
| `ts` | `tsc --noEmit --strict` if present, else `deno check`, else `node --check` | `node main.ts` (types stripped) |
| `wj-interp` | none (errors surface at run time) | `wj run --interpret main.wj` |
| `wj` | `wj build --no-cargo --json`, then `cargo build` (shared target dir) | the built binary |

Toolchains are detected at start; a missing one prints `SKIP [lang] toolchain missing: ...` on stderr and that language is
not graded (the report marks it `s`, never as a failure). `tsc` is not installed on the author's machine, so TypeScript
type errors are caught by `deno check`, which is stricter about nothing in particular but is a real type checker.

## Agent-in-the-loop driver

```
python3 -m agentdx run --agent-cmd '<template>' --langs wj,rust,python --tasks t010,t020 --max-iters 3 --trials 5 --out transcripts/my-run
```

Each *trial* creates a fresh workspace containing `PROMPT.md` (spec + visible examples), a starter file, and an executable
`./check` that builds the program and runs the visible examples (printing compiler diagnostics and diffs). Then, for
iteration 1..`--max-iters`:

1. the agent command runs in the workspace (it may call `./check` any number of times itself);
2. the driver builds and grades the file on the visible examples (written to `FEEDBACK.md` for the next iteration) AND on the
   hidden vectors (recorded, never shown);
3. the trial stops at the first iteration that passes every hidden vector.

The agent command is a shell template with placeholders `{workspace} {prompt_file} {feedback_file} {lang} {task}
{iteration} {src} {harness}`. The agent may write `{"input_tokens": n, "output_tokens": m}` to `$AGENTDX_USAGE_FILE`.

Recorded per trial (`transcripts/*.json`, schema 1, full sources included):

| Metric | Definition |
|--------|------------|
| pass@1 | hidden vectors all pass after the first iteration |
| pass within k iterations | share of trials green within k iterations (the iteration budget plays the role of "k attempts with feedback") |
| pass@k (sampling) | `metrics.pass_at_k(n, c, k)`: unbiased estimator over n independent trials, c green |
| iterations-to-green | 1-based index of the first all-green iteration (null if never) |
| tokens in/out | as reported by the agent via the usage file; `null` when not reported (never 0) |
| wall time | agent turn seconds, summed over iterations |
| diagnostic categories | histogram over build errors (and run failures) in a common taxonomy: syntax, name-resolution, type-mismatch, ownership-borrow, trait-bound, api-misuse, lint-style, unsupported, runtime-panic, timeout, wrong-output, toolchain-bug, other |
| first-error-to-fix distance | from the first failing build to the next passing build: number of build attempts (including the agent's own `./check` calls) and number of changed source lines between the two snapshots |

## Replay and CI (no LLM)

```
python3 -m agentdx replay --transcripts transcripts            # re-grade every recorded source with the real toolchains; reports any drift
python3 -m agentdx replay --transcripts transcripts --no-regrade   # metrics only, needs no toolchain at all
python3 -m agentdx report --baseline results/baseline.json --transcripts transcripts --md results/report.md --json results/report.json
```

`replay` feeds each iteration's recorded source through the same grading code path and fails (exit 1) if status, build result
or hidden pass counts differ from the recording, so a compiler change that breaks (or fixes) a recorded solution is visible
in CI without spending tokens. `--no-regrade` is the toolchain-free path for report generation. See `tests/test_driver.py`
for an end-to-end example with scripted agents.

## Running with Claude Code as the agent

`agents/claude_code.sh` runs one iteration with `claude -p` (non-interactive), edits only the workspace, and converts Claude Code's
JSON usage into the usage file:

```
python3 -m agentdx run \
  --agent-cmd 'sh {harness}/agents/claude_code.sh {workspace} {prompt_file} {feedback_file} {iteration}' \
  --langs wj,rust,go,python,ts --tasks t001,t010,t020,t030,t040,t050,t056 \
  --max-iters 3 --trials 3 --out transcripts/claude-$(date +%Y%m%d)
```

Optional environment: `AGENTDX_CLAUDE_MODEL`, `AGENTDX_CLAUDE_TURNS` (default 30), `AGENTDX_CLAUDE_ARGS` (for example
`--append-system-prompt-file wj_notes.md` to test whether a Windjammer cheat sheet helps; run it as a separate, labelled
condition). To make the "hidden oracle" meaningful, run the agent without access to this repository (container, or a
checkout that deletes `tasks/*/oracle` and `solutions/` before the run; the driver itself needs `oracle/` so keep it
outside the agent's reachable paths). This repository's self-tests never invoke an LLM.

## Self-tests (`tests/`, `python3 -m unittest discover -s tests`)

- `test_oracle.py`: vectors reproducible from seeds; samples are a subset of the hidden vectors; every task has >= 25 cases with
  diverse outputs; the Python and Go reference solutions (independent of `ref.py`) pass 100% of every task. Set
  `AGENTDX_ORACLE_LANGS=python,go,rust,ts` to cross-check all four.
- `test_mutation.py`: a program that memorizes the visible examples passes the samples and FAILS the hidden vectors for every task;
  empty/echo/constant/crashing programs fail; build errors and timeouts are detected; exit codes are graded; AST mutants of the
  Python reference solutions (operator flips, constant shifts) are killed at a rate >= 85% (`AGENTDX_FULL=1` for every task;
  survivors are printed, typically equivalent mutants).
- `test_driver.py`: scripted agents drive the loop (green first try, wrong-then-good, syntax-error-then-fix with the right
  first-error-to-fix distance, in-turn `./check` logging, never-green), the workspace never contains oracle data, replay is
  deterministic and detects a tampered transcript, the report aggregates correctly.
- `test_diagnostics.py`: category mapping and the output comparison rule.

## Baseline (reference solutions, hidden vectors, Oct 2026 toolchains)

Full matrix: `results/report.md` (generated from `results/baseline.json`). Windjammer built from the `scrub-compiler` tree (`wj 0.50.0`).

| Language | Tasks passing all hidden vectors | Notes |
|----------|----------------------------------|-------|
| python, go, rust, ts | 60/60 each | independent authors; these cross-check the oracle |
| `wj` (compiled: `wj build` + cargo) | 59/60 | only t043 (thread + channel pipeline) cannot be written; **25/60 compiled on the first draft** (`results/wj_compiled_first_draft.json`), the rest needed the workarounds below |
| `wj-interp` (`wj run --interpret`) | 54/60 | t042-t046 (threads, channels, Mutex) and t054 (`fs::read_dir`) are unsupported by design (loud, exit 2) |

Reading this honestly: the Windjammer solutions were written by the benchmark author who knows the language and ran a compile-fix loop of
about six rounds; the 25/60 first-draft number is the more agent-like figure. Every workaround is a line of evidence for a compiler or
tooling fix, documented with a minimal repro in `docs/known-issues/agent-dx-*.md` (interpreter: none-first-arm, entry binding, range
indexing, `env::args`, stdin APIs, char `to_lowercase`, unsupported-by-design; parser: leading `*` statement, `if (x as T)` expression;
compiled: literal/Option codegen, `cmp` and range literals, miscellaneous codegen, `self` mutability inference, threads and channels,
lint-vs-interpreter mode split). The interpreter bugs "None as first match arm" and "entry().or_insert binding" are already fixed on the
interp-gaps branch; the baseline numbers are from the older tree on purpose.

The `transcripts/scripted/` files are harness smoke runs (a scripted agent that copies reference solutions, plus one canned
syntax-error-then-fix run). They contain no LLM output. No live LLM run has been made; agent numbers must come from `run`.

## Methodology limits (read before quoting any number)

- **Contamination.** Rust, Go, Python and TypeScript are heavily represented in model training data; Windjammer is not. Part of
  any measured gap is data, not language design. Several task ideas (LRU, RPN, Game of Life, union-find) are classics that models
  have seen in every language. Task-level specs are original, but the underlying problems are not.
- **Prompt variance.** Results depend on the system prompt, tool set, model, temperature and wording of `spec.md`. Small trial counts
  give wide intervals; use >= 5 trials per (task, language) and report `pass_at_k` with the sample size, not a single run.
- **Equal prompts are not equal knowledge.** The Windjammer prompt contains no language documentation. An agent in the loop learns it from
  compiler feedback and from whatever docs it can find. Running a labelled condition with a cheat sheet or with `wj` docs on disk is
  supported but must be reported separately.
- **Interpreter vs compiled.** `wj-interp` and `wj` can disagree; report both. Several rows measure toolchain bugs the interpreter or
  compiler has today and say nothing about the language's ceiling.
- **Selection bias.** The tasks were written by the Windjammer team, and the reference solutions in each language were written by a single
  author per language; idiomatic style varies. Tasks are I/O-text programs: they under-sample GUIs, long-lived services, large codebases,
  library design and refactoring, which is where an agent-friendly language should matter most.
- **Grading scope.** Hidden vectors test behavior only; they cannot measure readability, idiom, maintainability or performance. The
  concurrency tasks check deterministic results, not that real parallelism was used.
- **Oracle trust.** The oracle is checked by independent implementations, which reduces but does not eliminate the chance that all share a
  misreading of a spec. Specs were tightened wherever authors disagreed.
- **Hidden is a convention.** See above; the oracle is in-repo.
- **Machine load.** Wall times and timeouts are meaningful only on a quiet machine; use `AGENTDX_TIMEOUT_SCALE` and compare like with like.
