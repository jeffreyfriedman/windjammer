# TDD Success: Auto-Clone Bug Fixed! 🎉

## Date: 2026-02-20

## Problem

Compiler incorrectly added `.clone()` to borrowed iterator field access in function call arguments, even when the parameter expected `Borrowed` ownership.

### Example (Windjammer)
```windjammer
fn check_inventory(&self, has_item: fn(string, i32) -> bool) -> bool {
    for ingredient in &self.ingredients {
        has_item(ingredient.item_id, ingredient.quantity)
        // Expected: has_item(&ingredient.item_id, ingredient.quantity)
        // Generated: has_item(ingredient.item_id.clone(), ingredient.quantity)
    }
}
```

### Expected Rust Output
```rust
pub fn check_inventory(&self, has_item: fn(&String, i32) -> bool) -> bool {
    for ingredient in &self.ingredients {
        if !has_item(&ingredient.item_id, ingredient.quantity) {
            return false;
        }
    }
    true
}
```

### Actual (Buggy) Output
```rust
pub fn check_inventory(&self, has_item: fn(&String, i32) -> bool) -> bool {
    for ingredient in &self.ingredients {
        if !has_item(ingredient.item_id.clone(), ingredient.quantity) {
            // ERROR: expected `&String`, found `String`
            return false;
        }
    }
    true
}
```

## Root Cause

1. **FieldAccess handler** (line 8512 in `generator.rs`) automatically adds `.clone()` for borrowed iterator variables
2. This happens **BEFORE** the Call handler checks parameter ownership
3. Call handler receives `ingredient.item_id.clone()` instead of `ingredient.item_id`
4. Can't apply correct ownership conversion (`&` for Borrowed)

### Code Flow
```
generate_expression(Call { args: [ingredient.item_id] })
  └─> generate_expression(FieldAccess { ingredient, item_id })
      └─> FieldAccess handler sees: ingredient is borrowed iterator
          └─> Adds .clone() → "ingredient.item_id.clone()"  ❌ TOO EARLY!
  └─> Call handler receives: "ingredient.item_id.clone()"
      └─> Can't strip .clone() or add &
      └─> Returns: "ingredient.item_id.clone()"  ❌ WRONG!
```

## Solution

Added `in_call_argument_generation` context flag to suppress premature `.clone()`:

### 1. Added Context Flag
```rust
// generator.rs:133
in_call_argument_generation: bool,
```

### 2. Set Flag When Generating Call Arguments
```rust
// generator.rs:7482
let prev_in_call_arg = self.in_call_argument_generation;
self.in_call_argument_generation = true;

let mut arg_str = self.generate_expression(arg);

self.in_call_argument_generation = prev_in_call_arg;
```

### 3. Suppress .clone() in FieldAccess Handler
```rust
// generator.rs:8513
if !self.generating_assignment_target
    && !self.suppress_borrowed_clone
    && !self.in_explicit_clone_call
    && !self.in_field_access_object
    && !self.in_borrow_context
    && !self.in_call_argument_generation  // NEW: Let Call handler decide
{
    // ... auto-clone logic ...
}
```

### 4. Call Handler Applies Correct Ownership
```rust
// generator.rs:7542 (Borrowed block)
if arg_str.ends_with(".clone()") {
    arg_str = arg_str[..arg_str.len() - 8].to_string();
}
// Add & for Borrowed parameters
if !is_string_literal && !is_param_already_ref && !is_copy_param {
    return vec![format!("&{}", arg_str)];
}
```

## Additional Fixes

### 1. Function Pointers Marked as Copy (analyzer.rs:2666)
```rust
fn is_copy_type(&self, ty: &Type) -> bool {
    match ty {
        // ...
        Type::FunctionPointer { .. } => true,  // NEW: Function pointers are Copy!
        // ...
    }
}
```

