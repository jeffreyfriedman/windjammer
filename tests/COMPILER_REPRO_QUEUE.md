# Compiler repro queue (dogfooding — do not work around in application code)

Cross-crate / multipass dogfooding surfaced these codegen gaps. Each has a **failing** repro; fix the compiler/analyzer/codegen, then delete application workarounds.

| Priority | Bug | Repro test(s) | Workaround pattern (forbidden long-term) |
|----------|-----|---------------|------------------------------------------|
| P0 | `HashMap<i64, T>::get/contains_key` — i64 keys need auto-borrow | `regression_hashmap_i64_*.wj`, **`library_multipass_map_key_codegen_test::test_library_multipass_hashmap_i64_key_auto_borrow`** | Parallel `Vec` key/val maps |
| P0 | Loop reused binding — owned binding in loop must borrow for `&T` callee | `regression_loop_reused_graph_borrow_test.wj`, `bug_loop_reused_binding_borrow_test.rs`, **`test_library_multipass_loop_reused_graph_borrow`** | `.clone()` in loop bodies |
| P1 | String literal → `string` param must emit `&"lit".to_string()` not owned String | `regression_andstring_literal_call_test.wj` | Manual string compares / delayed transpile |
| P1 | Cross-crate `Type::new("lit")` with owned `String` formal — no WJ sig → bare `&str` | `codegen_cross_crate_associated_new_bare_literal_must_auto_own_gate_test` | `owned_string("lit")` / WJ-source `.to_string()` |
| P1 | `strings::split(line, "\|")` — pipe delimiter must stay `&str`, not `.to_string()` | `regression_strings_split_pipe_delimiter_test.wj`, `bug_loop_reused_binding_borrow_test.rs`, **`test_library_multipass_strings_split_pipe_delimiter`** | Byte-at-a-time CSV parse |
| P1 | `strings::starts_with(s, "#")` — literal prefix same as split | `regression_strings_starts_with_literal_test.wj` | Magic byte compares |
| P2 | Cross-module `Vec` helper calls omit `&` borrows | `bug_cross_module_vec_borrow_test.rs` | Inline helpers per file (duplicate logic) |

## Run repros

```bash
unset CARGO_TARGET_DIR && cargo test --release --test all -- \
  regression_hashmap_i64 regression_loop_reused regression_andstring \
  regression_strings_split regression_strings_starts_with \
  bug_loop_reused_binding_borrow bug_cross_module_vec_borrow \
  test_library_multipass_hashmap_i64 test_library_multipass_strings_split \
  test_library_multipass_loop_reused \
  cross_crate_associated_new_bare_literal \
  -- --test-threads=1
```

When a row's gate is green, remove the corresponding application workaround in the same session.
