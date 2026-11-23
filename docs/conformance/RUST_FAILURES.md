# Rust backend: the conformance and spec cases that still fail

Measured with `scripts/wj/target_tiers.wj run <wj> --backend rust` (programs that need `windjammer_runtime` or the relaxed
overflow lints build through cargo, the rest through rustc). The ledger (`tests/conformance/backend_ledger.tsv`) carries the same
text per program; this table groups it by who has to act. No case below is a harness problem: each is a first-error line of the
generated Rust (or of the program) and a bug or a gap in one component.

| Case | First error | Owner | What is wrong |
|---|---|---|---|
| conformance `stdlib/async_runtime` | `E0433: cannot find module or crate async_runtime` | the program | `use std::async_runtime` is the runtime crate's internal path; the std module is `std::async` (kept by ASYNC_FN_REMOVAL.md: `sleep_ms_blocking` stays). Interpreted with `use std::async` it prints its expected block; rename the import |
| SPEC-ATTR-001 | `cannot find attribute custom_tag` | codegen (decorators) | an unknown user decorator is emitted as a Rust attribute |
| SPEC-ATTR-004 | `cannot find attribute route` | codegen (decorators) | same, and the positional argument is dropped (`#[route( = "/users", ...)]`) |
| SPEC-CLOS-009 | `E0308 expected i64, found i32` at `x + base` | width | a closure captures a literal-typed `int` printed `i32` (drift family A) |
| SPEC-COLL-005 | `lifetime may not live long enough` at `wj_chunk_by(\|x\| x)` | std surface | the runtime helper takes a borrowed-key closure |
| SPEC-COLL-009 | same at `wj_sort_by_key_desc(\|x\| x)` | std surface | same helper family |
| SPEC-COLL-011 | `E0277 Vec<i64> cannot be built from usize` | width | `slice.len()` collected into `Vec<int>` without the conversion |
| SPEC-STD-002 | `E0277 Vec<String> cannot be built from &String` | std surface | `HashMap.keys()` yields borrowed keys, the surface says owned |
| SPEC-TRT-004 | `E0369 cannot add V to V` | traits/codegen | an operator on a generic type parameter is emitted without its `Add<Output = V>` bound |
| SPEC-TRT-011 | `E0308 expected bool, found &bool` | ownership/traits | a user trait implemented for `bool`/`str` takes `self` by reference |

## Fixed by reading, not by a compiler change

`conformance/types/enums_and_matching`, `types/traits_and_generics` and `error_handling/result_and_option` failed on every backend
with the same first mismatch, because their expected blocks were wrong, not the compiler: `Shape::Circle(5)` is computed as
`3 * r * r` = 75 (the expected block said 78, which is the value of `precise_area`, `314 * 25 / 100`, a different method), and
`parse_int("xyz")` returns `"invalid number"` (the expected block said `oops`). With the blocks corrected all three pass on the
interpreter, rust, javascript and typescript, and `traits_and_generics` and `result_and_option` pass on go too.
