# Drift findings from agent pilot 6

Source: `bench/agent-dx/results/pilot6/` (arms R rust, U compiled-first, V2 interpreted-first with predictions and `wj run --verify`; 90 trials over 10 tasks), compiler
`wj 0.50.0` commit 82d5ac0579cd built from `next/v0.50.0`, 2026-11-11. Programs that the compiler rejected are in `results/pilot6/failing-sources/`; minimal programs for each class are in
[pilot6-repros/](pilot6-repros/) (interpreter and compiled behaviour checked on this build). Format as in [pilot5-findings.md](pilot5-findings.md).

Result in one line: no silent divergence in any trial (no U or V2 trial ended with a program the interpreter ran and the build rejected), every trial passed the hidden oracle, and every compiler
rejection that agents met was loud (a `WJ0003` from the compiled build, shown by `wj run --compiled` or `wj run --verify`). All entries below are therefore friction (extra turns, tokens, tool-wait), not wrong output.
`wj check` accepted every one of the rejected programs (exit 0, no prediction).

## F. Findings (compiler and tooling)

| Entry | Interpreter | Compiled / tooling today | Trials (sequential run) | Agent-program site |
|---|---|---|---|---|
| 1. `same_dir_name_runs_other_binary` (tooling, not a program shape) | correct | `wj run --compiled` and `wj run --verify` run the binary of whichever same-named project was built last: x, y, x prints `program Y` for the second x | 0 (the sequential run is immune); at least 7 calls in one trial of the discarded parallel run | every agent workspace is called `ws`; two trials in parallel shared the binary and `--verify` reported `stdout differs ... compiled: invalid input` for a program that was right |
| 2. `closure_if_else_ordering` | prints `bb cc a` | `WJ0003 expected Ordering, found void` (rustc E0308; location "approximate", points at the `else`) | 5 (U 2, V2 3, all t104); 5 more in the parallel run | `users.sort_by(\|a, b\| { let x = ..; if x != y { y.cmp(x) } else { a.cmp(b) } })`; same with `totals.get(a).unwrap_or(0)` inside (`closure_map_get_if_else_ordering.wj`, same message) |
| 3. `map_insert_indexed_vec_string` | prints `edit` | `WJ0003 expected string, found str` (points at an unrelated `Vec<string>` line) | 1 trial, 5 failing calls, 25 turns, USD 0.15 (V2, t104) | `let ps: Vec<string> = key.split_whitespace().collect()`, `let act = ps[1]`, `best.insert(u, act)` into `HashMap<string, string>` |
| 4. `nested_map_unwrap_or_new` | prints `1` | `WJ0003 expected &HashMap<string, i64>, found HashMap<_, _>` | 1 (V2, t104) | `let mut m = actions.get(u).unwrap_or(HashMap::new())`, then `m.insert(..)`, `actions.insert(u, m)` |
| 5. `adapter_into_vec_annotation` | prints `a b` | `WJ0003 expected Vec<string>, found Map<Iter<_>, closure>` | 1 (V2, t104) | `let names: Vec<string> = keyed.map(\|p\| p.1)`: the sheet's own advice ("adapters start at the collection") produces a lazy `Map` in the compiled build and a `Vec` in the interpreter |
| 6. `noise_warnings_in_build_output` | n/a | every compiled build that uses `fs` prints rustc `unused import` warnings for `windjammer_runtime::fs::Metadata`, `DirEntry` (and `json::Value`, `process::ProcessOutput`), and an index expression is emitted as `totals[(i as usize)]` (`unnecessary parentheses`) | every U trial had these in a tool result (no V2 trial did: `--verify` prints the compiled program's stderr only when outputs differ) | `use std::fs` plus `totals[i] += 2.5`; harmless to the build, ~300 tokens of tool output per call that an agent reads |
| 7. unminimised: `expected &_, found string` at a `HashMap<string, int>` annotation | prints the right counts | `WJ0003 mismatched types: expected &_, found string` | 1 (U, t104) | source: `results/pilot6/failing-sources/t104_activity_log.U.1.call0.wj` (`contains_key(u)`, `counts.insert(u, counts.get(u).unwrap_or(0) + 1)`, `users.push(u)` with `u = parts[0]`); the lines above compiled on their own in a 12-line extract, so the trigger is a combination in the larger program |

Notes:
- Entries 2 to 5 and 7 are the same kind of defect as the pilot 4 and 5 entries (interpreter green, compiled red, rustc's first error mapped to `WJ0003` with an approximate location), and none is in family A, B or C
  of ADR-0033, so `wj check` does not predict them. 2 is a new class (a block-bodied closure whose value is an `if`/`else`); 3 and 4 touch String against `str` and a reference to an owned map, closest to
  families B and C but with other generated shapes. The drift corpus (`tests/drift_corpus/repros/`) does not contain them yet; the repro files are written to be copied there.
- Entry 1 changes how to read earlier pilots: pilots 4 and 5 ran two trials at a time with the same workspace name, so some of their tool-wait, turn and "divergence" figures may include this effect.
  The graded final results are not affected (the hidden oracle builds each program in its own directory).
- The one "wrong output" the agents saw in the first run (`compiled: invalid input` for a task that never prints it) was entry 1, not a miscompile.
- `wj run --verify` blocks reading standard input until it is closed when it is started without redirected input; all trials pipe or redirect, so it did not occur. Not a defect for programs that read stdin
  (they need the input anyway), but a program that ignores stdin waits too; worth a bounded read or a note in the sheet.

## V2 and U failure classes by trial (counts of trials in the sequential run in which a compiler diagnostic was shown)

| Class | U | V2 |
|---|---|---|
| closure `if`/`else` as `Ordering` (entry 2) | 2 | 3 |
| string vs str via `Vec<string>` index (entry 3) | 0 | 1 |
| nested map `unwrap_or(HashMap::new())` (entry 4) | 0 | 1 |
| `.map` result bound to `Vec<string>` (entry 5) | 0 | 1 |
| unminimised `&_` / string (entry 7) | 1 | 0 |
| rustc warnings in tool output (entry 6) | 30 | 0 |
| silent divergence (interpreter green, build red, final) | 0 | 0 |
| prediction shown (any family) | 0 | 0 |

All the compiler diagnostics came from one task (t104). The other nine tasks needed no compiler diagnostic beyond the programs' own exit status 1 for invalid input.
