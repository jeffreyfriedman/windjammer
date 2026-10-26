# Findings from dogfooding a large application

Each item below has a regression test in `tests/dogfood_findings_test.rs` (compiled output, interpreter
output, and a hand-derived expectation must agree). Open items are listed in
`.config/known-test-failures.txt`; a test that starts passing must be removed from that list.

| Test | Status |
|------|--------|
| `untyped_int_accumulator_over_fields_is_64_bit` | open, P0, numeric solver (step C4) |
| `const_string_in_if_else_join_with_parameter` | open, ownership/type solver |
| `const_string_let_mut_later_assigned_owned_string` | open, ownership/type solver |
| `const_string_passed_where_owned_string_is_required` | open, ownership/type solver |
| `crate_qualified_call_through_nested_modules_borrows_str_formals` | open, call-site signature resolution |
| `consecutive_field_doc_comments_parse` | fixed (parser collects all consecutive `///` lines; codegen emits each line) |
| `interpreter_bodiless_mod_loads_sibling_file`, `interpreter_bodiless_mod_with_crate_paths` | fixed |
| `interpreter_module_and_function_with_same_name` | fixed |
| `interpreter_test_assertions_and_typed_json_parse` | fixed |
| `closure_char_param_passed_to_str_find` | fixed |
| `value_moved_into_vec_macro_then_used_again_is_cloned`, `concat_temp_passed_to_read_only_string_formal`, `loop_var_field_move_then_second_loop_over_same_collection`, `range_loop_index_over_len_used_as_counter` | pass on this tip with the minimal shapes; the original failures need larger programs |
| `module_alias_call_*`, `super_qualified_call_*`, `self_qualified_call_*` (`tests/compiler_findings_test.rs`) | fixed: calls through `use crate::m as alias`, `self::` and `super::` resolve to the same-crate definition (`resolve::same_crate_callee_key`) |
| `map_get_unwrap_or_ref_literal_yields_value` | fixed: `map.get(k).unwrap_or(&0)` keeps the `&T` result instead of copying the receiver |
| `interpreted_test_*`, `interpreted_doctests_in_library_module_run`, `compiled_test_on_crate_with_crate_use_passes` | fixed: a test file runs against its whole project (interpreted and compiled), including tests in sub-directories |
| `untyped_float_local_min_max_chain_stays_f32`, `push_typed_literal_width_does_not_leak` | open, numeric solver (step C4) |
| `trait_receiver_widens_to_mutating_impl_across_modules` | open, ownership (convergence): single-file form is correct, the three-module form emits `&self` for a mutating impl |
| `mutated_by_value_param_moved_into_field_stays_owned` | open, ownership (convergence), single file |
| `by_value_scalar_forwarded_through_indexed_receiver_across_modules`, `owned_vec_and_struct_args_across_modules_reused_after_call` | guards; the original failures were not reproduced in minimal projects |


## Open: untyped integer accumulator becomes `i32`

`let mut total = 0` that only accumulates `int` struct fields is emitted as `i32` with truncating
`as i32` casts, so sums above 2^31 are wrong with no diagnostic. The interpreter is correct. This is a
numeric inference question (the literal has no width anchor; the field type must flow back into the
binding), owned by the numeric solver. Acceptance test: `untyped_int_accumulator_over_fields_is_64_bit`.

## Open: `string` constants used as owned values

A `const NAME: string` is emitted as `&'static str`. Three shapes need an owned `String` and get a
mismatch: both arms of an `if`/`else` where the other arm is a `String` parameter, a `let mut` initialised
from the constant and later assigned a `String`, and the constant passed to an owned formal or placed in a
`Vec<string>`. The class fix is a single decision in the ownership solver: a constant read in an owned
position is `.to_string()`. The single-file shapes fail identically on the earlier frozen compiler build.

## Open: `crate::a::b::f(x.field)` through nested modules

The call site resolves the callee under the module-qualified key `domain::text::join`, which still holds the
declaration stub (every parameter owned), while the converged signature (`&str` formals) is registered only
under the bare name `join`. One-level paths (`crate::text::join`) hit a converged key and work. A local
retry with shorter module suffixes does not help (the stale key exists, so nothing is retried), and
preferring the bare-name signature inside `ir_call_site.rs` and `get_signature_with_global` did not change
the output, because the signature handed to `regular_call_arguments.rs` is resolved earlier by another
path. The real fix is to register one converged entry per function under its full module path after
multipass convergence, so no lookup can see the stub.

## Handed to the semantic-layer stream

These need ownership or width decisions in the IR solver rather than local patches:

- a value moved into a call or `vec![]` and used again should be cloned automatically (passes on the
  minimal shape today; the failing shapes involve partially moved structs and struct literals);
- a concatenation temporary passed to a read-only `string` formal needs a borrow;
- a loop variable's field moved out in one loop, with a second loop over the same collection;
- `for i in 0..v.len()` with an `int` loop variable: `usize` versus `int` index typing;
- incremental cache: reverting files after an edit once reported hundreds of phantom type errors until
  the build directory and cache were removed. Not minimized, so there is no test yet.

## Round 2 (large backend)

| Test (`tests/expression_operand_positions_property_test.rs`, `round2_findings_test.rs`, `range_precedence_test.rs`, `string_literal_roundtrip_test.rs`) | Status |
|---|---|
| operand `if`/`match` in compound assignment, conditions, binary operands (generated gate) | fixed (value position generates the block tail even inside void blocks) |
| `outer + 1..n` precedence | fixed (range is the lowest binary operator; parser, formatter and codegen agree) |
| `$$` in string literals | documented as SPEC-LEX-009 (literal `$`); round-trip gate on fmt, interpreter, Rust, JS, Go |
| owned trait-impl `string` formal into a runtime `&str` formal | fixed (emit-truth sync of `str_ref_optimized_params`) |
| same-name private structs, `json.parse_string` target from later use, `ServerRequest` literal (interpreter) | fixed |
| untyped counter vs `len()`, push literal width from return type, loop variable vs literal inside `if` | open, numeric solver (ADR-0015 / C4) |
| call-site `&T`/`T` mismatches across modules | not reproduced in minimal projects (guards added) |
