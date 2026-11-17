---
status: proposal
---
# `wj economics` and `wj optimize`: command specification

- **Status:** `wj optimize` implemented (2026-11-16): `FILE`/`--explain`, `--pass`, `--json`, `--list`, with promise
  entries in `docs/promises.toml` and cases in `tests/promises/optimize.toml`. `--diff` and all of `wj economics` are
  specified, not implemented (the suite numbers come from `scripts/wj/economics.wj`).
- **Related:** [DEFINITION.md](DEFINITION.md), [PASSES.md](../optimization/PASSES.md), `docs/promises.toml`,
  `tests/promises/*.toml`, `tests/promise_registry_test.rs`

Both commands follow the repository's CLI promise rule: every flag is a `[[promise]]` entry with a one-line statement,
covered by at least one positive and one negative acceptance case in `tests/promises/<command>.toml`, and the entries
move into `docs/promises.toml` in the change that implements them (never as `pending`, which is a shrink-only ratchet).

## `wj optimize`

Shows (and, for a generated crate, applies) the post-codegen optimizer's passes for a Windjammer file, with the
soundness argument of each pass. It answers "what did the compiler do to my program beyond a literal translation, and
why is it safe?".

```
wj optimize --explain FILE.wj [--json] [--pass NAME]
wj optimize --diff FILE.wj [--pass NAME]
wj optimize --list
```

| Promise id | Statement |
|---|---|
| `wj optimize --explain` | For one file: each pass that changed the emitted Rust, with its edit count, the PASSES.md section that argues its soundness, and whether it is on by default |
| `wj optimize --json` | The same report as JSON: `{"file", "passes": [{"name", "edits", "default_on", "soundness": "docs/optimization/PASSES.md#..."}], "clones_before", "clones_after"}` |
| `wj optimize --pass` | Limit the report (or diff) to one pass; an unknown name is an error that lists the known names |
| `wj optimize --diff` | Unified diff of the emitted Rust without and with the optimizer (only the hunks of the selected passes) |
| `wj optimize --list` | Every pass: name, default state, one-line rewrite, soundness section |

Behaviour:

- The file is transpiled once with the optimizer off (the same code path as `wj build --no-cargo`, with
  `OptimizationConfig::disabled()`), then `optimizer::run(&OptimizationConfig::production())` is applied and its
  `applied` map is the report. The report therefore describes exactly what `wj build` emits.
- Exit 0 when the file compiles (whether or not a pass applied), 1 on a compile error (with the usual diagnostic).
- No `--apply`: the optimizer is already applied by every build. Turning a default-off pass on stays a compiler
  configuration (`WJ_AUTO_ARENA=1` for P8), reported by `--list`.

Acceptance cases (`tests/promises/optimize.toml`):

| Case | Polarity | Checks |
|---|---|---|
| a `s = s + "${w},"` loop reports `string_append: 1` and names `PASSES.md#p5-string_append-on` | positive | `stdout_has` |
| `--json` on the same file has `"name": "string_append"` and `"edits": 1` | positive | `json = true` |
| a file with nothing to optimize prints `no pass changed this file` and exits 0 | positive | `stdout_has` |
| `--pass nope` exits 1 and lists the pass names | negative | `exit = 1`, `output_has` |
| a parse error exits 1 with the parser's message | negative | `exit = 1` |
| `--list` names all nine passes, `scratch_reuse` as off | positive | `stdout_has` |

Gate: `tests/promise_registry_test.rs` (cases), plus a mutation check: a build of `wj` whose report omits one pass
must fail the first case (`gate-must-fail-mutation-check`).

## `wj economics`

Reports the economic dimensions of DEFINITION.md for the current project (or one file), against a recorded baseline of
the same project, and optionally against a Rust twin.

```
wj economics [PATH] [--json] [--dims static,verify,compile,run,size,memory] [--input FILE] [--twin FILE.rs]
wj economics --baseline [--write]
```

| Promise id | Statement |
|---|---|
| `wj economics` | Report the dimensions of docs/economics/DEFINITION.md for a project or file: source tokens and ceremony, verify-loop cost (interpreted run), compile cost, run cost, binary size and peak memory |
| `wj economics --json` | The report as JSON (one object per dimension, each with `value`, `unit`, `counter` and, when a baseline exists, `baseline` and `ratio`) |
| `wj economics --dims` | Only these dimensions; `static` needs no build and no run |
| `wj economics --input` | Stdin for the measured runs (default: none) |
| `wj economics --twin` | Also measure a hand-written Rust file and print Windjammer / Rust ratios |
| `wj economics --baseline` | Compare with `.wj-economics.json`; `--write` records the current numbers (lower only unless `--allow-raise`) |

Behaviour:

- The counter is chosen as in `scripts/wj/economics.wj`: `instructions retired` on macOS, `perf` instructions on
  Linux, CPU seconds otherwise; the report names the counter, and numbers from different counters are never compared.
- `static` is computed from the source alone and is deterministic, so it is the dimension a CI gate can use on any
  machine. The others are measured and carry the load average.
- Exit 0 on success; with `--baseline`, exit 1 when any dimension is more than 3% above its baseline (the same rule as
  `scripts/wj/interp_instr.wj --check`).

Acceptance cases (`tests/promises/economics.toml`): `static` on a fixed file prints the exact token and ceremony counts
(positive, deterministic); `--json --dims static` has `"tokens"`; `--dims nope` exits 1 listing the dimension names
(negative); `--baseline` with no `.wj-economics.json` exits 1 and says how to write one (negative); a recorded baseline
plus a file edited to add tokens exits 1 (negative, the gate fails when it should).

Implementation order: `wj economics --dims static` first (deterministic, no new measuring code: it is the D7 counter of
`economics.wj`, moved into a library function shared with the script), then `verify` (interpreted run under the
counter), then the build-backed dimensions, which reuse `wj build`'s cargo cache.

## Why the script first

Both commands' measuring logic exists today as interpreted Windjammer (`scripts/wj/economics.wj`), which is how the
suite numbers in RESULTS.md were produced. A command is worth adding when a user (or an agent) needs the answer for
their own project without this repository's benchmark layout; the script keeps serving the suite.
