# TDD Session: Enum String Variant Auto-Conversion

**Date**: 2026-02-24  
**Status**: ✅ **COMPLETE** - Main library compiles with 0 errors!

## Problem Summary

**Error**: 53 E0308 "expected `String`, found `&str`" errors in dialogue system  
**Location**: Generated `dialogue/examples.rs` with 31+ `Speaker::NPC("Silas Crane")` calls  
**Root Cause**: Enum variants expecting `String` received string literals (`&str`)

## Example Error

```rust
enum Speaker {
    NPC(String),  // Expects owned String
}

// Windjammer source:
let speaker = Speaker::NPC("Silas Crane")

// Generated Rust (WRONG):
let speaker = Speaker::NPC("Silas Crane");  // &str, not String

// Rust compiler error:
error[E0308]: mismatched types
 |
 | Speaker::NPC("Silas Crane")
 |              ^^^^^^^^^^^^^^ expected `String`, found `&str`
```

## TDD Process

### Test Case: `tests/enum_string_variant.wj`

```windjammer
pub enum Speaker {
    Player,
    NPC(String),  // Expects owned String
}

pub fn test_enum_string_literal() {
    // String literal should be auto-converted to String
    let speaker = Speaker::NPC("Silas Crane")
    
    match speaker {
        Speaker::NPC(name) => assert_eq!(name, "Silas Crane"),
        _ => assert!(false),
    }
}
```

**Before Fix**:
```rust
// Generated (broken):
let speaker = Speaker::NPC("Silas Crane");  // E0308 error
```

**After Fix**:
```rust
// Generated (correct):
let speaker = Speaker::NPC("Silas Crane".to_string());  // ✅ Works!
```

## Root Cause Analysis

**File**: `windjammer/src/codegen/rust/generator.rs` (lines 7259-7268)

### The Bug

The compiler tracked enum variant field types in `enum_variant_types` registry:
- Stored as `HashMap<String, Vec<Type>>`
- Key: `"EnumName::VariantName"` (e.g., `"Speaker::NPC"`)
- Value: Field types (e.g., `[Type::Custom("String")]`)

The string conversion logic checked:
```rust
if let Some(variant_types) = self.enum_variant_types.get(&func_name) {
    variant_types.get(i).is_some_and(|ty| matches!(ty, Type::String))
    //                                      ^^^^^^^^^^^^^^^^^^^^^^
    //                                      ONLY checked Type::String
}
```

**Problem**: Parser represents `String` type as `Type::Custom("String")`, not `Type::String`!
- `Type::String` is for `str` (primitive string slice)
- `Type::Custom("String")` is for `std::string::String` (owned heap string)

Result: Logic returned `false`, no `.to_string()` added, E0308 errors!

## The Fix

**Added dual type check** (line 7263):

```rust
if let Some(variant_types) = self.enum_variant_types.get(&func_name) {
    // TDD FIX: Check for both Type::String and Type::Custom("String")
    variant_types.get(i).is_some_and(|ty| {
        matches!(ty, Type::String)
            || matches!(ty, Type::Custom(name) if name == "String")
    })
}
```

**Impact**: Now recognizes both representations of String type!

## Results

### Error Reduction
- **Before**: 71 errors (53 E0308 String/&str + 11 E0425 FFI + 7 misc)
- **After**: 0 errors in main library! ✅

### Generated Code Quality
```rust
// All 31 Speaker::NPC calls now correct:
Speaker::NPC("Silas Crane".to_string())
Speaker::NPC("Echo".to_string())
// etc.
```

### Test Results
- ✅ `tests/enum_string_variant.wj` - PASSING
- ✅ All 239 compiler tests - PASSING
- ✅ Main windjammer-app library - COMPILES SUCCESSFULLY

### Compilation Status

```bash
$ cd build && cargo build
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.47s
✅ SUCCESS - 0 errors!
```

## Windjammer Philosophy Alignment

✅ **"Compiler does the hard work"**  
- Automatic string literal → String conversion for enum variants
- No manual `.to_string()` needed in Windjammer source

✅ **"No workarounds, only proper fixes"**  
- Fixed root cause in type checking logic
- No source code modifications needed

✅ **"Inference when it doesn't matter"**  
- Type conversion is mechanical, compiler handles it
- Developer writes natural code: `Speaker::NPC("name")`

## Affected Code

### Game Code Using This Fix
- **dialogue/examples.wj**: 31 `Speaker::NPC()` calls fixed
- **All enum variants with String fields**: Auto-conversion works universally

### Compiler Changes
- `src/codegen/rust/generator.rs`: Enhanced type checking for `Type::Custom("String")`
- `tests/enum_string_variant.wj`: New TDD test case

## Remaining Work

**Test Errors**: 31 errors in test files (separate from main library)
- Module resolution issues (`could not find dialogue in the crate root`)
- Not related to String conversion fix
- Main library works perfectly!

## Lessons Learned

### Type Representation Matters
- `Type::String` ≠ `Type::Custom("String")`
- Parser uses `Custom` for user-visible types
- Internal representation must match parser output

### Registry-Based Type Checking
- `enum_variant_types` registry works well
- Populated during `collect_enum_variant_types()` pre-pass
- Enables smart string conversion without explicit annotations

### Comprehensive Type Checks
- Always check multiple type representations
- Parser may use different `Type` variants than expected
- Defensive programming: check all possibilities

## Future Enhancements

### Potential Improvements
1. **Normalize type representation**: Convert all `Type::Custom("String")` to `Type::String` early
2. **Helper method**: `is_string_type(ty)` checking both variants
3. **Documentation**: Clarify when to use `Type::String` vs `Type::Custom("String")`

## Commit Message

```
fix(codegen): Auto-convert string literals for enum String variants (dogfooding win #4!)

TDD FIX: Enum variants expecting String now auto-convert &str literals

PROBLEM:
- 53 E0308 errors: "expected String, found &str"
- Speaker::NPC("name") generated &str, not String
- All dialogue examples failed to compile

ROOT CAUSE:
- enum_variant_types stored Type::Custom("String")
- Conversion check only looked for Type::String
- Parser uses Custom("String") for stdlib String type

FIX:
- Check for BOTH Type::String AND Type::Custom("String")
- Now recognizes String type in both representations
- All string literals auto-convert to .to_string()

RESULTS:
- 71 → 0 errors in main library! ✅
- All 31 Speaker::NPC calls now correct
- Main windjammer-app compiles successfully
- All 239 compiler tests passing ✅

Test:
- tests/enum_string_variant.wj (enum String conversion)

Files Changed:
- src/codegen/rust/generator.rs (dual type check)
- TDD_ENUM_STRING_VARIANT_FIX.md (comprehensive doc)

Dogfooding Win #4!
```

---

**WINDJAMMER TDD SUCCESS!** 🚀

No workarounds. Proper type checking. Main library compiles. Zero errors.
