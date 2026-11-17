---
status: proposed
last_verified: 7a7a70823
---
# ADR-0037: Agent-cost levers, measured offline

- **Status:** Proposed (first slice landed: the friction corpus and replay, the offline cost proxies and their ratchet,
  quiet compiled runs, `--error-format agent`, lint W0017)
- **Date:** 2026-11-11
- **Related:** [ADR-0033](0033-interpreted-first-cost.md) (interpreted-first cost), `bench/agent-dx/results/pilot6/report.md`,
  `docs/drift/pilot6-findings.md`, `bench/agent-dx/friction/`, `tests/agent_cost_ratchet_test.rs`,
  `tests/agent_cost_levers_test.rs`, `docs/book/src/showcase/agent-cost.md`

## Context

As more code is written by agents, what it costs an agent to produce a correct program becomes a property of the
language and its tools, as much as speed or safety. Pilot 6 measured it: 90 trials, 10 tasks, the same model. Every arm
passed every trial; the mean cost per trial was lower for plain Rust than for either Windjammer arm, the differences
include zero, and Windjammer used more turns (3.8 and 4.8 against 3.0). The extra turns all trace to friction:

| Friction (pilot 6) | Trials | What it cost |
|---|---|---|
| Valid-looking programs the compiled build rejects (closure `if`/`else` as `Ordering`; `string` against `str`; `unwrap_or(HashMap::new())`; an adapter bound to `Vec`) | t104: every Windjammer trial | t104 cost 0.062 and 0.084 against 0.028; one trial spent 25 turns and USD 0.15 on one `string`/`str` error |
| `"$${d}.${c}"` prints `${d}.50`: `$$` is an escaped `$`; the program runs, the output is wrong, nothing says why | t103: every Windjammer trial | 6.3 and 7.0 turns against 3.0 |
| rustc warnings about the generated Rust and build progress in every compiled run | every compiled-first trial | about 300 tokens per call, read again on every later turn |
| The interpreted-first arm never used the interpreter: all but one run was `wj run --verify` | every V2 trial | a compiled build on every call; tool-wait 17 s against 11 s |

Pilots are expensive and noisy (three trials per cell). The levers below need a measurement that costs no LLM spend, is
deterministic, and runs on every change.

## Decision

### 1. A friction corpus and offline cost proxies

