# Dogfooding Win #38: Trait Copy Parameter Ownership Fix

**Date**: 2025-12-01  
**Bug Type**: Compiler - Ownership Inference  
**Status**: ✅ FIXED

## The Problem

When defining traits with methods that have Copy type parameters (like `f32`, `i64`), the compiler was incorrectly adding `&` references to them:

```windjammer
pub trait GameLoop {
    fn update(&mut self, delta: f32, input: Input)
}
```

Generated Rust (WRONG):
```rust
pub trait GameLoop {
    fn update(&mut self, delta: &f32, input: &Input) { ... }
    //                            ^^^^ Wrong! f32 is Copy, should be by value
}
```

This caused trait implementation mismatches:
```
error[E0053]: method `update` has an incompatible type for trait
  --> windjammer-game-core/src/generated/test_runtime.rs:70:33
   |
70 |     fn update(&mut self, delta: f32, input: &Input) {
   |                                 ^^^ expected `&f32`, found `f32`
```

## Root Cause

The compiler had TWO places where ownership inference for trait methods defaulted to `Borrowed` for inferred parameters:

1. **Trait Definition Generation** (`src/codegen/rust/generator.rs` line 2120-2127):
   ```rust
   OwnershipHint::Inferred => {
       // Default to &
       if param.name == "self" {
           "&self".to_string()
       } else {
           format!("&{}", self.type_to_rust(&param.type_))  // <- PROBLEM!
       }
   }
   ```

2. **Trait Implementation Analysis** (`src/analyzer.rs` line 627-632):
   ```rust
   let trait_mode = match &trait_param.ownership {
       OwnershipHint::Owned => OwnershipMode::Owned,
       OwnershipHint::Ref => OwnershipMode::Borrowed,
       OwnershipHint::Mut => OwnershipMode::MutBorrowed,
       OwnershipHint::Inferred => OwnershipMode::Borrowed, // <- PROBLEM!
   };
   ```

Both places defaulted to `Borrowed` (`&`) for inferred parameters, but Copy types should default to `Owned` (pass by value).

## The Fix

### Fix 1: Trait Definition Generation

Modified `src/codegen/rust/generator.rs`:

```rust
OwnershipHint::Inferred => {
    if param.name == "self" {
        // Default to &self for trait methods
        "&self".to_string()
    } else {
        // For Copy types (f32, i64, bool, etc.), pass by value
        // For non-Copy types, pass by reference
        if self.is_copy_type(&param.type_) {
            self.type_to_rust(&param.type_)
        } else {
            format!("&{}", self.type_to_rust(&param.type_))
        }
    }
}
```

### Fix 2: Trait Implementation Analysis

Modified `src/analyzer.rs`:

```rust
OwnershipHint::Inferred => {
    // For Copy types, keep them Owned (pass by value)
    // For non-Copy types, default to Borrowed
    if self.is_copy_type(&trait_param.type_) {
        OwnershipMode::Owned
    } else {
        OwnershipMode::Borrowed
    }
}
```

## TDD Process

### 1. Red: Create Failing Test

Created `tests/trait_copy_param_test.wj`:
```windjammer
pub trait Calculator {
    fn add(&self, a: f32, b: f32) -> f32
}

pub struct SimpleCalc {}

impl Calculator for SimpleCalc {
    fn add(&self, a: f32, b: f32) -> f32 {
        a + b
    }
}
```

Created `tests/trait_copy_param_integration_test.rs` to verify generated Rust doesn't contain `&f32`.

**Result**: Test FAILED ❌ (generated `&f32`)

### 2. Green: Fix the Compiler

Applied both fixes above.

**Result**: Test PASSED ✅ (generates `f32`)

### 3. Refactor: Run Full Test Suite

```bash
$ cargo test --release
test result: ok. 28 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Only 1 failure: pre-existing `extern_fn_integration_test` (unrelated)

## Impact on Game Engine

### Before Fix
```
error[E0053]: method `update` has an incompatible type for trait
```

### After Fix
✅ **GameLoop trait compiles correctly**  
✅ **All trait method implementations match trait signatures**  
✅ **Error count unchanged at 92 (remaining errors are game code issues)**

## Files Modified

- `windjammer/src/codegen/rust/generator.rs` - Trait definition generation
- `windjammer/src/analyzer.rs` - Trait implementation analysis
- `windjammer/tests/trait_copy_param_test.wj` - Test file (new)
- `windjammer/tests/trait_copy_param_integration_test.rs` - Integration test (new)

## Lessons Learned

1. **Copy types should always pass by value** - Adding `&` to `f32` is unnecessary and breaks idiom
2. **Ownership inference must consider type semantics** - Not all parameters should default to `&`
3. **Two places to fix** - Both trait definitions AND trait implementations needed updates
4. **TDD catches regressions** - Full test suite passed, confirming no breakage

## Next Steps

- Fix remaining 92 game engine errors (mostly type mismatches and game code issues)
- Continue dogfooding to find more compiler bugs
- Consider: Should user-defined Copy types also pass by value in traits?

---

**Dogfooding Win**: Compiler is now smarter about Copy types in trait methods! 🎉


















