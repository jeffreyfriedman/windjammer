# E0507 Complete Categorization & Fixes (2026-03-15)

## Summary

Comprehensive ownership inference extensions to eliminate remaining "cannot move out of" (E0507) errors. Philosophy: **"Compiler Does the Hard Work"**.

## Complete E0507 Error Categorization (from build_errors.log)

| # | Pattern | File:Line | Example | Fix Applied |
|---|---------|-----------|---------|-------------|
| 1 | Option if-let with &mut self | npc_behavior.rs:255 | `if let Some(search) = self.search` | `&mut self.search` (match_scrutinee_is_borrowed_field + inferred_mut_borrowed_params) |
| 2 | Vec index + method(owned self) | clip.rs:125 | `self.tracks[i].sample(time)` | `tracks[i].clone().sample(time)` (method_receiver_ownership) |
| 3 | Option match iterator var | query_system.rs:200 | `match entity.health` in for loop | `&entity.health` (borrowed_iterator_vars) |
| 4 | Option if-let iterator var | query_system.rs:252 | `if let Some(vel) = entity.velocity` | `&entity.velocity` (borrowed_iterator_vars) |
| 5 | self.method() owned (builder) | hierarchy_panel.rs:124 | `self.build_node(...)` | `self.clone().build_node(...)` |
| 6 | Vec index let binding | particle_editor.rs:136 | `let preset = self.available_presets[i]` | `&self.available_presets[i]` (Index handler) |
| 7-9 | Builder pattern | shader_graph.rs:340,344,348 | `self.input_uniform(buffer)` | `self.clone().input_uniform(...)` (object_is_borrowed) |
| 10-11 | Struct literal self fields | shader_graph.rs:402 | `bindings: self.bindings` | `self.bindings.clone()` (struct literal) |
| 12 | Let binding nested field | shader_graph.rs:403 | `let mut new_passes = self.graph.passes` | `self.graph.passes.clone()` (in_owned_value_context) |
| 13-14 | Option match param | octree.rs:133,152 | `match node.children { Some(c) => c }` | `&node.children` (match_scrutinee_is_borrowed_field) |

## Implemented Fixes

### 1. in_owned_value_context (NEW)
**Files:** generator.rs, statement_generation.rs

- **Problem:** `let mut new_passes = self.graph.passes` when self is &self - cannot move
- **Fix:** Set `in_owned_value_context = true` when generating let binding values. FieldAccess handler adds `.clone()` when root is borrowed and in owned context.

### 2. FieldAccess Clone for Borrowed Root (NEW)
**File:** expression_generation.rs

- **Problem:** `self.graph.passes` in let binding when self is borrowed
- **Fix:** Use `extract_root_identifier` for any FieldAccess. When root in inferred_borrowed_params/inferred_mut_borrowed_params AND (in_struct_literal_field OR in_owned_value_context) AND not Copy → add `.clone()`

### 3. Builder Pattern - Extended object_is_borrowed (ENHANCED)
**File:** expression_generation.rs

- **Problem:** `builder.input_uniform(buffer)` when builder is &T (from match arm or param)
- **Fix:** Check not just `self` but any Identifier in inferred_borrowed_params. Also use extract_root_identifier for FieldAccess receivers.

### 4. Existing Fixes (Verified)
- Option if-let/match with &self/&mut self → match_scrutinee_is_borrowed_field
- Option match with iterator var → borrowed_iterator_vars in for-loop
- Vec index + method(owned self) → method_receiver_ownership
- Struct literal from borrowed field → extract_root_identifier + clone
- Vec index let binding → Index handler auto-borrow

## Decision Framework

| Context | Borrow | Clone |
|---------|--------|-------|
| Vec index, read-only | `&vec[i]` | - |
| Vec index, method(owned self) | - | `vec[i].clone().method()` |
| Vec index, struct literal field | - | `vec[i].clone()` |
| Option match/if-let, borrowed base | `&expr.field` or `&mut` | - |
| Struct literal, borrowed base | - | `expr.field.clone()` |
| Let binding, borrowed base | - | `expr.field.clone()` |
| self.method() owned, self borrowed | - | `self.clone().method()` |
| For-loop iterable, used 2+ times | `&iterable` | - |

## Test Suite (20 cases)

1. test_vec_string_index_generates_borrow
2. test_vec_index_method_owned_self_generates_clone
3. test_option_if_let_borrows_self_field
4. test_option_map_uses_as_ref
5. test_vec_non_copy_index_let_binding
6. test_option_match_borrows_self_field
7. test_vec_index_field_access_no_clone
8. test_for_loop_param_used_multiple_times_borrows
9. test_option_match_param_field_borrows
10. test_struct_literal_from_vec_index_clones
11. test_option_match_iterator_var_field_borrows
12. test_option_if_let_mut_self_borrows
13. test_struct_literal_self_field_clones
14. test_param_used_in_multiple_nested_loops_borrows
15. test_vec_index_let_binding_borrows
16. test_option_if_let_param_field_borrows
17. test_builder_pattern_self_clone_when_owned_method
18. test_option_match_param_returns_owned_clones
19. test_struct_literal_nested_field_clones
20. test_let_binding_borrowed_field_clones (NEW)

## Verification

```bash
cd windjammer
cargo test --test e0507_ownership_inference_test --features cli
```

## Files Changed

- windjammer/src/codegen/rust/generator.rs - in_owned_value_context
- windjammer/src/codegen/rust/statement_generation.rs - set in_owned_value_context in Let handler
- windjammer/src/codegen/rust/expression_generation.rs - FieldAccess clone for borrowed root, extended object_is_borrowed
- windjammer/tests/e0507_ownership_inference_test.rs - test_let_binding_borrowed_field_clones