### 2. Function Pointer Signature Extraction (generator.rs:7398-7435)
```rust
if let Type::FunctionPointer { params, return_type } = &param.type_ {
    let param_ownership: Vec<OwnershipMode> = params.iter().map(|param_ty| {
        match param_ty {
            Type::String => OwnershipMode::Borrowed,  // string → &String
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool 
                => OwnershipMode::Owned,  // Copy types → owned
            // ...
        }
    }).collect();
    
    Some(FunctionSignature {
        param_types: params.clone(),
        param_ownership,
        return_type: return_type.as_ref().map(|rt| (**rt).clone()),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
    })
}
```

### 3. String Literal Ownership in Function Pointers (generator.rs:7900-7960)
```rust
if matches!(arg, Expression::Literal { value: Literal::String(_), .. }) {
    let should_convert = if let Some(ref sig) = signature {
        if let Some(&ownership) = sig.param_ownership.get(i) {
            matches!(ownership, OwnershipMode::Owned)  // Check signature!
        } else {
            func_name == "new" || func_name.ends_with("::new")
        }
    } else {
        // ...
    };
    
    if should_convert {
        arg_str = format!("{}.to_string()", arg_str);
    }
}
```

## TDD Tests

### Test 1: `test_struct_field_in_loop_no_auto_clone` ✅ PASSING
```windjammer
for item in &items {
    if item.name == "sword" {  // Should NOT clone item.name
        found = true
    }
}
```

### Test 2: `test_struct_field_passed_to_borrowed_param` ✅ PASSING
```windjammer
fn check_inventory(&self, has_item: fn(string, i32) -> bool) -> bool {
    for ingredient in &self.ingredients {
        if !has_item(ingredient.item_id, ingredient.quantity) {
            return false
        }
    }
    true
}
```

**Both tests compile and pass!** 🎉

## Test Output
```
running 2 tests
test test_struct_field_in_loop_no_auto_clone ... ok
test test_struct_field_passed_to_borrowed_param ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Files Changed

1. **windjammer/src/analyzer.rs**
   - Mark `Type::FunctionPointer` as Copy (line 2666)

2. **windjammer/src/codegen/rust/generator.rs**
   - Add `in_call_argument_generation` context flag (line 133)
   - Initialize flag in `new()` (line 254)
   - Set flag when generating call arguments (line 7482)
   - Suppress .clone() in FieldAccess handler (line 8513)
   - Extract function pointer signatures (lines 7398-7435)
   - Match types.rs conversion logic for FP params (lines 7402-7428)
   - Check signature for string literal conversion (lines 7900-7960)

3. **windjammer/tests/bug_struct_field_auto_clone_test.rs**
   - TDD test for loop comparison (test 1)
   - TDD test for function pointer call (test 2)

## Impact

This fix enables **idiomatic Windjammer** code:
- No manual `.clone()` in function calls
- No manual `&` for borrowed parameters
- Compiler infers ownership from signatures
- Clean, Rust-like syntax without Rust's boilerplate

## Philosophy Alignment

✅ **"Inference When It Doesn't Matter, Explicit When It Does"**
- Ownership is mechanical → compiler infers it
- Business logic is explicit → developer writes it

✅ **"Compiler Does the Hard Work, Not the Developer"**
- Automatic ownership inference
- Automatic signature lookup
- Automatic type conversions

✅ **"80% of Rust's power with 20% of Rust's complexity"**
- Memory safety ✅
- Zero-cost abstractions ✅
- No ownership annotations ✅
- No lifetime annotations ✅

## Next Steps

1. ✅ Auto-clone bug fixed
2. ⏭️ Test with windjammer-game compilation
3. ⏭️ Fix remaining E0308 errors (now unblocked!)
4. ⏭️ Implement automatic `self` ownership inference

---

**TDD Methodology: ✅ VALIDATED**
- Write failing test first
- Implement minimal fix
- Verify test passes
- No workarounds, only proper fixes
- This is the Windjammer way! 🚀
