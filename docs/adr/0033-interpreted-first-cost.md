---
status: proposed
last_verified: 4424fe873
---
# ADR-0033: Interpreted-first iteration without the final-build penalty

- **Status:** Proposed (first slice landed: the compiled-verdict cache and the width prediction of family A)
- **Date:** 2026-11-09
- **Related:** [ADR-0030](0030-interpreter-rejects-what-rustc-rejects.md) (predict rustc's ownership errors from the generated Rust),
  [ADR-0022](0022-interpret-by-default.md) (interpret by default), [ADR-0021](0021-effects-and-capabilities.md) (effects and grants),
  [ADR-0028](0028-usize-class-in-the-numeric-solver.md) (widths), `docs/drift/findings.md`, `docs/drift/pilot4-findings.md`,
  `docs/drift/pilot5-findings.md`, `docs/testing/INTERPRETER_CONTRACT.md`, `bench/agent-dx/results/pilot4/report.md`,
  `bench/agent-dx/results/pilot5/report.md`, `src/check/widths/`, `src/cli/verdict.rs`

## Context

Agent pilots 4 and 5 ran the same tasks three ways: compiled-first (`wj build`/`wj run --compiled` in the loop),
interpreted-first (`wj run --interpret` only) and interpreted with one final `wj build`. The hidden oracle grades the
compiled program in every arm.

| Pilot 5 (sheet `sheet_noloop.md`) | U compiled-first | V interpreted-first | W interpreted + final build |
|---|---|---|---|
| pass@1, compiled oracle | 1.000 | 0.667 | 0.967 |
| silent divergence (interpreter green, build red) | 0.000 | 0.300 | 0.033 |
| cost per trial, USD | 0.043 | 0.026 | 0.038 |
| wall seconds | 27.1 | 15.2 | 22.3 |
| tool-wait seconds (`wj` commands) | 6.5 | 0.2 | 3.9 |
| agent turns | 6.7 | 4.3 | 6.5 |

Pilot 4 points the same way: compiled-first K 1.000, interpreted-first L 0.600, interpreted plus final build M 0.833
(N, the same with the v2 sheet, 0.967); costs 0.041, 0.024, 0.034 and 0.042 USD; tool-wait 19.9, 0.2, 10.5 and 12.0 s.

Interpreted-first is the cheapest loop by far (about 0.6 of the compiled-first cost and wall time), but a third of its
trials end with a program the compiled build rejects, and the agent never sees it. Adding the final build recovers the
pass rate (0.967) and gives back most of the savings: the build is slow on a loaded machine, and fixing its errors
costs two more turns, far from the edit that caused them.

Every silent divergence in both pilots is a compile-time rejection; none corrupts output. The rejected programs fall
into a few families (rustc's first error on the generated code; `docs/drift/findings.md` sections A, B, I and the pilot
findings). Measured on this head over the drift ledger's compiled-red programs and the pilot repros
(`wj build --no-cargo`, then `rustc --emit=metadata`):

| Family | rustc error | Ledger programs whose first error it is | Pilot trials |
|---|---|---|---|
| A. integer literal typed `i32` beside `int` | E0308 `expected i64, found i32` | 11 | 3 (pilot 4 `let_mut_zero_then_parse`), 1 more in a discarded pilot 5 run |
| B. a `&T` expected for a lookup or an adapter | E0308 `expected &i64, found i64`, E0277 `Borrow` | 8 | 1 (pilot 5 `map_get_in_sort_by_closure`) |
| C. `String` against `&str` at a call boundary | E0308 | 4 | 4 (pilot 5 `assign_split_word_to_mut_empty_string`, fixed since) |
| D. a binding mutated through a method without `mut` | E0596 | 23 test cases of one app | 0 |
| E. use after move | E0382 | (ADR-0030, refused today) | 0 |
| other E0308 shapes (float widths, nested `Vec` by value, `&mut` places, fn pointers) | E0308 | 9 | 7 (`map_get_unwrap_or_*`) |
| other errors (`collect` element types, `Box<dyn Fn>`, unresolved names) | E0277, E0593, E0425, ... | the rest | 11 (`collect_turbofish` 8, `dyn_box_param` 2, `nth_cast` 1; fixed or ledgered) |

Two facts shape the decision. First, the families are code-generation defects or inference gaps (ADR-0030's third
class), so the lasting fix is in the backend, family by family, and the drift gate tracks that. Second, the agent
loop needs the verdict now, while those fixes land: an interpreted run that is green must mean the build is green, or
must say that it is not.

## Options

### (a) Speculative background compile of the last-saved state (landed as an opt-in: `--speculate`)

While the interpreter runs, a detached `wj build` of the same content runs at low priority and records its verdict
(built, or the rustc errors) under a key of the content and the compiler build. The next interpreted run prints the
verdict after the program's output (`wj: note: the compiled build of this exact program failed ...`); a status
command that lists verdicts does not exist yet.

- **Agent value:** high for agents that run more than once. Pilot 5 interpreted-first trials made 1.8 verify calls on
  average (1.5 to green), with 4 to 10 s of agent thinking between calls, longer than a warm compiled build of a small
  program (about 1 s of CPU when the runtime is cached; 6.5 s of tool-wait in the compiled-first arm under load 29).
  For the trials that stop after their first green run it comes too late: the verdict needs a second call.
- **Machine cost:** one compiled build per distinct saved content. On a shared machine (ten agents, three build slots)
  this can double the build load if every save compiles. Budget: at most one speculative build per user at a time;
  a newer save cancels the older one (only the last-saved state matters); `nice` 19; skip entirely when the load
  average exceeds the core count or free memory is under 2 GB; a small program's rustc peaks at about 300 MB.
  Kill criterion: the pilot's machine load rises by more than 10% or tool-wait of other arms rises.
- **Determinism:** the verdict is a function of the key (content of every input, manifest, lock file, dependency
  sources, crate surfaces, compiler build). Builds run `--offline`; a failure that is not a rustc error (a fetch,
  a linker) records nothing, so a verdict never depends on the network.
- **Effects and grants:** compiling runs no program code of the user. It does run build scripts of Rust crate
  dependencies, so a speculative build of a project with Rust crate dependencies runs only when those crates are
  already built for the same lock file (no new build script ever runs in the background), and never reaches the
  network (`--offline`). Grants (`WJ_CAPS`, `[capabilities]`) are not needed for compiling and are not widened: the
  background job never runs the program.

### (b) `wj run --verify` (landed as the second slice)

One command for the end of an iteration: interpret, then build (a cache hit when (a) already built this content) and,
when it is safe, run the compiled binary and compare stdout and the exit status.

- **Agent value:** replaces the separate final `wj build` of arm W with one call that the sheet names as "done";
  a cache hit makes it as fast as the interpreted run.
- **Effects:** running the program twice repeats its effects. The compiled run happens only when the effect analysis
  (ADR-0021) finds no sink beyond stdout and stderr, stdin is captured once and replayed, and the compiled run gets
  exactly the grants of the interpreted run. Otherwise `--verify` builds and reports "built, not re-run (effects:
  fs.write)", which is still the verdict that matters.