`bench/agent-dx/friction/` holds every failing `wj` call of pilots 3 to 6 (all arms) as an event: the command, the output
the agent saw, the program at that moment where it can be recovered (stored by pilot 6, or the trial's first call), a
class, and its status on the current compiler. `replay.wj` (interpreted Windjammer) replays every event and computes:

| Proxy | Key (`docs/generated/measured.json`) | Direction |
|---|---|---|
| (a) friction events still open, per task | `cost.friction.events_open`, `cost.friction.open_per_task`, `cost.friction.open_compiler_bugs` | shrink |
| (b) tokens per error (bytes / 4), human and agent format, kept apart by source: diagnostics of `wj check` and rustc errors of the compiled build (one mean over both moves with the mix of events alone) | `cost.diag.check_tokens_per_error_{human,agent}`, `cost.diag.build_tokens_per_error_{human,agent}` | shrink, each |
| (b) share of error codes seen in the corpus that carry a machine-applicable fix | `cost.diag.fix_coverage` | grow |
| (c) turns to green for an automatic fix-applier (apply every fix, re-check) | `cost.fixsim.*` | grow; `new_diagnostics` = 0 |
| (d) noise lines per compiled run (anything but the program's output and real diagnostics) | `cost.noise.lines_per_build` | shrink (target 0) |
| (e) CPU time of check, interpreted run, compiled run after an edit and warm, verify; small and medium program | `cost.time.*` | recorded on a quiet machine only |
| (f) tokens to write the same tasks, Windjammer against Rust (reference solutions; pilot-6 final programs) | `cost.write.*` | tracked |

`tests/agent_cost_ratchet_test.rs` holds the shrink and grow directions against `friction/ratchet.json`, so no change can
make them worse unnoticed. Every still-open compiler bug in the corpus is listed in `friction/open-bugs.md` with an
owner stream; `replay.wj` and the test fail on an unlisted one.

Classes: `compiler-bug` (valid code the toolchain rejects or mishandles), `user-error-good-message` (a real mistake with a
coded, located diagnostic and help), `unhelpful-message` (no code, location in generated Rust, or no help), `noise`,
`skipped-loop` (behaviour: an interpreted-first trial whose first run was not interpreted), `program-output` (the task's
own non-zero exit status; not friction). Events without a program keep their recorded class and are `unverifiable`.

### 2. Levers, ranked by expected tokens saved per unit of effort

| # | Lever | Expected effect | Effort | State |
|---|---|---|---|---|
| L1 | W0017: `$${` in a string is reported, with the `\$${` rewrite offered | t103 cost 3.3 to 4.0 extra turns in every Windjammer trial; on the pilot-6 mix about 0.35 turns per trial (a third of the U minus R gap) | small | done |
| L2 | Quiet compiled run: progress only on a terminal (`WJ_PROGRESS=1` forces it), no replayed rustc warnings | about 300 tokens per compiled call in every compiled-first trial; with 1.7 calls and the context re-read on later turns, 0.5k to 1.5k input tokens per trial | small | done |
| L3 | `--error-format agent` on `wj check` and `wj run` (and `WJ_ERROR_FORMAT=agent`): one line per diagnostic with location, code, message, first help and fix | the replay measures the token ratio per error (`cost.diag.check_tokens_per_error_*`); every error an agent reads, on every later turn | small | done |
| L4 | Fix the open compiler bugs of the corpus (`friction/open-bugs.md`) | t104 alone: 4 extra turns (compiled-first) and 9 (V2) per trial | medium to large, owner streams | tracked |
| L5 | One command that is the whole loop: `wj run --verify` (or a new `ship` subcommand) as the single documented step, printing one compact report (interpreted result, build verdict, output comparison, test summary) and stopping before the build when the interpreted run fails | agents already chose one command (`cost.loop.v2_*`); 1 to 2 tool results per trial shorter, tool-wait lower when the first interpreted run fails | medium | proposed |
| L6 | First-write success: the agent sheet and message wording for the top classes (this change: the `\$` line in `docs/agent/WINDJAMMER_FOR_AGENTS.md`; next: the adapter-to-`Vec` advice once corpus entry 5 is fixed or reworded) | removes a turn whenever the sheet's advice produced the failure | small each | in progress |
| L7 | A `sheet` subcommand: a sheet with only the modules the project imports | pack tokens are 450 to 530 per turn today; a third to half of that for a single-module task | small | proposed |
| L8 | Training-data presence: ecosystem packages and verified public samples | slow, compounding; the model has seen far more Rust | ongoing | proposed |

The cheap loop. When pilots 4 and 5 offered only `wj run --interpret`, interpreted-first agents used it in every
trial; when pilot 6 also named `wj run --verify` as the last step, every trial started with it (`cost.loop.v2_*`), and
nearly every first program was already correct. Agents run the most complete command they are given, once. Fighting that with
prompt wording is the wrong lever: the one command should be cheap (L5), not avoided.

## Consequences

- Cost becomes a gated property: the proxies are regenerated by two Windjammer scripts and checked by a Rust test; an
  improvement must tighten the ratchet in the same change.
- The proxies are proxies. They predict turns and tokens from the toolchain alone; only a pilot measures cost. Pilot 7
  (below) tests whether the levers move the real number.
- The quiet run hides nothing the user can act on: Windjammer's own lints and every compile error are still printed;
  only build progress (on a pipe) and rustc's warnings about generated Rust are dropped.
- The measurement may not be older than the code that feeds it: `ratchet.json` lists the `feeds` paths (diagnostic
  rendering, the run driver, the source map, the linter, the replay and its corpus), and the ratchet test fails when a
  ratcheted key was measured at a commit before the last change to any of them. Re-measure with `replay.wj --write` on a
  build of the head, commit `measured.json` and tighten the limits.
- Compiled-build errors, after the secondary-label and lint-block changes (9 events, bytes / 4 per error): human 148 -> 116,
  agent 81 -> 57. The agent limit was tightened to 57. The human limit is re-based 111 -> 116 rather than cut further:
  the 5 tokens over the old limit are the note lines for rustc's secondary labels that sit on a different line than the
  error (`expected due to this value (main.wj:34)`); the replay cannot say what they are worth (no fix applies to these
  errors, `cost.fixsim.reached_green` is 0), they are absent from the agent format, and a label on the error's own line
  is already dropped. If a pilot shows they do not shorten the loop, drop them and tighten the limit.
- W0017 is a warning with a `maybe_incorrect` fix: `$${` is occasionally meant literally, so `wj check --fix` never
  applies it.

## Pilot 7 plan (owner decision 2026-11-11; not run)

- **Question.** With the landed levers and a loop agents actually follow, what does a correct program cost in plain Rust
  and in Windjammer, per model, on tasks hard enough that pass rate can discriminate; and for Opus, where do the dollars
  go?
- **Matrix.** Models haiku, sonnet, opus x arms R (plain Rust, `rustc`, std only), U7 (Windjammer compiled-first, the
  current sheet without the loop bullet, `packs/pilot7/sheet_u.md`) and V3 (one command: `wj run --verify` with
  `WJ_ERROR_FORMAT=agent`, sheet `packs/pilot7/sheet_v3.md`) x 10 tasks, 2 trials per cell for haiku and sonnet and 1 for
  opus (150 trials; the owner's 180 with 2 opus trials per cell projects above the cap on the validation data, see Budget),
  one at a time. Configuration: `bench/agent-dx/agentdx/pilot7_config.json`; `python3 -m agentdx pilot7 --dry-run`
  validates it with no LLM call; `pilot7 --yes` launches (owner only); `pilot7-report` reports.
- **V3, from the pilot-6 transcripts.** V2 agents ran the most complete command they were given, once, first: all but one
  run was `wj run --verify`, none was `wj run --interpret`, and nine first programs in ten were already correct. When only
  `--interpret` was offered (pilots 4 and 5) they used it every time. V3 therefore names one command and nothing else, and
  makes it cheap: no build progress or rustc warnings on a pipe (L2), one line per diagnostic (L3), W0017 (L1).
- **Tasks.** t103, t104 (the pilot-6 friction tasks), t302-t305 and t306-t309 from `bench/agent-dx/tasks_set3/`
  (independent Python oracles, 41 to 69 vectors, Rust and Windjammer references passing every vector). Validation with a
  haiku arm-R smoke (USD 0.30 of the 1.5 allowed, `results/pilot7-validation/`): every first-round candidate passed in
  one shot, so the second round made the tasks larger (250-475 line references); there t309 failed both trials and t306
  one of two on hidden edge cases, and t307 took 13 turns once. No trial hit a rustc error: the current haiku
  writes compiling Rust for these shapes, so pass rate and turns, not compile errors, are what discriminate.
- **Budget.** Hard cap USD 35. Per-trial caps by model (haiku 0.10, sonnet 0.60, opus 1.50). Expected cost per trial
  comes from the validation tokens priced per model (the dry run prints it: arm R costs USD 0.011 with haiku, and the same
  tokens would cost 0.21 with sonnet and 0.42 with opus), raised for the Windjammer arms' extra turns: haiku 0.015, sonnet
  0.25, opus 0.45, total 29.40 for the 150 trials. After the first 6 trials of each model the guard projects the total from
  that model's observed mean and stops that model if the projection breaks the cap; models run cheapest first, so a stopped
  opus leaves the haiku and sonnet results intact.
- **Cost decomposition (every trial).** Fresh input, cache write, cache read and output tokens, priced with the dated
  table in the config; thinking tokens as an estimate (Claude Code reports output tokens including thinking, and the
  current models return thinking blocks without text, so thinking = output minus visible text and tool inputs, marked
  as an estimate); tool-result tokens; turns; lines of the final program and tokens in the write calls; the computed
  total checked against Claude Code's reported cost per trial. The report answers, for Opus: the share of context, visible
  output and thinking; the attribution of the R-versus-U7/V3 difference to context, visible output (of which the program
  text), thinking and turns; and what 1k output tokens and 1k context tokens per turn are worth at Opus prices.
- **Launch gates** (all green before `--yes`):

| Gate | Status at this commit |
|---|---|
| Run identity: same-named projects never share a compiled binary (`docs/drift/pilot6-repros/same_dir_name_runs_other_binary.md`) | open: no fix on `next/v0.50.0` yet (the harness runs trials one at a time, which avoids it, but the gate stays) |
| Pilot-6 compiler bugs fixed (`bench/agent-dx/friction/open-bugs.md` has no t104 entry) | open: every t104 class still fails on the current compiler (closure values, ownership, adapters); owner streams in `open-bugs.md` |
| Noise-free compiled runs (`cost.noise.lines_per_build` = 0) | met on this branch (gated by the ratchet) |
| `--error-format agent` and W0017 merged | on this branch, awaiting merge |
| Tasks validated (Rust features, haiku R smoke) and listed in the config | done (10 tasks; see Tasks) |
| `pilot7 --dry-run` prints `ready` | ready (with `AGENTDX_RUSTC` set to the real rustc, as pilot 6) |
| The `wj` used is fresh (`wj doctor` clean) and recorded per trial (`AGENTDX_WJ_VERSION`) | at launch |

- **Kill criteria.** A model's runs stop when its projection breaks the cap. If pass rate is at the ceiling for every
  model and arm, the task set is too easy and is replaced before any conclusion. If V3 minus R cost stays above zero
  with an interval excluding zero, the next levers are L4 (compiler bugs) and L5 (one compact command).

## Pilot 7 follow-up: compiler-side fixes (agent-dx stream)

From `private-docs/pilot7-analysis.md`; each is gated.

- **`wj run --verify` verdict (analysis fix 5).** When the interpreted and compiled runs agree (same stdout, same exit
  status) it prints `wj verify: OK, the compiled run matches the interpreted run (stdout N bytes, program exit status C)` and
  exits 0 whatever C is; a difference, a failed build or a refused program still exits non-zero (the interpreted status
  when that is non-zero, else 1). 44% of recorded failing runs (51% in V3) were a correct program exiting 1 on purpose, and
  the sheet said to re-run until it exits 0. Gates: `tests/run_verify_test.rs`, promise cases for `wj run --verify`.
- **Sheet idioms (fix 6).** The six idioms agents looked up with `wj docs` before writing are one executed block in the
  pilot-8 candidate sheets (`bench/agent-dx/packs/pilot8/`, run by `tests/agent_docs_test.rs`). The default sheet is
  unchanged: its budget rises only with a measurement. The design, cost and decision rules are in
  `bench/agent-dx/PILOT8_SHEET_AB.md` (not run).
- **`mut self` (fix 9).** One diagnostic, WJ0011, in `wj check`, `wj run` and `wj build`, located at the `mut` token, with
  a machine-applicable fix (`wj check --fix` deletes it); the parser recovers and the receiver is plain `self`.
- **WJ0051 and partial moves (fix 10).** The agent format says where an insertion goes (`=> insert `mut ` at 2:9`), a
  deletion by span. A field moved into an assignment (`self.last = e2.text`) while the struct is read again afterwards
  is copied by the existing CLONE-USE-AFTER-MOVE rule (`field_copy_sites`); the assignment path did not apply it.
- **Agent mode output (pilot 8 finding).** Agents appended `2>&1 | head -40`, `$?` and `${PIPESTATUS}` to `wj run --verify`
  and were denied. With `WJ_AGENT=1` or the agent format, `wj run|check|build|test` re-run themselves with stdout and stderr
  in one pipe, bounded to 160 lines (`WJ_MAX_OUTPUT`) with an omitted-lines marker, ending in `exit status: N`
  (`src/cli/agent_wrap.rs`; the child is marked `WJ_AGENT_CHILD`). Human output is untouched. Gates: unit tests of the plan
  and the budget, promise cases for `wj run`, the pilot-8 sheet gate, and the arm prompt (`AGENT_MODE_TEXT`).
- **Harness shapes (pilot 8 failure analysis).** Denied: `$?`, redirects, a guessed `./build/main`, `$()`, a `printf`
  argument starting with `=` (the shell's `=cmd` expansion), heredocs. `wj run --stdin-file FILE` gives input from a file the
  agent wrote with its Write tool (no `printf`, no quoting); in agent mode `wj build` ends with `run: wj run --compiled FILE`;
  the pilot-8 sheets and prompt say both, and `agent_sheets_teach_no_denied_shell_shape` keeps denied shapes out of every
  sheet example. Which quoting of a leading `=` the harness accepts is untested here; `--stdin-file` makes the question moot.
