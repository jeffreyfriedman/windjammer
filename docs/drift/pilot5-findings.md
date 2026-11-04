# Drift findings from agent pilot 5

Source: `bench/agent-dx/results/pilot5/` (second task set, arms V interpreted-first and W interpreted plus one final build, 60 trials, compiler `wj` built
from `next/v0.50.0` 03e15b0830ea, 2026-11-04) and the reference solutions in `bench/agent-dx/solutions/wj-set2/` (their authoring hit three of the same classes).
Format as in [findings.md](findings.md): the interpreter accepts and runs the program, the compiled backend's generated Rust is rejected, so programs of this shape are
"interpreter green, compiled red". Minimal programs: [pilot5-repros/](pilot5-repros/) (interpreter output and compiled diagnostic checked on this build). They are not in
`tests/drift_corpus/repros/` or `known_divergences.txt`; the drift owner should move them. Counts are silent-divergence trials (10 in total: 9 in V, 1 in W) unless stated.

## F. Findings

| Repro | Interpreter | Compiled diagnostic today | Trials | Agent-program site |
|---|---|---|---|---|
| `map_get_unwrap_or_float` | prints `1` | `WJ0031 `float` is not a reference, so it cannot be dereferenced` (generated `(*cur + t)` over an owned `f64`, and an `as f32` narrowing on the stored value) | 5 | `let cur = totals.get(k).unwrap_or(0.0)` then `totals.insert(k, cur + x)` (t101 3, t104 2); same class as pilot 4 `map_get_unwrap_or_deref` for `int`, still open |
| `map_get_in_sort_by_closure` | prints `b` | `WJ0017 trait bound not satisfied: `string: Borrow<&string>`` (generated `totals.get(&x)` where `x` is already `&String`) | 1 | `names.sort_by(\|x, y\| totals.get(x).unwrap_or(0) ...)` (t104) |
| `assign_split_word_to_mut_empty_string` | prints `alpha beta gamma` | `WJ0003 expected an owned `string`, found a borrowed `string`` (the help text proposes `.to_string()`, which fixes it) | 4 | `let mut cur = ""` then `cur = w` with `w` from `split_whitespace()` (t107, one W trial and all three V) |
| `keys_collect_vec_string` | prints `a` | `WJ0029 cannot collect `&String` items into `Vec<string>`` (R3 hint fires; lowering still wrong for `HashMap::keys()`) | 0 in agents, hit by the reference solutions of t101 and t104 | `let names: Vec<string> = m.keys().collect()` |
| `slice_into_string_binding` | prints `2024` | `WJ0003 expected an owned `string`, found a borrowed `string`` | 0 in agents, hit by the reference solutions of t105 and t107 | `let year: string = s[0..4]` |

Notes:
- 1 and 3 explain nine of the ten silent divergences. Seen once each in a discarded first run (a harness defect had masked exit statuses): `Fn(char)` closure mismatch for
  `s.find(c)`-style calls (t105) and an `i32`/`i64` mix (t109); neither minimised.
- The dropped-branch form `if c { st.pop() } else { st.push(..) }` compiles on this build in its minimal form; the reference solution of t108 hit it only inside a longer
  `else if` chain (`let _ = st.pop()` fixed it). Not minimised.
- Direction: all are compile-time failures, none corrupts output silently. Two further silent-looking records (t102 and t103) were grader flakes under load 40-60 (both modes
  pass on re-grade; fixed in the records by `pilot-regrade`).