- **Determinism:** the build verdict is exact. The output comparison inherits the documented differences of the
  interpreter contract (hash iteration order, clocks, randomness, thread interleaving): a difference is reported with
  that caveat, never as a failure of the program, and `--deterministic-time` has no compiled counterpart.
- **Cost:** a cold build is the same as `wj build`; the gain over arm W comes from (a)'s cache and from one call instead
  of two.

### (c) Widen ADR-0030's prediction to the E0308 families in `wj check`

Read the generated Rust (already produced in-process for the move prediction) and report, as the existing WJ0003, the
places where rustc would reject a type mismatch, for the families above, each only where the generated text makes both
types certain.

- **Agent value:** highest per trial: the error appears in the first interpreted run, at the line, with the
  workaround, before the agent believes it is done. Pilot data: families A to C account for the larger part of
  the ledger's compiled-red programs, and family A also appears in agent trials.
- **Which generated Rust:** the checker's in-process generator (the one WJ0007 reads) does not run the build's
  multi-pass width inference, and its integer widths differ from the build's: read alone, it flagged nine corpus
  files whose built Rust rustc accepts (the build had typed the same literals `i64`). Widths must therefore be read from what `wj build` emits. The slice uses the
  in-process text as a trigger only and confirms on the build's own output (see the first slice).
