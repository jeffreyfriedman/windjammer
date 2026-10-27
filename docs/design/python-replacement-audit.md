---
status: current
last_verified: 8ff93b405aaf
---
# Python replacement audit: repository tooling in Windjammer

Windjammer scripts start instantly (`wj run tool.wj`), run on the interpreter every developer already has, and carry their
authority in a `[capabilities]` table that anyone can read. The repository's own tooling is the first place that pays off:
every script we write in Windjammer is a daily test of the language and a reason to grow the ecosystem packages that
Windjammer developers and agents reach for next. This page inventories every Python and shell helper in the compiler
repository, ranks them, records the ports, and lists what each port taught us.

## Result

| | |
|---|---|
| Scripts inventoried | 260 `.py` and `.sh` files: 27 helpers in `scripts/` (7 Python, 20 shell), the 17-file readiness package, the `bench/agent-dx` harness (24 modules, 9 test files), 3 benchmark runners, 8 benchmark twins, and 225 reference models, task oracles and reference solutions |
| Ported to Windjammer | **16** (9 Python, 7 shell), each pinned to its original by recorded differential cases |
| Gate cases | **63** cases in `tests/py_ports/`, run by `scripts/wj/port_gates.wj` and by `tests/python_port_gates_test.rs` |
| New ecosystem packages | `wj-json-doc`, `wj-table`, `wj-stats`, `wj-proc` (tested, with README and MODULE.md) |
| Compiler and runtime changes | 5 fixes and one new std function, each with a regression test (see "Gaps") |
| Python kept on purpose | independent reference models (oracles), benchmark twins, and the steps listed under "Python fallbacks" |

## How a port is verified

A port is behaviourally identical when it prints the same bytes, exits with the same status and leaves the same files as the
original on the same inputs. `scripts/wj/port_gates.wj` checks exactly that:

* every case lives in `tests/py_ports/<port>/<case>/`: a command line for the port (`cmd`), a command line for the original
  (`cmd.record`), the input tree, and the recorded `expected.stdout`, `expected.stderr`, `expected.code` and `expected_tree/`;
* `--record` re-derives the expectations by running the *original* (`cmd.record` reads it from the commit named in
  `tests/py_ports/ORIGIN`), so the expectations are never produced by the port;
* anything that may legitimately differ is declared next to the case (`normalize` patterns for timings and process ids,
  `ignore_stdout` / `ignore_stderr` with a written reason for the few reviewed divergences);
* tools that measure (benchmarks) run against a stand-in `wj`/`rustc`/`cargo` that burns a little CPU, so the gate checks
  structure, ordering, exit codes and error paths, with the measured numbers masked;
* three ports also have a stronger, repository-sized oracle: `gen_check_mutants` must regenerate the 131 committed mutants
  byte for byte, `spec_check` must regenerate the committed conformance matrix, and `regen_conformance_vectors` re-derives
  vectors from the unchanged Python reference models;
* `tests/python_port_gates_test.rs` runs the whole gate in CI and proves it can fail (it damages one expectation and requires a
  FAIL).

The originals are removed in the same change that moves their callers (CI jobs, Makefile, hooks, tests, docs) to the
Windjammer version.

## Inventory

Ranked by dogfood value (how much of the language and ecosystem the script exercises) times simplicity (how cheaply it
ports), each scored 1 to 5.

### Ported

| Script | Was | Lines | Dogfood x simplicity | What it exercises |
|---|---|---:|---|---|
| `check-versions.sh` | shell | 84 | 3 x 4 | TOML text, directory listing, exit codes |
| `sync-version.sh` | shell | 75 | 3 x 4 | regex rewrite, files, git |
| `check-test-ratchet.sh` | shell | 78 | 4 x 3 | streaming a child, set difference, exit codes |
| `check-orphan-tests.sh` | shell | 108 | 5 x 2 | git, globs, regex, hash sets over a 20 MB corpus |
| `check-negative-oracle.sh` | shell | 33 | 3 x 4 | FNV-1a on `u64`, cargo, tab-separated state |
| `spec_check.sh` | shell | 99 | 5 x 2 | rule tables, sorting, generated markdown |
| `diagnostics-leak-report.sh` | shell + embedded Python | 60 | 4 x 3 | JSON diagnostics, cargo JSON stream, table |
| `coverage_report.py` | Python | 80 | 4 x 4 | llvm-cov JSON, ratchet, ordered JSON output |
| `gen-check-mutants.py` | Python | 173 | 5 x 3 | regex and string surgery on 132 programs |
| `regen_vectors.py` | Python | 19 | 3 x 5 | files, child processes, seeds |
| `rust_parity/report.py` | Python | 15 | 3 x 5 | JSON in, markdown table out |
| `rust_parity/run.py` | Python | 136 | 4 x 2 | rustc, timing, statistics, JSON |
| `arenas/run_wj.py` | Python | 63 | 3 x 4 | build, run, median |
| `interp_bench.py` | Python | 63 | 4 x 4 | CPU timing, JSON baseline, ratchet |
| `bench_effects_overhead.py` | Python | 77 | 4 x 4 | interleaved arms, CPU timing, load guard |
| `interpreter_project_bench.py` | Python | 100 | 4 x 3 | generated projects, watch loop, wall time |

