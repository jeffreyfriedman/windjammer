# E0507 Ownership Inference Fixes (2026-03-15)

## Summary

Extended ownership inference to fix remaining "cannot move out of" (E0507) and "use of moved value" (E0382) errors. Philosophy: **"Compiler Does the Hard Work"** - infer borrow/clone automatically.

## Complete Categorization of E0507 Errors (from build_errors.log)

| Pattern | Count | Example | Fix |
|---------|-------|---------|-----|
| Option if let/match with &self/&mut self | 4 | `if let Some(search) = self.search` | `&self.search` or `&mut self.search` |
| Option match with borrowed param | 2 | `match node.children { Some(c) => c }` | `&node.children` |
| Option match with iterator var | 2 | `match entity.health` when entity from `for entity in &self.entities` | `&entity.health` |
| Vec index with move | 2 | `let preset = self.available_presets[i]` | `&self.available_presets[i]` |
| Vec index + method(owned self) | 1 | `self.tracks[i].sample(time)` | `self.tracks[i].clone().sample(time)` |
| For-loop param used multiple times | 1 | `for comp in entity_components` (nested) | `&entity_components` |
| Struct literal from borrowed field | 3 | `bindings: self.bindings` when &self | `self.bindings.clone()` |
| Struct literal nested field | 1 | `self.graph.passes` when &self | `self.graph.passes.clone()` |
| Self.method() with owned self | 3 | `self.input_uniform(buffer)` when &self | Requires analyzer fix (infer &mut self) |

## Implemented Fixes (Session 2 - 2026-03-15)

### 5. Option Match/If-Let with Iterator Var (E0507) - NEW

**File:** `windjammer/src/codegen/rust/statement_generation.rs`

- **Problem:** `match entity.health { Some(h) => ... }` when `entity` is from `for entity in &self.entities` - entity is `&Entity`, so entity.health is behind a shared reference.
- **Fix:** Extended `match_scrutinee_is_borrowed_field()` to also check `borrowed_iterator_vars`. When the base identifier (e.g. `entity`) is in `borrowed_iterator_vars`, generate `&expr.field`.

### 6. For-Loop Param in Multiple Nested Loops (E0382) - ENHANCED

**File:** `windjammer/src/codegen/rust/variable_analysis.rs`

- **Problem:** `for comp in entity_components` used in nested loops - param consumed on first iteration.
- **Fix:** Replaced linear scan with `count_for_loop_iterables_recursive()` that recursively counts ALL for-loop iterables (including nested). When any identifier appears as iterable count > 1, add to `for_loop_borrow_needed`.
- **Removed:** The `if is_param { continue }` - params now get borrow when used in multiple loops.

### 7. Struct Literal from Iterator Var Field (E0507) - NEW

**File:** `windjammer/src/codegen/rust/expression_generation.rs`

- **Problem:** `Foo { field: entity.field }` when entity is from for-loop (borrowed).
- **Fix:** Extended struct literal clone logic to check `borrowed_iterator_vars` in addition to `inferred_borrowed_params`.

## Previously Implemented Fixes

### 1. For-Loop Auto-Borrow (E0382)

**File:** `windjammer/src/codegen/rust/variable_analysis.rs`

- **Fix:** `should_borrow_for_iteration()` + `for_loop_borrow_needed` set.

### 2. Option Match/If-Let with Borrowed Base (E0507)

**File:** `windjammer/src/codegen/rust/statement_generation.rs`

- **Fix:** `match_scrutinee_is_borrowed_field()` for params and self.

### 3. Struct Literal Field from Borrowed Base (E0507)

**File:** `windjammer/src/codegen/rust/expression_generation.rs`

- **Fix:** `extract_root_identifier()` + clone for inferred borrowed.

### 4. Vec Index + Method(owned self) (E0507)

**File:** `windjammer/src/codegen/rust/expression_generation.rs`

- **Fix:** `method_receiver_ownership` check, generate `.clone()` when needed.

## Test Cases (15+ patterns)

**File:** `windjammer/tests/e0507_ownership_inference_test.rs`

1. `test_vec_string_index_generates_borrow` - Vec<String> index
2. `test_vec_index_method_owned_self_generates_clone` - vec[i].sample()
3. `test_option_if_let_borrows_self_field` - if let Some with &self
4. `test_option_map_uses_as_ref` - Option::map
5. `test_vec_non_copy_index_let_binding` - let p = self.presets[i]
6. `test_option_match_borrows_self_field` - match self.health
7. `test_vec_index_field_access_no_clone` - vec[i].x
8. `test_for_loop_param_used_multiple_times_borrows` - entity_components
9. `test_option_match_param_field_borrows` - match node.children
10. `test_struct_literal_from_vec_index_clones` - Wrapper { item: items[i] }
11. `test_option_match_iterator_var_field_borrows` - match entity.health in for loop
12. `test_option_if_let_mut_self_borrows` - if let Some with &mut self
13. `test_struct_literal_self_field_clones` - self.graph.passes
14. `test_param_used_in_multiple_nested_loops_borrows` - entity_components nested
15. `test_vec_index_let_binding_borrows` - let p = self.presets[i]
16. `test_option_if_let_param_field_borrows` - if let Some(vel) = entity.velocity

## Decision Framework (Borrow vs Clone)

| Context | Borrow | Clone |
|---------|--------|-------|
| Vec index, read-only | `&vec[i]` | - |
| Vec index, method(owned self) | - | `vec[i].clone().method()` |
| Vec index, struct literal field | - | `vec[i].clone()` |
| Option match/if-let, borrowed base | `&expr.field` | - |
| Struct literal, borrowed base | - | `expr.field.clone()` |
| For-loop iterable, used 2+ times | `&iterable` | - |

## Patterns Not Yet Fixed

- **self.method() with owned self when self is borrowed:** Builder pattern `self.input_uniform(buffer)` when &self. Requires analyzer to infer &mut self, or generate self.clone(). Deferred.

## Verification

To run E0507 tests (requires `wj` binary with cli feature):
```bash
cd windjammer
cargo test --test e0507_ownership_inference_test --features cli
```

Note: The windjammer binary (src/main.rs) has a pre-existing build error when cli is enabled. The wj binary should build; if the windjammer binary blocks the build, the tests cannot run until that is fixed.

## Files Changed

- `windjammer/src/codegen/rust/variable_analysis.rs` - precompute_for_loop_borrows, count_for_loop_iterables
- `windjammer/src/codegen/rust/statement_generation.rs` - match_scrutinee_is_borrowed_field, match_scrutinee_base_identifier
- `windjammer/src/codegen/rust/expression_generation.rs` - struct literal clone for inferred borrowed
- `windjammer/src/type_inference/float_inference.rs` - parent_opt fix
- `windjammer/tests/e0507_ownership_inference_test.rs` - 2 new tests
