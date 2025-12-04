# Dogfooding Win #37: ASI Before Parenthesized Expressions

**Date**: 2025-12-01
**Type**: Compiler Bug Fix (TDD)
**Impact**: Critical - Affects any code with multi-line expressions

## The Bug

When a newline appeared before a parenthesized expression, the ASI (Automatic Semicolon Insertion) failed to insert a semicolon, causing the parser to treat the `(` as a function call instead of a new statement.

### Bad Code Generated

**Windjammer source:**
```windjammer
let dx = 3.0
let dy = 4.0
let dz = 5.0
(dx * dx + dy * dy + dz * dz).sqrt()
```

**Before fix (WRONG):**
```rust
let dx = 3.0;
let dy = 4.0;
let dz = 5.0(dx * dx + dy * dy + dz * dz).sqrt();  // ❌ Treats ( as function call!
```

**After fix (CORRECT):**
```rust
let dx = 3.0;
let dy = 4.0;
let dz = 5.0;
(dx * dx + dy * dy + dz * dz).sqrt()  // ✅ Separate statement
```

## Root Cause

The `had_newline_before_current()` method in `parser_impl.rs` was just a stub that always returned `false`, disabling ASI checks entirely.

```rust
pub(crate) fn had_newline_before_current(&self) -> bool {
    // Stub implementation - always return false for now
    false
}
```

The ASI check existed at line 1606 of `expression_parser.rs`:
```rust
if self.had_newline_before_current() {
    // ASI: Treat newline as statement terminator
    break;
}
```

But it never triggered because the method always returned `false`!

## The Fix

Implemented proper newline detection by comparing line numbers between tokens:

```rust
pub(crate) fn had_newline_before_current(&self) -> bool {
    if self.position == 0 {
        return false; // No previous token
    }
    
    let prev_token = self.tokens.get(self.position - 1);
    let curr_token = self.tokens.get(self.position);
    
    match (prev_token, curr_token) {
        (Some(prev), Some(curr)) => {
            // If the line number changed, there was a newline
            curr.line > prev.line
        }
        _ => false,
    }
}
```

## TDD Process

### 1. Red Phase: Created Failing Test

**File**: `tests/asi_paren_integration_test.rs`

```rust
#[test]
fn test_asi_before_parenthesized_expression() {
    let wj_code = r#"
pub fn test_asi() -> f32 {
    let dx = 3.0
    let dy = 4.0
    let dz = 5.0
    (dx * dx + dy * dy + dz * dz).sqrt()
}
"#;
    // ... compile and check it doesn't generate "dz(dx"
}
```

**Result**: ❌ FAILED (confirmed bug)
```
ASI should insert semicolon after let statement.
Generated: let dz = 5.0(dx * dx + dy * dy + dz * dz).sqrt();
```

### 2. Green Phase: Fixed the Compiler

Implemented `had_newline_before_current()` properly.

**Result**: ✅ PASSED

### 3. Refactor Phase: Verified No Regressions

Ran full compiler test suite:

```
test result: ok. 206 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Impact

- **Game engine errors**: 66 → 62 (-4 errors fixed by ASI)
- **Compiler correctness**: Multi-line expressions now work correctly
- **Code clarity**: Windjammer code can be written without semicolons naturally

## Files Changed

- `windjammer/src/parser_impl.rs` - Implemented `had_newline_before_current()`
- `windjammer/tests/asi_paren_integration_test.rs` - Added test (NEW)
- `windjammer/tests/asi_paren_expression_test.wj` - Test source (NEW)

## Philosophy Validated

✅ **No workarounds!** When the user said "add explicit semicolons as a workaround", I was about to do that, but they correctly said "Nope! we fix problems as we encounter them!"

This is the **Windjammer Way**:
- Fix root causes, not symptoms
- Write tests first (TDD)
- Proper fixes prevent future bugs
- No technical debt

## Related Issues

This fix will help with:
- `vec3.wj` distance calculation (was generating `dz(dx * dx...)`)
- Any multi-line expression in the codebase
- Future code written in natural, semicolon-free style

---

**Status**: ✅ Fixed, Tested, Verified
**Test Coverage**: 207 tests passing (added 1 new ASI test)
**Regressions**: 0


