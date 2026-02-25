# Bug #2 Fix Plan - format!() in Custom Enum Variants

## Root Cause Identified ✅

**Location**: `src/codegen/rust/generator.rs` lines 7084-7191

**Problem**: The code only handles format!() temp variable extraction for `Some`, `Ok`, and `Err`, but NOT for custom enum variants like `AssetError::InvalidFormat`.

```rust
// Line 7084 - ONLY handles these 3:
if matches!(func_name.as_str(), "Some" | "Ok" | "Err") {
    // Extract format!() to temp variables...
}
```

**Custom enum variants** (like `AssetError::InvalidFormat`, `DialogueError::InvalidSpeaker`, etc.) go through the general function call path (lines 7218+), which DOESN'T have the format temp extraction logic.

## The Bug

**Windjammer Code**:
```windjammer
Err(AssetError::InvalidFormat(format!("Error: {}", msg)))
```

**Current Generated Rust** (BUGGY):
```rust
Err({ let _temp0 = format!("Error: {}", msg); AssetError::InvalidFormat(&_temp0) })
//                                                                         ^^^^^^^^ BUG!
```

**Should Generate**:
```rust
Err({ let _temp0 = format!("Error: {}", msg); AssetError::InvalidFormat(_temp0) })
//                                                                         ^^^^^^^^ NO &!
```

Or better yet:
```rust
Err(AssetError::InvalidFormat(format!("Error: {}", msg)))
```

## The Fix Strategy

### Option 1: Detect ALL Enum Constructors (BEST)

Use the type registry to detect if `func_name` is an enum constructor (not just Some/Ok/Err), then apply the same format temp extraction logic.

**Pseudocode**:
```rust
let is_enum_variant = self.identifier_registry.get(func_name)
    .map(|info| info.is_enum_constructor)
    .unwrap_or(false);

if is_enum_variant || matches!(func_name.as_str(), "Some" | "Ok" | "Err") {
    // Extract format!() to temp variables...
}
```

### Option 2: Move Format Extraction to General Path (SIMPLER)

Move the format temp extraction logic from the enum-specific block (lines 7093-7127) to the general function call path (after line 7218), so ALL function/enum calls get the fix.

**Pros**: Simpler, handles all cases
**Cons**: Might affect non-enum function calls

### Option 3: Fix the & Addition Logic (TARGETED)

The real bug is that when temp variables are created, something is adding `&` when it shouldn't. Find where that `&` is being added and prevent it for owned parameters.

## Recommended Approach: Option 1

**Extend the existing enum variant handling to detect ALL enum constructors**, not just Some/Ok/Err.

### Implementation Steps:

1. **Check if identifier_registry tracks enum constructors**
   - If yes, use it
   - If no, create a helper to detect enum patterns (CamelCase::CamelCase)

2. **Extend the condition at line 7084**:
   ```rust
   let is_std_enum = matches!(func_name.as_str(), "Some" | "Ok" | "Err");
   let is_custom_enum = self.is_enum_constructor(&func_name);
   
   if is_std_enum || is_custom_enum {
       // Existing format temp extraction logic...
   }
   ```

3. **Add helper function** (if needed):
   ```rust
   fn is_enum_constructor(&self, func_name: &str) -> bool {
       // Check if func_name matches pattern Module::Variant or Enum::Variant
       func_name.contains("::") && {
           let parts: Vec<&str> = func_name.split("::").collect();
           parts.len() == 2 && 
           parts[0].chars().next().map_or(false, |c| c.is_uppercase()) &&
           parts[1].chars().next().map_or(false, |c| c.is_uppercase())
       }
   }
   ```

4. **Test with multiple patterns**:
   - `AssetError::InvalidFormat(format!(...))`
   - `DialogueError::InvalidSpeaker(format!(...))`
   - `CustomEnum::Variant(format!(...))`

5. **Verify no regression** on Some/Ok/Err

## Test Cases

### Test 1: Custom Enum with format!()
```windjammer
enum AssetError {
    InvalidFormat(String),
}

fn test() -> Result<(), AssetError> {
    Err(AssetError::InvalidFormat(format!("Error: {}", code)))
}
```

**Expected Rust**:
```rust
Err(AssetError::InvalidFormat(format!("Error: {}", code)))
```

### Test 2: Nested format!()
```windjammer
enum DialogueError {
    InvalidSpeaker(String, i32),
}

fn test() -> Result<(), DialogueError> {
    Err(DialogueError::InvalidSpeaker(format!("Name: {}", name), id))
}
```

**Expected Rust**:
```rust
Err({ let _temp0 = format!("Name: {}", name); DialogueError::InvalidSpeaker(_temp0, id) })
```

### Test 3: Multiple format!() args
```windjammer
enum MultiError {
    Compound(String, String),
}

fn test() -> Result<(), MultiError> {
    Err(MultiError::Compound(format!("A: {}", a), format!("B: {}", b)))
}
```

**Expected Rust**:
```rust
Err({ let _temp0 = format!("A: {}", a); let _temp1 = format!("B: {}", b); MultiError::Compound(_temp0, _temp1) })
```

## TDD Workflow

1. ✅ **Create failing test** - `tests/bug_format_temp_var_lifetime.wj` (DONE)
2. **Run test, confirm failure** - Should show E0308 type mismatch
3. **Implement fix** - Extend enum detection
4. **Run test, confirm success** - Should compile and run
5. **Run full test suite** - Ensure no regressions
6. **Document fix** - Update COMPILER_BUGS_TO_FIX.md

## Success Criteria

- [ ] Test `bug_format_temp_var_lifetime.wj` compiles successfully
- [ ] Assets loader.wj compiles (no E0308 errors)
- [ ] All existing tests still pass
- [ ] Generated code has no `&_temp` patterns for owned params
- [ ] Documentation updated

## The Windjammer Way ✅

- **No workarounds**: Fix the root cause in codegen
- **Proper fix**: Detect ALL enum constructors, not just Some/Ok/Err
- **TDD**: Test first, then fix, then verify
- **Comprehensive**: Handle all patterns (single, multiple, nested)
