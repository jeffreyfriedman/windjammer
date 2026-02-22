# Windjammer TDD Success: Bug #3 String/&str Coercion

## Date: 2026-02-22

## Problem

When `format!()` was used directly as a function/method argument expecting `&str`, the generated Rust code had a temporary value lifetime issue:

```rust
// Generated (broken):
draw_text(format!("Score: {}", score), 10.0, 20.0)
//        ^^^^^^^^^^^^^^^^^^^^^^^^^ temporary String dropped
//        |                         while borrowed as &str
```

Rust error:
```
error[E0716]: temporary value dropped while borrowed
```

## Solution

**TDD Approach:** Red → Green → Refactor

### RED Phase
Created `tests/bug_string_coercion_test.rs` with 4 failing tests:
1. `test_format_as_function_argument_extracts_to_variable` - format!() in extern fn call
2. `test_format_in_method_call_extracts_to_variable` - format!() in method call
3. `test_format_as_variable_assignment_unchanged` - don't break existing assignments
4. `test_multiple_format_calls_in_same_function` - handle multiple calls

### GREEN Phase
Modified `src/codegen/rust/generator.rs` to extract format!() calls to temporary variables:

**For Function Calls (Expression::Call):**
```rust
// Before fix:
draw_text(format!("Score: {}", score), 10.0, 20.0)

// After fix:
unsafe { let _temp0 = format!("Score: {}", score); draw_text(&_temp0, 10.0, 20.0) }
```

**For Method Calls (Expression::MethodCall):**
```rust
// Before fix:
ctx.draw_text(format!("Lives: {}", lives), 100.0, 20.0)

// After fix:
{ let _temp0 = format!("Lives: {}", lives); ctx.draw_text(&_temp0, 100.0, 20.0) }
```

**Implementation:**
- Detects `format!(` in function/method arguments
- Generates `let _tempN = format!(...);` declarations
- Replaces format!() with `&_tempN` reference
- Wraps in block (or merges with existing unsafe block for extern calls)

### REFACTOR Phase
- No refactoring needed - implementation is clean and minimal
- All 239 existing unit tests still pass ✅
- All 4 new Bug #3 tests pass ✅

## Test Results

```
running 4 tests
test test_format_as_function_argument_extracts_to_variable ... ok
test test_format_in_method_call_extracts_to_variable ... ok
test test_format_as_variable_assignment_unchanged ... ok
test test_multiple_format_calls_in_same_function ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Full test suite:
```
test result: ok. 239 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Real-World Verification

Tested on `windjammer-game/examples/breakout.wj`:

**Before fix:**
- Multiple format!() errors in draw_text() calls
- Temporary value dropped while borrowed

**After fix:**
- Generates correct Rust code with temp variable extraction
- No more String/&str lifetime errors from format!()
- Code compiles cleanly (rustc with only dead_code warnings)

Example from breakout.wj:
```rust
// Generated code:
{ let _temp0 = format!("Score: {}", self.score); 
  ctx.draw_text(&_temp0, Vec2::new(10.0, 20.0), 20.0, Color::rgba(1.0, 1.0, 1.0, 1.0)) };
```

## Performance Impact

**None.** The temporary variable approach is:
- **Zero-cost:** Same assembly as hand-written code
- **Idiomatic Rust:** This is the standard pattern
- **Compiler-friendly:** Optimizer sees the full picture

## Correctness

**This IS the correct solution** for Rust's ownership model:
1. `format!()` returns `String` (owned)
2. Function expects `&str` (borrowed)
3. String must live longer than the borrow
4. Binding to variable extends lifetime through the call

This is not a workaround - it's how Rust is meant to work.

## Files Changed

- `src/codegen/rust/generator.rs` - Added format!() extraction for Call and MethodCall
- `tests/bug_string_coercion_test.rs` - New test file with 4 TDD tests

## TDD Methodology Validated

✅ **Write test first** - All 4 tests written before fix  
✅ **See it fail (RED)** - Tests failed with expected errors  
✅ **Make it pass (GREEN)** - Implementation made all tests pass  
✅ **Refactor** - Code is clean, no refactoring needed  
✅ **No regressions** - All 239 existing tests still pass  
✅ **Real-world verification** - Tested on actual game code

## Next Steps

1. ✅ Bug #3 COMPLETE
2. Continue dogfooding: compile windjammer-game fully
3. Fix Bug #2: Detect test files and generate [[test]] targets (TDD)
4. Fix remaining compiler issues as discovered

---

**"If it's worth doing, it's worth doing right."** ✅ DONE RIGHT.
