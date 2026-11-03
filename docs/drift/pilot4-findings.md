# Drift findings from agent pilot 4

Source: `bench/agent-dx/results/pilot4/` (arms L, M: interpreted-first and interpreted plus one final `wj build`, 60 trials, compiler built from
`next/v0.50.0` e0ec926dc plus harness changes, 2026-11-03). Format as in [findings.md](findings.md): the interpreter accepts and runs the
program, the compiled backend's generated Rust is rejected (agents see `WJ0003`/`WJ0029`/`WJ0031`). Minimal programs are in
[pilot4-repros/](pilot4-repros/) (interpreter output and the compiled diagnostic were checked on this build); they are not yet in
`tests/drift_corpus/repros/` or `known_divergences.txt`: the drift owner should move them there. Counts are silent-divergence trials out of the 60.

## D. Findings

| Repro | Interpreter | Compiled diagnostic today | Trials | Site in the agent programs |
|---|---|---|---|---|
| `collect_turbofish` | prints `4` | `WJ0029 cannot collect `&str` items into `Vec<string>`` (the R3 hint fires; the lowering is still wrong) | 8 | `line.trim().split(" ").collect::<Vec<string>>()` and `let p: Vec<string> = line.trim().split(" ").collect()` (t006, t011, t013, t034) |
| `let_mut_zero_then_parse` | prints `5` | `WJ0003 expected an owned `int`, found an owned `int`` (both sides print the same; rustc says `i32` vs `i64`) | 3 | `let mut k = 0` then `k = line.trim().parse::<int>().unwrap_or(0)` (t018, t003) |
| `map_get_unwrap_or_deref` | prints `1` | `WJ0031 `int` is not a reference, so it cannot be dereferenced` (generated `*n + 1` over an owned `i64`) | 2 | `let n = counts.get(key).unwrap_or(0)` then `counts.insert(key, n + 1)` (t018) |
| `dyn_box_param` | prints `9` | `WJ0003 expected `&Box<dyn Shape>`, found `&Box<Square>`` (parameter `Box<dyn Trait>` is borrowed by inference; the argument is not coerced); a second error on the same program calls the trait method `name` as `file_name` | 2 | `fn show(s: Box<dyn Shape>)` called with `show(Box::new(Square { .. }))` (t034) |
| `nth_cast` | prints `7` | `WJ0003 expected `uint`, found `int`` (the explicit `r as usize` cast is lost when the value feeds `chars().nth(..)`) | 1 | `digits.chars().nth(r as usize)` (t006) |

Two further compiled-only sites appeared (`last[k] = l` with `k`, `l` from loop arithmetic in t003, and a `usize` loop variable in t006) but did not minimise on this
build: the nearest minimal form (`index_assign_loopvar`) compiles and agrees. They stay in the pilot records (`t003_prime_counts.L.1`, `t006_base_convert.L.2`).

Direction and fix hints: all five are "interpreter green, compiled red" at compile time, so none corrupts output silently; the cost is a failed final build. 1 and 2 are the
largest (11 of 60 trials). For 2 the diagnostic itself is a defect (identical type names on both sides): `diagnostics/rustc_text.rs` should keep the raw `i32`/`i64` pair when the
`wjify` spellings collide.

## E. Grading note (harness, not a drift finding)
14 of 120 first compiled grades under machine load 25-35 failed all vectors for programs that pass on a quiet re-run (`wrong-output`, 0 passed). The harness now re-grades a
compiled failure once when the interpreter passes everything, and records the first result (`first_compiled`); the pilot-4 records were corrected with `pilot-regrade`.
