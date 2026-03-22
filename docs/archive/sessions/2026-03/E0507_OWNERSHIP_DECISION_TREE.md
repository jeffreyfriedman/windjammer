# E0507 Ownership Inference - Complete Decision Tree

**Philosophy:** "Compiler Does the Hard Work" - ownership should be automatic.

## Decision Algorithm

```
1. Is value behind &self or &param? → clone() if non-Copy when owned needed
2. Is value in Vec/slice? → &vec[i] unless consumed (method(owned) or struct literal)
3. Is value in Option? → .as_ref() or pattern &opt, clone in arm when returning owned
4. Is value used multiple times? → borrow all uses (for-loop iterable)
5. Is value moved to owned context? → clone()
```

## Pattern → Fix Mapping

| Pattern | Context | Fix | Location |
|---------|---------|-----|----------|
| **Vec indexing** | Read-only (field access, let binding) | `&vec[i]` | expression_generation.rs Index handler |
| **Vec indexing** | Method takes owned self | `vec[i].clone().method()` | expression_generation.rs Index + method_receiver_ownership |
| **Vec indexing** | Struct literal field | `vec[i].clone()` | expression_generation.rs Index force_clone_for_owned_context |
| **Option if-let** | &self / &param / iterator var | `&expr.field` or `&mut expr.field` | statement_generation.rs if-let value_str |
| **Option match** | &self / &param / iterator var | `&expr.field` | statement_generation.rs match value_str |
| **Option match arm** | Return owned from borrowed Option | Add bound var to borrowed_iterator_vars → `Some(c.clone())` | statement_generation.rs added_borrowed |
| **Option::map** | &self | `.as_ref().map(...)` | expression_generation.rs MethodCall |
| **Struct literal** | Field from borrowed base | `expr.field.clone()` | expression_generation.rs StructLiteral |
| **self.method()** | &self, method takes owned | `self.clone().method()` | expression_generation.rs MethodCall |
| **For-loop iterable** | Used in 2+ loops | `&iterable` | variable_analysis.rs for_loop_borrow_needed |

## Implementation Details

### match_scrutinee_is_borrowed_field()
Checks if scrutinee base is in:
- inferred_borrowed_params
- inferred_mut_borrowed_params  
- borrowed_iterator_vars

### scrutinee_borrowed for match arms
When we generate `match &expr` (Option behind borrow), bound vars are `&T`.
Add to borrowed_iterator_vars so `Some(c)` becomes `Some(c.clone())` when return expects owned.

### extract_root_identifier()
Recursively extracts base: `self.graph.passes` → `self`

## Test Coverage (20 cases)

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

## Files Changed

- `windjammer/src/codegen/rust/statement_generation.rs` - scrutinee_borrowed for match/if-let
- `windjammer/src/codegen/rust/expression_generation.rs` - self.clone() for builder pattern
- `windjammer/src/analyzer/self_analysis.rs` - &body for for_loop_mutates_self_field_elements
- `windjammer/tests/e0507_ownership_inference_test.rs` - BoneTrack non-Copy, new tests
