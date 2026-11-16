# Pilot 7 reference-solution repros

The Windjammer reference solutions of tasks t302 and t304-t309 (`bench/agent-dx/solutions/wj-set3/`) ran correctly
under the interpreter and failed the compiled half of `wj run --verify` on `next/v0.50.0` at b5444baa7. Every failure
was a compiler defect; no reference solution was wrong. Each file below is a minimal neutral program with its expected
output in `// expect:` lines: the interpreter prints it, the compiled build is rejected by rustc.

Fixed on `wt/pilot7-tasks` (cases in `tests/codegen_correctness_corpus/`):

| Corpus case | Defect | Tasks |
|---|---|---|
| `if_else_statement_value_tail.wj` | a non-final `if`/`else` statement whose branch ends in a value (`v.remove(i)`) kept the value as the branch tail | t302 |
| `index_by_borrowed_loop_var.wj` | a borrowed loop element used as an index was emitted `*(a as usize)` | t306 |
| `if_value_owned_and_borrowed_text.wj` | an `if`/`else` value mixing an owned `string` with a literal or a slice kept the borrowed branch | t307, t308 |

Fixed on `wt/width-11` (moved to `tests/codegen_correctness_corpus/`, judged by `tests/codegen_correctness_test.rs`):

| Corpus case | Defect | Tasks |
|---|---|---|
| `int_element_compared_with_len_and_used_as_index.wj` | an `int` element read compared with `.len()` and used as an index got mixed `i64`/`usize`; the element of a declared local `Vec` now has a static type, so the class is decided by the solver | t304 |
| `struct_field_read_moves_literal_to_i32.wj` | a struct field read through a local `Vec` held the solver class and the legacy decider printed `0_i32`; the field now has its declared type, the class is published | t305 |

Fixed on `wt/width-14` (pilot-7 trial repro, `tests/codegen_correctness_corpus/` and `tests/drift_corpus/repros/`):

| Corpus case | Defect | Tasks |
|---|---|---|
| `text_slice_range_bounds_from_int.wj` | the bounds of a text slice `s[i..i + 1]` with `i: int` sit at declared `usize` slots that were not recorded as converted at the argument, so the class was held and the literal printed `1_i32` beside `i: i64`; the bounds now keep the class of `i` and are converted where they are passed | t306 |

Open, with the owning stream:

| Repro | Defect | Owner | Tasks |
|---|---|---|---|
| `rebound_name_other_type_in_sibling_scope.wj` | an `int` local converted with `.to_string()` because a later `for n in names` binds `n` as a string | wt/ownership-9 (scope-aware local types) | t305, t308 |
| `for_over_mut_param_then_push.wj` | `for o in out` over a `&mut Vec` parameter moves it before `out.push` | wt/ownership-9 (for-mut loop shapes) | t307 |
| `self_field_arg_into_mut_self_method.wj` | `self.special(self.forms[h].s, ..)` borrows a field of `self` into a `&mut self` call | wt/ownership-9 (&T/T adaptation) | t307, t309 |
| `element_read_borrowed_across_container_write.wj` | `let r = self.orders[i]` is emitted as a borrow, then `self.orders[i].qty = ..` writes while `r` is live | wt/ownership-9 (&T/T adaptation) | t309 |
| `str_array_element_in_owned_branch.wj` | a `&str` element of a string-literal array as the value next to an owned `string` branch stays `&str` | wt/ownership-9 (&T/T adaptation) | t308 |
| `caller_param_name_suffix_of_field_arg.wj` | an argument `self.rooms` becomes `self.rooms.to_string()` when a caller `string` parameter's name is a suffix of the argument text | wt/ownership-9 (call-argument adaptation; a text match, so it also shrinks the heuristic surface) | t308 |

Diagnostics found on the way (fixed on `wt/pilot7-tasks`): compiled-build errors under `wj run --compiled` and
`wj run --verify` were reported at the generated Rust line number, because cargo names the file relative to the
package (`main.rs`) while the source map records the path the compiler wrote. WJ0031 told the reader to remove a
`*` that the source did not contain, and WJ0016 suggested ending a reference the source never wrote.

The open repros also run in the drift corpus (`tests/drift_corpus/repros/`), with compiled baselines recorded live
on 9316595e2 (all `exit1:0:17`, the build is rejected) and one ledger line each in
`tests/drift_corpus/known_divergences.txt` naming the stream whose merge clears it (ceiling 82 -> 90).

## Expectation for the pilot-7 preflight (`wj run --verify` on the references)

Measured on 9316595e2 (Mac), stdin = first non-empty vector of `tasks_set3/<task>/oracle/vectors.json`. The
interpreted stdout equals the oracle in all nine.

| Task | Verdict on 9316595e2 | Compiled-build errors | Expected after |
|---|---|---|---|
| t301 | pass | none | pass |
| t302 | pass (exit 1 is the program's own, the compiled run matches) | none | pass |
| t303 | pass | none | pass |
| t304 | compiled build fails | WJ0003 x2, WJ0017, WJ0020 | wt/width-11 merge |
| t305 | compiled build fails | WJ0003 x2, WJ0020 | wt/width-11 and wt/ownership-9 merges |
| t306 | pass | none | pass |
| t307 | compiled build fails | WJ0007, WJ0016 | wt/ownership-9 merge |
| t308 | compiled build fails | WJ0003, WJ0015 x2 | wt/ownership-9 merge |
| t309 | compiled build fails | WJ0016 x5 | wt/ownership-9 merge |
