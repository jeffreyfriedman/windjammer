# Compiler repro queue (dogfooding — do not work around in application code)

Cross-crate / multipass dogfooding surfaced these codegen gaps. Each has a **failing** repro; fix the compiler/analyzer/codegen, then delete application workarounds.

| Priority | Bug | Repro test(s) | Workaround pattern (forbidden long-term) |
|----------|-----|---------------|------------------------------------------|
| P0 | **`HashMap::contains_key/insert` — call-return / loop-local i64 in 62-file multipass** | isolated: **`test_library_multipass_graph_bfs_hashmap_compiles`** · full crate: swap `graph_vertex_map.wj` | `graph_vertex_map.wj` Vec backend (single swap point) |
| P0 | Loop reused binding — owned binding in loop must borrow for `&T` callee | `regression_loop_reused_graph_borrow_test.wj`, **`test_library_multipass_loop_reused_graph_borrow`** | `.clone()` in loop bodies |
| P0 | **`for v in vertices { f(vertices, v) }` — must borrow `vertices`** | **`test_library_multipass_for_in_vertices_reuse_borrow`** | Index `while` loops instead of `for-in` (LCC) |
| P1 | **`HashMap<i64, f64>::insert(k, 0.0)` — literal must infer f64 not f32** | **`test_library_multipass_hashmap_i64_f64_zero_literal_insert`** | Vec maps / explicit f64 helpers |
| P1 | String literal → `string` param must emit `&"lit".to_string()` not owned String | `regression_andstring_literal_call_test.wj` | Manual string compares / delayed transpile |
| P1 | Cross-crate `Type::new("lit")` with owned `String` formal — no WJ sig → bare `&str` | `codegen_cross_crate_associated_new_bare_literal_must_auto_own_gate_test` | `owned_string("lit")` / WJ-source `.to_string()` |
| P1 | `strings::split(line, "\|")` — pipe delimiter must stay `&str`, not `.to_string()` | ✅ isolated gate · ❌ **full wdb-layers `for line in lines` → `string` param** | Byte-at-a-time CSV parse; while-index loop in loader |
| P1 | `strings::starts_with(s, "#")` — literal prefix same as split | ✅ `regression_strings_starts_with_literal_test.wj` | ~~Magic byte compares~~ **REMOVED** |
| P2 | Cross-module `Vec` helper calls omit `&` borrows | `bug_cross_module_vec_borrow_test.rs` | Inline helpers per file (duplicate logic) |

## WindjammerDB migration path (ArcadeDB perf)

1. **Now:** All graph engines use `graph_vertex_map.wj` (hexagonal port). Vec backend is centralized — one swap point.
2. **When P0/P1 HashMap repros pass:** Replace `graph_vertex_map.wj` internals with `HashMap<i64, T>` only.
3. **Next perf (no compiler block):** SSSP binary heap, real `run_nanos` wall-clock in engines, parallel PageRank iterations.

## Run repros

```bash
unset CARGO_TARGET_DIR && cargo test --release --test all -- \
  regression_hashmap_i64 regression_loop_reused regression_andstring \
  regression_strings_split regression_strings_starts_with \
  bug_loop_reused_binding_borrow bug_cross_module_vec_borrow \
  test_library_multipass_hashmap_i64 test_library_multipass_strings_split \
  test_library_multipass_loop_reused \
  test_library_multipass_graph_bfs_hashmap \
  test_library_multipass_csv_for_in_line \
  cross_crate_associated_new_bare_literal \
  -- --test-threads=1
```

When a row's gate is green, remove the corresponding application workaround in the same session.