- **Cost:** a syntax walk over generated Rust for every file (milliseconds), plus one transpile (about 1.1 s of CPU for
  a small file) for a program the trigger flags. A faster build-faithful generator in-process is the next step.
- **Risk:** a false positive fails `wj check` on a program that builds. Every family lands with a precision scan over the
  repository corpora with rustc as the oracle, and a pinned list, as WJ0007 did.
- **Families and shapes:**
  - A: a suffixed `i32` literal (or a binding, field, parameter, return value, `Option` payload, `vec!` element
    of that width) meeting `i64` in an arithmetic or comparison operator, an assignment, an annotated `let`, a call of
    the file's own function or method, a struct field, `unwrap_or`, or the arms of a `match`/`if` (this slice).
  - B: a lookup that needs a reference (`HashMap::get`/`contains_key`/`remove`, `HashSet::contains`,
    `Vec::contains`) given an owned integer or `String` whose type is certain; an iterator adapter's closure parameter
    used as a value. Needs receiver types of collections; same walker.
  - C: a `String` parameter (of the file's own function or closure) given a `&str` literal or a `&str` binding, and
    `String + String`. Needs `String`/`&str` in the walker's type view.
  - D (E0596): `let x = ...` without `mut` followed by `x.m()` where `m` takes `&mut self`. Investigated after the
    first slice: `wj build` infers the `mut` correctly both for a single file and for a multi-file program (a method
    that mutates `self` declared in another module), so the Rust a build emits has nothing to predict. The ledger's
    E0596 cases (the `wj-webhook` app) come from the test crate `wj test` generates for an app's `tests/`, where the
    library's receiver modes are not visible to the auto-mut inference. That is a backend fix in the test-crate path,
    not a prediction; it moves out of this list.
  - E (E0382): ADR-0030.

### (d) Permissiveness policy: what an interpreted run does with a certain prediction (Decided, 2026-11-09)

The interpreted-run gate (`cli::check::require_clean`) refuses every error `wj check` reports, including ADR-0030's
WJ0007 (a language rule: the program is wrong). The families of (c) are different: the program is right and the
backend is wrong. Two policies were tried on this slice:

- **Refuse** (the program does not run). Measured consequence: the drift gate's own tool `scripts/wj/spec_oracle.wj`
  stopped running, because its compiled build really does fail (`let mut idx = -1` becomes `-1_usize`). Repository
  scripts, which are only ever interpreted, would all be exposed to backend defects they never meet.
- **Warn** (the prediction is printed as a warning before the program's output, the program runs). The agent sees
  the same text, at the same moment, with the workaround; a script that is never built keeps working.

**Decided by the owner (2026-11-09): warn mode.** Certain predictions of backend defects are **errors in `wj check`**
(exit status 1, so a check-then-run loop stops) and **warnings in `wj run --interpret`, which then runs the program**.
The warning says plainly what happened: the program is correct and ran fine in the interpreter; rustc is stricter
than the interpreter and would not accept the Rust the compiler generates, so `wj build` would fail. It is a build
error, not a crash. The warning goes to stderr only; stdout and the exit status are the program's
(`tests/width_prediction_test.rs::the_interpreted_warning_never_changes_stdout_or_the_exit_status`). Uncertain shapes are never reported;
(a) and (b) report the build's verdict instead. Language-rule predictions (WJ0007) keep refusing. A backend fix for a
family turns its predictions off by construction: the generated text no longer mixes widths. Revisit after the next
pilot: if agents run without reading warnings, switch interpreted runs to refuse unless the program is a declared
script (a `wj.toml` setting).

## Decision proposed

1. **Adopt (c) with the policy of (d), family by family, in the order A, B, C** (D is a backend fix, see above). Each family is one slice with a
   precision scan (rustc on the generated code is the oracle) and a pinned list; any false positive that one revision
   cannot remove drops the family. The prediction reads the Rust the build emits, never a parallel analysis.
2. **Adopt the compiled-verdict cache now** (`src/cli/verdict.rs`) as the substrate of (a) and (b): every compiled build
   that reaches rustc records its verdict under the content and compiler key; an interpreted run of the same content
   prints the verdict when it is a failure.
3. **Adopt (b) next**, as the documented last step of an interpreted-first iteration in the agent sheet
   (`docs/agent/`), with the effect rule above. It needs a promise registry entry and acceptance cases (a negative one:
   a program with a file write is built, not re-run).
4. **Adopt (a) behind an opt-in first** (`[dev] speculative_build = true` in `wj.toml`, or an environment switch for
   agent harnesses), with the CPU and memory budget above. Make it the default only if the next pilot shows the budget
   holds on the shared machine.
5. **Fix the families in the backend in parallel.** The predictions and the verdict footer are the safety net, not the
   fix; the drift ledger stays the list of work.

## Cost model

Per trial, with pilot 5's numbers: interpreted-first costs 0.026 USD and passes 0.667; adding a final build costs
0.038 and passes 0.967. Counting a silent failure as a trial that must be redone in full (an upper bound on what it
costs the user), expected cost per correct program is about 0.039 for both V and W and 0.043 for compiled-first.
The target is V's price at W's pass rate:

- (c)+(d) move the failure into the first run. The fix then costs one edit and one rerun in a small context (about
  0.004 USD and 1 to 2 s of interpretation, from V's per-turn numbers), instead of W's extra build and two turns.
- (b) with a cache hit costs one interpreted run; with a miss, one build (W's 3.9 s of tool-wait).
- (a) moves the build off the critical path: its CPU is spent while the agent thinks.

If (c) covers the families that make up most silent divergences and (b) catches the rest, the expected cost per
correct program approaches 0.028 to 0.030 USD.

## Acceptance metric for the next agent pilot

New arm V2: interpreted-first with this ADR's predictions and footer, and `--verify` named in the sheet as the last
step once it exists; same tasks, at least 30 trials per arm, compiler built from the integration head.

- pass@1 (compiled oracle) at least 0.95, with the bootstrap interval of V2 minus W including zero or positive;
- cost per trial at most 0.030 USD (V plus 15%) and tool-wait at most 1.0 s;
- silent divergence at most 0.05;
- no trial shown a prediction for a program the compiled build accepts (precision in the field, from the records);
- machine load during V2 trials within 10% of the V arm.

Kill criteria: if V2's pass@1 stays below W's after families A to D land, the remaining divergences are outside the
predictable families, and `--verify` (or the opt-in background build) becomes the recommended default loop instead.

## First slice (landed)

- **Compiled-verdict cache** (`src/cli/verdict.rs`): key = SHA-256 of the compiler identity (version, executable path,
  size and modification time, the check caches' identity) and every input of the program (`check::program_inputs`:
  canonical paths and texts of the sources, manifest, lock file, dependency sources, crate surfaces). Entries are one
  JSON file each under `<cache root>/verdicts/`, written atomically; the directory keeps the newest 512. `wj build` and
  `wj run --compiled` record a verdict when cargo succeeds or when rustc reports errors (a failure without a rustc
  error records nothing). `wj run --interpret` prints a one-line note on stderr after the program when the compiled
  build of the same content failed; stdout is untouched. Gate: `tests/verdict_cache_test.rs` (key follows content and
  compiler, round trip, bound, no verdict without a rustc error, an edit misses, the binary prints the note).
- **Family A prediction** (`src/check/widths/` for the walker, `src/cli/widths_gate.rs` for the gate): a scoped walk
  over generated Rust types integer expressions where the text makes the type certain and reports every place two
  widths must be one type. The checker's in-process Rust is the trigger; the build's own output (`wj build --no-cargo
  --no-lint`, a child process inside `wj`) is what is reported on. `wj check` reports WJ0003 errors; `wj run
  --interpret` prints them as warnings and runs. Gate: `tests/width_prediction_test.rs`.
  - Reported: `let_literal_captured_by_closure`, `literal_in_option_field_arm`, `shadow_match_binding_literal`,
    `shadowed_option_literal_arms`, `62_vec_alias_semantics`, the value-position programs `closure_params`,
    `closures_compiled` and `option_arm_values`, the inference-corpus program `closure_len_width`, a library fixture,
    a spec program (`SPEC-CLOS-009`) and a compute shader written as `.wj`; outside the scanned corpora, `scripts/wj/spec_oracle.wj` (its build emits
    `-1_usize`).
  - Precision: the scan over every `.wj` file under `examples/` and `tests/` (with the drift corpus's single-file
    repros; multi-file programs of `tests/project_corpus` judged whole from their `main.wj`) reports only the pinned
    files, and each is checked with rustc (E0308) on the build's output inside the test. The recorded goldens of the
    convergence gates give a second oracle: every golden the walker flags is in `tests/golden/rustc_rejects.txt`.
    The first version read the checker's in-process Rust alone and flagged nine files whose build rustc accepts; that
    is why the build's output is the one reported on.
  - Recall: five of the eleven family-A ledger programs. The others need iterator element types, map value types or
    a runtime extension trait (`wj_zip_with`).
  - Cost: a clean program pays the trigger only (no measurable change in `wj check` time on `bench/interp/structs.wj`,
    0.00 to 0.03 s of user time before and after); a flagged program pays one transpile (about 1.1 s of CPU).
  - The documented workaround (`let five: int = 5`, `1 as int`) builds and is not reported.
  - Mutation check: with the width comparison of `conflict` replaced by `true`, the corpus scan, the golden
    cross-check, the project scan, the positive cases and the workaround case fail; restored, all pass.

## Second slice (landed): `wj run --verify`

- `wj run --verify FILE` (`src/cli/verify.rs`) runs the interpreter (`wj run --interpret` in a child process, its
  stdout captured and printed once), then builds exactly what `wj run --compiled` builds (`run::build_native`, which
  records the compiled verdict), then runs the compiled program a second time only when the effect analysis finds that
  `main` reaches nothing beyond the console and a read of `/dev/stdin`, and stdin was captured (it is read once and
  given to both runs). No grant is involved in such a run. Otherwise it reports the build's verdict:
  `wj verify: compiled build OK; not run a second time (fs.write)`.
- Everything it says is on stderr, prefixed `wj verify:`. Exit status: the interpreted program's when that is
  non-zero; otherwise 1 when the build fails (`the interpreter ran the program, but the compiled build failed ...`) or
  the compiled run differs (first differing line, both exit statuses, the contract's documented differences), else 0.
  A program `wj check` refuses is refused once, and nothing is built.
- It conflicts with `--compiled`, `--release`, `--watch`, `--hot`, `--record` and `--replay`, and takes only the Rust
  target. Gates: `tests/run_verify_test.rs` (stdin replay and exit status, a file write built but not run again, a
  family-A program whose build fails, a refused program, the difference locator, the effect rule) and the promise
  registry entry `wj run --verify` with a positive and a negative acceptance case.
- Not yet: the build is not speculative (option a), so a cold `--verify` costs one compiled build; the interpreted run's
  stdout is printed when it ends rather than streamed.

## Third slice (landed): families B and C

- The same walker and gate (`src/check/widths/`, `src/cli/widths_gate.rs`) now also type `String`, `&str`,
  `HashMap`/`BTreeMap`, `HashSet`/`BTreeSet` and closures bound to locals, and report:
  - B: a lookup (`get`, `get_mut`, `contains_key`, `remove` on a map; `get`, `contains`, `remove` on a set; `contains`
    on a `Vec`) given an owned integer or `String`, and an owned value where a reference parameter, field or
    binding is required (or the reverse);
  - C: a `String` where `&str` is required or a `&str`/`&String` where `String` is (parameters of the file's
    functions, methods and annotated closures, fields, annotated `let`s, assignments, `match`/`if` arms), and
    `String + String` / `String += String`.
- One level of deref coercion is honoured behind a reference (`&String` for `&str`, `&&T` for `&T`), so only shapes
  rustc never accepts are reported. `tests/prediction_families_test.rs` checks eleven positive and eleven negative
  shapes against rustc itself.
- Corpus scan: every file with a prediction is again confirmed by rustc (E0308) on the build's output, and pinned.
  New on the verified programs: `hashset_basics`, `prog_lru_cache`, `98_bignum_factorial`,
  `174_closure_mut_capture`, three engine-parity programs (`map_get_match`, `match_arms`, `match_value`) and the
  repros `explicit_ref_literal_arg` and `closure_string_param_literal`. Programs `wj check` already rejects for a
  type error (the check-negative mutants, error-system fixtures) are reported too; their builds fail the same way.
- On this head `literal_in_option_field_arm` builds (`Some(v - 1_i64)`): the backend fixed it, and the prediction
  went away by construction. Its drift-ledger line is now stale (for the ledger's owner).

## Fourth slice (landed): `wj run --interpret --speculate` (opt-in)

- `src/cli/speculate.rs`: with `--speculate`, an interpreted run starts `wj build` of what `wj run --compiled` would
  build, detached, at `nice -n 19`, with `CARGO_NET_OFFLINE=true`, into a scratch directory it removes afterwards;
  `wj build` records the verdict (`cli::verdict`), and the next interpreted run of the same content prints the
  failure footer. The run itself never waits and prints nothing about it.
- Bounds, each checked before starting: no build when the verdict for this content and compiler is recorded; one
  speculative build per user (a pid file in the cache root, a live pid is never doubled); none when the 1-minute load
  average is at or above the core count; none for a project with Rust crate dependencies (their build scripts would
  run). Compiling runs none of the program's code and needs no grant.
- Gates: `tests/speculate_test.rs` (each bound, stdout and exit status unchanged, the run does not wait, the flag
  conflicts with `--compiled`) and the promise registry entry `wj run --speculate` with a positive and a negative case.
- Not measured yet: the CPU it adds on the shared machine during a pilot (the kill criterion above). On this machine
  the load average is usually above the core count, so the bound keeps it off most of the time; that is the intended
  behaviour for an opt-in on a busy host.

## Measurement (offline, no LLM calls): what `--verify` would have told the pilot-5 agents

Method: the final program of every pilot-5 trial in arms V (interpreted-first) and W (interpreted plus final build),
taken from the trial records (`final_source`), run with `wj run --verify` on sample 0 of its task, on the head of this
slice with a warm shared build cache. The six `t110_crc_lines` trials are left out: their workspace declared the
`crc32fast` crate in a `wj.toml` that the records do not keep. A live pilot arm with `--verify` in the prompt costs
LLM calls and is an owner decision; this replay is free.

| Outcome on this head (54 trials) | Trials |
|---|---|
| compiled build OK, compiled run matches the interpreted run | 45 |
| compiled build OK, not run a second time (effects beyond the console) | 5 |
| the interpreter ran it, the compiled build failed (`t104_activity_log` V, all three) | 3 |
| the program fails in the interpreter too (`t102_file_tally` V.0, a non-interpreted std call) | 1 |

- The pilot recorded ten silent divergences. On this head the backend fixes since pilot 5 made `t101` and `t107` build
  and match; the three that remain (`t104`, `map_get_in_sort_by_closure`, a closure returning `()` where `Ordering` is
  expected) are exactly the trials `--verify` flags. No passing trial was flagged: `--verify` gave no false alarm.
- `wj check` predicted none of the three (their first rustc error is outside families A to C), which is why `--verify`
  is the complement of the predictions rather than a replacement.
- Cost: median 2 s, maximum 10 s of wall time per `--verify` with a warm cache, against 3.9 s of mean tool-wait for arm
  W's separate final build in the pilot.
- Projection, not a measurement: an interpreted-first arm that ends with `--verify` would have seen every remaining
  divergence at the cost of one call; the pilot acceptance metric above is what confirms it.

## Consequences

- `wj check` on a family-A program now fails with WJ0003 at the line, with the workaround; an interpreted run prints
  the same as a warning before the program's output.
- The verdict footer only appears after a compiled build of the same content; it is the hook (a) and (b) will fill.

## Decision needed from: project owner

1. Decided: the policy in (d), warn mode.
2. The order of the next slices: `--verify`, then B and C, then the opt-in background build; D goes to the backend
   (auto-mut in the `wj test` crate).