### Kept, with the reason

| Script or group | Lines | Why it stays |
|---|---:|---|
| `scripts/readiness/*`, `interp_readiness.py` | about 1,900 | owned by the readiness stream (ported last, only if clearly safe); 5 x 1 |
| `bench/agent-dx/agentdx/*` and `tests/` | about 3,300 | a Python package with its own unittest suite; porting it module by module breaks the package, so it is ported whole or not at all (5 x 1); the Python reference solutions beside it are the comparison arm |
| `interpreter_coverage.py` | 191 | needs a per-child timeout (see "Python fallbacks"); 4 x 2 |
| `*.ref.py`, `oracle/ref.py`, `solutions/python`, `tests/fixtures/optimizer/reference.py` | about 225 files | independent reference models: the point is that they are not Windjammer |
| `bench/interp/twins/*.py` | 8 | the Python side of the interpreter-vs-Python comparison |
| `ci_check.sh`, `merge_gate.sh`, `mutants.sh`, `coverage.sh`, `docs_gen.sh`, `update-architecture-baselines.sh`, `install-hooks.sh`, `atomic_install_wj.sh` | about 350 | thin command sequences around cargo and git; usefulness 2, next wave |
| `spec_backends.sh`, `verified_backends.sh`, `test_all_examples.sh`, `interp_compare.sh`, `wj-fix-surface.sh`, `conformance/run_conformance_tests.sh`, `install.sh` | about 750 | per-example loops over backends with toolchain probing; next wave (3 x 3) |

## Gaps found and how each was resolved

Each gap was resolved one way: fixed in the compiler with a regression test, answered by a tested ecosystem package, or
recorded as a Python fallback.

### Fixed in the compiler

| Gap | Repro (neutral) | Fix and test |
|---|---|---|
| Interpreted `env::args()` was an iterator, so `args[1]` failed; the compiled runtime returns a `Vec` | `let a = env.args()` then `a[1]` | returns a `Vec` (`tests/interpreter_cli_test.rs`) |
| Flags after the script path were captured by `wj run` (`wj run tool.wj --release`, `--record` and others belong to the tool) | `wj run argv.wj --name value` | PATH and everything after it is the program's; wj's own flags go before PATH; a leading `--` is still accepted (`tests/interpreter_cli_test.rs`) |
| A grant could never cover generated sources or manifests: writes to `*.wj`, `wj.toml` and lock files needed an exact path | a generator writing `out/NAME.wj` for many names | a pattern whose last segment spells the file's own suffix (`./out/*.wj`, `./tmp/**/wj.toml`) grants it; `**` alone still never reaches them (unit test in `src/ir/effect_facts/manifest.rs`, documented in `docs/INTERPRETER.md`) |
| String and char literals had no `\u{...}` escape, and `"\0"` silently became `"0"` | `"caf\u{e9}"` | lexer support, malformed escapes keep their old meaning; spec rule SPEC-LEX-016 with an executable example and `tests/lexer_tests.rs` |
| No way to run a program on the terminal and get its status: `run_with_output` buffers, and `subprocess::read_line` treats a blank line as end of output | a test runner printing a progress line, a blank line, then results | `std::process::run_inherit(program, args) -> Result<i32, string>` (capability-gated like the other process sinks; `tests/interpreter_std_modules_test.rs`, runtime unit test) |

### Answered by ecosystem packages

