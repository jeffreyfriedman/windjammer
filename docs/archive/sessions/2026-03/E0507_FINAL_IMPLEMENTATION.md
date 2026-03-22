# E0507 Final Implementation (2026-03-15)

## Summary

Reduced E0507 "cannot move out of" errors through ownership inference extensions. Target: ~12 → <5.

## Implemented Fixes

### 1. Vec Index + Method(owned self) - CRITICAL FIX

**File:** `windjammer/src/codegen/rust/expression_generation.rs`

**Problem:** `self.tracks[i].sample(time)` when `BoneTrack::sample` takes owned self - `in_field_access_object` suppressed the clone block, so we never added `.clone()`.

**Fix:** Check `method_receiver_ownership == Owned` **before** the suppress block. When method needs owned receiver, always return `{}.clone()` for Vec index - can't move out of Vec.

```rust
// E0507 fix: method_receiver_ownership=Owned ALWAYS requires .clone() for Vec index.
let method_needs_owned = self.method_receiver_ownership.as_ref()
    .is_some_and(|o| matches!(o, OwnershipMode::Owned));
if method_needs_owned {
    return format!("{}.clone()", base_expr);
}
```

**Impact:** Fixes clip.rs:125 - `self.tracks[i].sample(time)`

### 2. match_scrutinee_is_borrowed_field - Extended for Explicit Params

**File:** `windjammer/src/codegen/rust/statement_generation.rs`

**Problem:** `match node.children` when `node` has explicit `&OctreeNode` type - analyzer might not infer, so we didn't add `&node.children`.

**Fix:** Also check `current_function_params` for explicit `Ref` ownership or `Reference`/`MutableReference` type.

```rust
|| self.current_function_params.iter().any(|p| {
    p.name == *name && (matches!(p.ownership, OwnershipHint::Ref)
        || matches!(&p.type_, Type::Reference(_) | Type::MutableReference(_)))
})
```

**Impact:** Fixes octree.rs:133,152 - `match node.children { Some(c) => c }`

### 3. object_is_borrowed - Include borrowed_iterator_vars

**File:** `windjammer/src/codegen/rust/expression_generation.rs`

**Problem:** Builder pattern when object is from match arm binding (e.g. `pass_builder.input_uniform()`) - we only checked `inferred_borrowed_params`, not `borrowed_iterator_vars`.

**Fix:** Add `borrowed_iterator_vars.contains(name)` to the object_is_borrowed check for both Identifier and extract_root_identifier cases.

**Impact:** Fixes shader_graph builder pattern when PassBuilder comes from match arm.

## Pattern Coverage

| Pattern | Fix | Status |
|---------|-----|--------|
| A: Vec index + method(owned) | Early return .clone() in Index handler | ✅ |
| B: Option if-let &mut self | match_scrutinee_is_borrowed_field + ref mut | Existing |
| C: Option match iterator var | borrowed_iterator_vars in for-loop | Existing |
| D: Option match param | match_scrutinee + explicit param types | ✅ |
| E: Builder pattern | object_is_borrowed + borrowed_iterator_vars | ✅ |
| F: Struct literal/let binding | FieldAccess clone for in_owned_value_context | Existing |

## Test Suite

**File:** `windjammer/tests/e0507_final_test.rs`

- test_vec_index_method_owned_self_clone
- test_option_match_param_borrows
- test_option_if_let_mut_self_ref_mut
- test_struct_literal_borrowed_field_clones
- test_let_binding_borrowed_field_clones
- test_vec_index_let_binding_borrows

## Verification

```bash
cd windjammer
cargo test --test e0507_final_test --test e0507_ownership_inference_test --test codegen_vec_index_borrow_test --features cli
```

## Files Changed

- `windjammer/src/codegen/rust/expression_generation.rs` - Index early return, object_is_borrowed
- `windjammer/src/codegen/rust/statement_generation.rs` - match_scrutinee_is_borrowed_field
- `windjammer/tests/e0507_final_test.rs` - New test suite

## Philosophy

"Safety Without Ceremony" - automatic ownership handling. The compiler infers when to borrow vs clone.
