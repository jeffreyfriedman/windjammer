# Compiler Bug: Over-Aggressive Auto-Cloning

**Date**: 2026-02-20
**Severity**: HIGH - Causes compilation errors
**Status**: IN PROGRESS

## Summary

The Windjammer compiler incorrectly adds `.clone()` when passing struct fields from borrowed references to functions, even when the destination function expects a borrowed reference (`&String`), not an owned value (`String`).

## Example

### Source (Windjammer)
```wj
pub struct Ingredient {
    pub item_id: string,
    pub quantity: i32,
}

impl Recipe {
    pub fn check(inventory: &Inventory) -> bool {
        for ingredient in &self.ingredients {
            if !inventory.has_item(ingredient.item_id, ingredient.quantity) {
                return false
            }
        }
        true
    }
}

// has_item signature
impl Inventory {
    pub fn has_item(item_id: string, quantity: i32) -> bool {
        // ...
    }
}
```

### Generated (Rust) - INCORRECT
```rust
if !inventory.has_item(ingredient.item_id.clone(), ingredient.quantity) {
//                                       ^^^^^^^^ WRONG! Should be & or nothing
    return false;
}

// has_item signature - wants BORROWED
pub fn has_item(&self, item_id: &String, quantity: i32) -> bool {
//                                ^^^^^^^ expects borrowed!
```

### Expected (Rust) - CORRECT
```rust
if !inventory.has_item(&ingredient.item_id, ingredient.quantity) {
//                     ^ Just borrow, don't clone!
    return false;
}
```

## Root Cause

**Location**: `windjammer/src/codegen/rust/generator.rs:7535-7589, 7593-7626`

The compiler has logic to add `.clone()` when passing fields from borrowed parameters:

```rust
// Lines 7556-7589 (commented out as attempted fix)
if let Expression::FieldAccess { object: field_obj, .. } = arg {
    if let Expression::Identifier { name, .. } = &**field_obj {
        if (is_explicitly_borrowed || is_inferred_borrowed)
            && !arg_str.ends_with(".clone()")
        {
            // BUG: Adds .clone() without checking if DESTINATION wants owned or borrowed!
            arg_str = format!("{}.clone()", arg_str);
        }
    }
}
```

**The Bug**: This code checks if the SOURCE is borrowed (`ingredient` from iterator), but does NOT check if the DESTINATION parameter wants `&String` (borrowed) vs `String` (owned).

**Correct Logic Should Be**:
1. Check if source is borrowed ✓ (currently does this)
2. Check if destination wants owned vs borrowed ✗ (MISSING!)
3. If destination wants `&String` → add `&`, don't clone
4. If destination wants `String` → add `.clone()`

## Impact

**Compilation Errors**: 14 E0308 errors in windjammer-game-core:
- `rpg/crafting.rs`: 3 errors
- `rpg/trading.rs`: 6 errors
- Other files: 5 errors

All follow the same pattern:
```
error[E0308]: mismatched types
   |
72 |             if !inventory.has_item(ingredient.item_id.clone(), ingredient.quantity) {
   |                           -------- ^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `&String`, found `String`
   |                           |
   |                           arguments to this method are incorrect
```

## TDD Tests

**File**: `windjammer/tests/bug_struct_field_auto_clone_test.rs`
**Status**: 2/2 FAILING (as expected for TDD)

Tests verify:
1. `item.id` in comparison should not have `.clone()`
2. `ingredient.item_id` passed to borrowed param should not have `.clone()`

## Attempted Fix #1 (INCOMPLETE)

**Approach**: Commented out lines 7556-7589 and 7593-7626 that were adding `.clone()`

**Result**: Tests still fail - the `.clone()` is being added elsewhere

**Next Steps**:
1. Trace where `borrowed_iterator_vars` is being used
2. Check if `ingredient.item_id` is being tracked as needing clone
3. Find ALL places where `.clone()` might be added for FieldAccess
4. Ensure signature lookup is working correctly

## Signature Lookup

The compiler DOES have signature information:
- `has_item` signature stored with `param_ownership: [Borrowed]`
- Line 7416: `if let Some(&ownership) = sig.param_ownership.get(i)`
- This correctly identifies that parameter wants `Borrowed` mode

**The Missing Link**: The FieldAccess auto-clone logic runs INSIDE the `OwnershipMode::Owned` block, but we should never be in that block if signature says `Borrowed`!

**Hypothesis**: The signature lookup is failing for some reason, so it falls through to the `else` block (no signature found), which then blindly adds `.clone()`.

## Next Actions

1. ✅ Create TDD tests
2. ✅ Document the bug
3. ⏳ Add debug logging to trace signature lookup
4. ⏳ Find why signature lookup fails
5. ⏳ Fix signature registry or lookup
6. ⏳ Verify tests pass
7. ⏳ Dogfood: compile windjammer-game-core
8. ⏳ Commit with TDD success

## Related Files

- `windjammer/src/codegen/rust/generator.rs` - Code generation
- `windjammer/src/analyzer.rs` - Ownership inference
- `windjammer/tests/bug_struct_field_auto_clone_test.rs` - TDD tests
- `windjammer-game/windjammer-game-core/src_wj/rpg/crafting.wj` - Affected game code
- `windjammer-game/windjammer-game-core/src_wj/rpg/trading.wj` - Affected game code

## Design Decision Needed

Should the compiler:
**Option A**: Never auto-clone struct fields, always require explicit `.clone()` in source
**Option B**: Auto-clone only when destination truly wants owned (fix signature lookup)
**Option C**: Auto-borrow (`&`) when destination wants borrowed, auto-clone when owned

**Recommendation**: Option C - Smart inference (aligns with Windjammer philosophy)

## References

- TDD Success: `WINDJAMMER_TDD_SUCCESS.md`
- Language Philosophy: `.cursor/rules/windjammer-development.mdc`
- Idiomatic Code Audit: `WINDJAMMER_IDIOMATIC_CODE_AUDIT.md`