| Need | Package | Notes |
|---|---|---|
| Ordered JSON and Python-compatible `json.dumps` (`std::json` sorts object keys) | `wj-json-doc` | parse, build, `dumps(indent, sort_keys)`, `float_repr` (`1e+16`, `1.5e-05`, `100.0`); tests take their expectations from Python's `json` |
| Markdown and aligned tables, fixed-point numbers | `wj-table` | `markdown`, `plain`, `fixed`, `signed`, padding by characters |
| Median, mean, standard deviations | `wj-stats` | definitions follow Python's `statistics` |
| Process capture, CPU time, load average, git listing | `wj-proc` | `capture`, `sh_in`, `quote`, `timed`, `load_avg`, `git_ls_files` |
| Argument parsing, directory walking | `wj-cli-args`, `wj-fs-walk` (existing) | used as they are |

### Friction recorded (not fixed here)

* `std::json` object keys come back sorted; fine for data, wrong for files that people diff.
* `strings.substring` counts characters while `find`, `rfind`, and regex matches report byte offsets; `s[a..b]` is bytes.
  Mixing them corrupts non-ASCII text. The ports slice with `s[a..b]` throughout.
* `std::regex` has no look-around and `Captures` carries no group offsets; the mutant generator checks neighbouring bytes by
  hand and derives group offsets from contiguous groups.
* `str.split(..)` returns an iterator, so `parts[0]` fails with `indexing Iterator with i64` and no hint to collect;
  `strings.split` returns a `Vec`.
* `len()` is `usize` and literals are `int`: `n - 1` on an empty collection panics at run time with "attempt to subtract with
  overflow" where a compile-time type error would be kinder.
* Diagnostics for an error inside a `${...}` method call print a synthetic span (`--> :4:11240018`) instead of the source line.
  Repro: `println("${s.find("l")}")` where `find` returns an `Option`.
* Interpreter and compiled runtime disagree in small ways that only a compiled build of the same package reveals:
  `strings.char_count` and `strings.repeat` are declared in `std/strings.wj` and run interpreted, but the compiled runtime has
  neither; `strings.split_lines` keeps a final empty element when compiled and drops it interpreted; `s.substring(0, n as usize)`
  compiled to `s[0..n]` with the cast dropped; in `c + 1` with `c: usize` the literal is typed `i32`. The new packages avoid the
  affected calls and are tested both ways (`wj test --interpret` and `wj test`); each item is a parity-gate candidate.
* No `isatty`: `check-versions` printed colour only on a terminal; the port prints plain text.
* `fs.write` drops file modes, so fixtures that must stay executable are copied with `cp -R`.
* Per-dependency capability grants cannot name an arbitrary program path (`process = ["/abs/wj"]` has to be spelled out), so
  scripts that drive a user-supplied binary launch it through `env`, which makes `env` and `sh` the broad grants of
  `scripts/wj/wj.toml`.
* CPU time comes from `/usr/bin/time -p` at 10 ms resolution; the Python originals read microsecond `rusage`. A
  `process::run_timed` returning user and system microseconds would remove the difference.
* `Path.resolve()` has no equivalent that tolerates a path outside the grant (`fs.canonicalize` is denied there, and a denial
  ends the run), so `coverage_report` compares the physical paths llvm-cov writes.

### Python fallbacks

Python fallback: per-child timeout, no `process` function can stop a program after N seconds (`scripts/interpreter_coverage.py` runs the interpreter on hundreds of files with a 20 s limit each), repro: `process::run_with_output("sh", ["-c", "sleep 100"])` blocks for 100 s.

Python fallback: owned by the readiness stream, `scripts/interp_readiness.py` and `scripts/readiness/*`; ported last and only if clearly safe.

Python fallback: package-sized, `bench/agent-dx/agentdx/*` is one Python package with a unittest suite and a Python arm in the comparison it measures; it moves as a whole.

## Measurements

CPU seconds (user plus system, children included) of one run of each case, median of 3, original against port, measured with
`wj run scripts/wj/port_gates.wj --wj PATH --time 3`. The machine was shared by many other jobs (load average between 60 and 140 while it ran), so read the
ratios as the order of magnitude and the absolute numbers as upper bounds.

