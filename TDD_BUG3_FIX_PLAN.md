# TDD Bug #3 Fix Plan - While Loop Index Type Inference

**Date**: 2026-02-25 04:05 PST

## Bug Summary

**Title**: While loop index incorrectly inferred as i64 instead of usize

**Symptom**:
```windjammer
let mut after_idx = keyframes.len() - 1  // usize
let mut i = 0  // Should be usize, but defaults to i64
while i < keyframes.len() {
    after_idx = i + 1  // ERROR: expected usize, found i64
    i = i + 1
}
```

**Generated Rust** (BUGGY):
```rust
let mut after_idx = self.keyframes.len() - 1;  // usize
let mut i = 0;  // Defaults to i64 ❌
while i < ((self.keyframes.len() - 1) as i64) {  // Cast to i64 ❌
    after_idx = i + 1;  // ERROR: expected usize, found i64
    i += 1;
}
```

**Expected Rust** (CORRECT):
```rust
let mut after_idx = self.keyframes.len() - 1;  // usize
let mut i: usize = 0;  // Explicitly usize ✅
while i < (self.keyframes.len() - 1) {  // No cast needed ✅
    after_idx = i + 1;  // Works! ✅
    i += 1;
}
```

## Root Cause Analysis

### Current Behavior

1. **For loops**: usize inference works correctly (lines 5375-5381)
   ```rust
   if let Some(var) = &loop_var {
       if let Expression::Range { end, .. } = iterable {
           if self.expression_produces_usize(end) {
               self.usize_variables.insert(var.clone());
           }
       }
   }
   ```

2. **While loops**: NO usize inference
   - `let mut i = 0` defaults to i64 (line 6312-6314)
   - No pre-pass to analyze while loop conditions
   - No tracking of usize usage in while loop bodies

### Why This Matters

**Common patterns that break**:
- Array/vector iteration with manual indexing
- Animation keyframe searching
- Sorting algorithms
- Binary search implementations
- Grid/tilemap iteration
- Any algorithm using `len()` with manual loops

**Impact**: Blocks most non-trivial game code

## Fix Strategy: Data Flow Analysis Pre-Pass

### Option 1: Precompute While Loop Index Types (RECOMMENDED)

Add a new pre-pass similar to `precompute_for_loop_borrows`:

```rust
fn precompute_while_loop_usize_indices(&mut self, body: &[&'ast Statement<'ast>]) {
    // Pattern to detect:
    // 1. let mut i = 0 (or similar init)
    // 2. while i < expr.len() (or similar usize comparison)
    // 3. i used with usize variables
    
    for (stmt_idx, stmt) in body.iter().enumerate() {
        // Find: let mut i = 0
        if let Statement::Let { name: Some(var_name), value: Some(init_value), .. } = stmt {
            if is_zero_literal(init_value) {
                // Check subsequent while loop
                if let Some(while_stmt) = body.get(stmt_idx + 1) {
                    if let Statement::While { condition, body: while_body, .. } = while_stmt {
                        // Check if condition is: i < expr.len()
                        if condition_uses_usize(condition, var_name) {
                            self.usize_variables.insert(var_name.clone());
                        }
                    }
                }
            }
        }
    }
}
```

**Pros**:
- Fixes the root cause
- Works for all while loop patterns
- Consistent with existing for-loop logic
- Proper data flow analysis

**Cons**:
- Requires new pre-pass
- More complex implementation
- Need to handle various patterns

### Option 2: Smart Default Based on Context

When generating `let mut i = 0`, check if:
1. Next statement is while loop
2. Condition compares `i` to `.len()`

**Pros**:
- Simpler implementation
- Inline fix

**Cons**:
- Fragile pattern matching
- Doesn't handle complex cases
- Not as thorough as pre-pass

### Option 3: Type Annotation Inference

Add type annotations when ambiguous:
```rust
let mut i: usize = 0;  // Explicit when needed
```

**Pros**:
- Clear and explicit
- Works immediately

**Cons**:
- Defeats "inference when it doesn't matter" philosophy
- Adds noise to generated code
- Temporary workaround, not proper fix

## Recommended Approach: Option 1 (Pre-Pass)

### Implementation Steps

1. **Create pre-pass function** (after line 3693):
   ```rust
   fn precompute_while_loop_usize_indices(&mut self, body: &[&'ast Statement<'ast>])
   ```

2. **Add detection logic**:
   - Find `let mut var = 0` statements
   - Look ahead for `while var < expr.len()`
   - Check if var is assigned to usize variables
   - Add var to `usize_variables` set

3. **Call pre-pass** (in `generate_function`, after line 3693):
   ```rust
   self.precompute_for_loop_borrows(&func.body);
   self.precompute_while_loop_usize_indices(&func.body);  // NEW
   ```

4. **Leverage existing infrastructure**:
   - `usize_variables` set already exists (line 115)
   - `expression_produces_usize` helper exists
   - Type annotation logic exists (line 6320-6321)

5. **Handle edge cases**:
   - Nested loops
   - Multiple index variables
   - Complex conditions
   - Arithmetic (i + 1, i - 1)

### Success Criteria

✅ **Test passes**:
```bash
wj run tests/bug_loop_index_usize_inference.wj
# Should output: ✅ All tests passed
```

✅ **Game library compiles**:
```bash
cd windjammer-game-core && cargo build --lib
# Should complete with 0 E0308 errors
```

✅ **Generated code is clean**:
```rust
let mut i: usize = 0;  // OR just rely on inference
while i < self.keyframes.len() {  // No cast
    after_idx = i + 1;  // No error
    i += 1;
}
```

## Test Case

**File**: `windjammer/tests/bug_loop_index_usize_inference.wj`

**Coverage**:
1. ✅ While loop with .len() comparison
2. ✅ Assignment to usize variable (after_idx)
3. ✅ Arithmetic (i + 1)
4. ✅ Array indexing (keyframes[i])
5. ✅ Nested while loops

## Next Steps

1. Read analyzer/codegen to understand current integer type inference
2. Implement `precompute_while_loop_usize_indices`
3. Update `generate_function` to call new pre-pass
4. Test with bug_loop_index_usize_inference.wj
5. Verify game library compilation
6. Commit with dogfooding win message

## Expected Timeline

- Implementation: 30 minutes
- Testing: 10 minutes
- Verification: 10 minutes
- Total: ~50 minutes

## Philosophy Alignment

✅ **Inference When It Doesn't Matter**: 
- Developer writes `let mut i = 0`
- Compiler infers `usize` from usage
- No manual type annotation needed

✅ **Compiler Does the Hard Work**:
- Data flow analysis automatically
- Smart type inference
- No user burden

✅ **No Workarounds**:
- Fix root cause (type inference)
- Not symptoms (adding `as usize` casts)
- Proper solution for long term

---

**Let's fix this properly!** 🚀