| port | original | what it is | Windjammer port | cases | ratio |
|---|---:|---|---:|---:|---:|
| `arena_bench` | 0.46 s | Python | 0.42 s | 1 | 0.91x |
| `bench_effects_overhead` | 13.85 s | Python | 13.87 s | 4 | 1.00x |
| `check_negative_oracle` | 0.74 s | shell | 0.23 s | 2 | 0.31x |
| `check_orphan_tests` | 0.78 s | shell | 0.49 s | 5 | 0.63x |
| `check_test_ratchet` | 0.50 s | shell | 0.20 s | 9 | 0.40x |
| `check_versions` | 0.46 s | shell | 0.05 s | 8 | 0.11x |
| `gen_check_mutants` | 0.38 s | Python | 0.77 s | 4 | 2.03x |
| `interp_bench` | 11.86 s | Python | 11.78 s | 5 | 0.99x |
| `interpreter_project_bench` | 0.36 s | Python | 0.21 s | 2 | 0.58x |
| `regen_conformance_vectors` | 0.56 s | Python | 0.47 s | 2 | 0.84x |
| `rust_parity_run` | 0.72 s | Python | 0.62 s | 2 | 0.86x |
| `spec_check` | 9.39 s | shell | 0.25 s | 5 | 0.03x |
| `sync_version` | 0.53 s | shell | 0.38 s | 5 | 0.72x |

The per-case rows are reproduced by the command above; the ones worth singling out: `spec_check` on the committed 276-rule specification took 8.46 s as shell and 0.17 s as the port, `gen_check_mutants` on the 132-program corpus took 0.15 s as Python and 0.58 s as the port, and the two benchmarks that time child processes (`interp_bench`, `bench_effects_overhead`) spend their time in the children, so original and port agree to the 10 ms resolution of `time -p`.

Startup, an empty program: `python3 -c pass` and `wj run hello.wj` each cost 0.01 to 0.05 CPU seconds here (`bash -c :` rounds to 0.00); the interpreter starts as fast as the Python it replaces, and a script that does real work is bounded by its children, not by the language.

### What the table says

* Scripts that mostly drive children (benchmarks, `cargo`, `rustc`, `git`) cost the same in either language: the child dominates.
* Scripts that do text work in the shell (`spec_check`, `check_versions`, `check_test_ratchet`, `check_negative_oracle`, `sync_version`) are faster as Windjammer, because the shell versions fork `grep`, `sed`, `awk` and `cut` per line.
* The one slower port is the string-surgery script `gen_check_mutants` (about 4x Python on 132 programs, 0.6 s): per-operator regex and `s[a..b]` slicing in the tree-walking interpreter against CPython's tuned string code. It is a candidate for the interpreter-speed work, not for a fallback; 0.6 s is well inside the budget of a script that runs when the corpus changes.
* The measurement machine was shared (load average 55 to 140), so the CPU-time columns are the honest comparison; wall time was not recorded.

## Developer experience

What helped:

* Capability errors are exact. Every denial names the call, the grant in force and the line to add:
  `capability denied: fs.write(/tmp/.../hello.wj) ... to allow: add fs.write = [...] or run with --allow ...`. Finding the
  right grant for a script took one iteration per denial.
* Per-package grants (`[capabilities.dependencies.NAME]`) showed at once which package wanted what.
* `wj run` of a project with path dependencies needs no build step, so a package edit and a script run are one command.
* `wj test --interpret` runs a package's tests in about a second; the compiled `wj test` of the same package took 95 seconds.

What hurt:

* The span problem above: an error inside `${...}` points at a made-up column.
* Silent wrong behaviour in escapes (`"\0"` was `"0"`, `"\u{e9}"` was `u{e9}`), found only by comparing with Python output;
  the gate is what caught it.
* Keywords as names: `fn string(self)`, `use crate::int` fail to parse with "Expected function name"; the error does not say the
  name is reserved.
* `${...}` in an ordinary string needs `$${` to be literal, which is easy to forget when writing a shell snippet (`${PIPESTATUS[0]}`).

## Running the ports

```bash
wj run scripts/wj/port_gates.wj --wj target/release/wj           # all gate cases
wj run scripts/wj/port_gates.wj --wj target/release/wj --port spec_check
wj run scripts/wj/port_gates.wj --wj target/release/wj --record  # re-derive expectations from the originals
wj run scripts/wj/port_gates.wj --wj target/release/wj --time 3  # CPU-time table
```

The scripts need the ecosystem packages at `windjammer-ecosystem/` in the repository root (CI checks it out; locally,
`ln -s ../windjammer-ecosystem windjammer-ecosystem`).
