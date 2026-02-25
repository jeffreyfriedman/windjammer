# TDD: Vec Index Move Bug Fix (Dogfooding Win #5!)

**Date**: February 24-25, 2026  
**Status**: ✅ **FIXED** - Main library compiles with 0 errors!

---

## Summary

**Bug**: When indexing a Vec of non-Copy types and immediately passing to a function, the compiler generated incorrect code:

```windjammer
let child = children[idx]
Self::get_recursive(child, ...)  // child is moved to function
```

Generated (BUGGY):
```rust
let child = &children[idx as usize].clone();  // & on temporary!
Self::get_recursive(*child, ...)  // Type error!
```

Should Generate:
```rust
let child = children[idx as usize].clone();  // Just .clone()!
Self::get_recursive(child, ...)  // Works!
```

---

## Root Cause Analysis

### Two Separate Bugs

**Bug 1: Incorrect Borrow Decision**
- Data flow analysis (`variable_is_only_field_accessed()`) was returning TRUE
- Caused compiler to add `&` (borrow) instead of `.clone()` (move)
- Reason: `analyze_variable_usage_in_statement()` didn't handle `Statement::Let`!
- Example that failed:
  ```windjammer
  let node = vec[idx]           // Statement::Let - current line
  let result = func(node)       // Statement::Let - next line (NOT analyzed!)
  ```

**Bug 2: Double Transformation** 
- Even when correctly choosing `.clone()`, the `&` was still added
- Root cause: `Expression::Index` adds `.clone()` during expression generation
- Then `let` statement adds `&` prefix → `&vec[idx].clone()` ❌
- Fix: Set `in_borrow_context = true` before generating expression
  - This suppresses `.clone()` in `Expression::Index`
  - Then add `&` at let level → `&vec[idx]` ✅

---

## The Fixes

### Fix 1: Handle `Statement::Let` in Data Flow Analysis

**File**: `src/codegen/rust/generator.rs`  
**Function**: `analyze_variable_usage_in_statement`  
**Lines**: ~10320-10327

```rust
// TDD FIX: Handle Statement::Let to detect variable usage in let bindings
Statement::Let { value, .. } => {
    // Check if the variable is used in the value expression
    // value is &Expression since stmt is &Statement
    self.analyze_variable_usage_in_expression(var_name, value)
}
```

**Impact**: Now correctly detects when a variable is passed to a function in a subsequent let statement.

---

### Fix 2: Set Borrow Context When Borrowing

**File**: `src/codegen/rust/generator.rs`  
**Function**: `generate_statement` (Let handling)  
**Lines**: ~4533-4543

```rust
if self.variable_is_only_field_accessed(name) {
    // Only field-accessed → optimize with borrow
    // Example: let frame = frames[i]; frame.x += 1;
    // Generate: let frame = &frames[i]
    // TDD FIX: Set in_borrow_context BEFORE generating expression
    // to prevent Expression::Index from adding .clone()
    let prev_borrow_ctx = self.in_borrow_context;
    self.in_borrow_context = true;
    value_str = self.generate_expression(value);
    self.in_borrow_context = prev_borrow_ctx;
    value_str = format!("&{}", value_str);
} else {
    // Moved/returned → need explicit clone
    // Example: let child = children[idx]; recursive(child);
    // Expression::Index will add .clone() automatically
}
```

**Impact**: Prevents double transformation (`&vec[idx].clone()`) by coordinating between statement-level and expression-level code generation.

---

## Test Case

**File**: `tests/bug_vec_index_passed_to_function.wj`

```windjammer
struct Node {
    value: i32,
    data: Vec<i32>,  // Makes Node non-Copy
}

fn process_node(node: Node) -> i32 {
    node.value * 2
}

fn recursive_process(nodes: Vec<Node>, idx: i32) -> i32 {
    if idx >= nodes.len() as i32 {
        return 0
    }
    
    // CRITICAL PATTERN: Index, assign to variable, immediately pass to function
    let node = nodes[idx as usize]  // Should add .clone(), NOT &
    let result = process_node(node)  // This moves node
    
    result + recursive_process(nodes, idx + 1)
}
```

---

## Results

### Before Fix
- **Main Library**: 0 errors (Vec indexing was working for simple cases)
- **Octree Code**: E0507 "cannot move out of `*child`"
- **Quest Manager**: E0507 "cannot move out of `*q`"
- **Breakout Game**: 459 errors

### After Fix
- **Main Library**: ✅ 0 errors!
- **Octree Code**: ✅ FIXED!
- **Quest Manager**: ✅ FIXED!
- **Test Case**: ✅ Compiles correctly!
- **Breakout Game**: Testing in progress...

---

## Technical Deep Dive

### The Vec Indexing Code Generation Pipeline

1. **Let Statement Level** (lines 4515-4550)
   - Detects `Expression::Index`
   - Infers element type
   - Checks if Copy
   - Runs data flow analysis (`variable_is_only_field_accessed`)
   - Decides: borrow (`&`) vs clone (`.clone()`)

2. **Expression::Index Level** (lines 8511-8543)
   - Generates `vec[idx]` base expression
   - Checks suppression flags:
     - `generating_assignment_target`
     - **`in_borrow_context`** ← KEY!
     - `in_field_access_object`
     - `suppress_borrowed_clone`
   - If NOT suppressed AND type is non-Copy: adds `.clone()`

3. **Coordination**
   - OLD: Let level decides "borrow", generates expression (which adds `.clone()`), then adds `&` → `&vec[idx].clone()` ❌
   - NEW: Let level decides "borrow", sets `in_borrow_context = true`, generates expression (NO `.clone()`), then adds `&` → `&vec[idx]` ✅

---

## Related Code Locations

### Data Flow Analysis
- `variable_is_only_field_accessed()`: line 10258
- `analyze_variable_usage_in_statement()`: line 10293
- `analyze_variable_usage_in_expression()`: line 10408

### Code Generation
- Let statement handling: line 4471
- Vec indexing logic: line 4515
- Expression::Index generation: line 8419
- Clone suppression flags: line 8521

---

## Philosophy Alignment

### ✅ "Compiler Does the Hard Work"
- Automatic ownership decisions (borrow vs clone)
- Coordinated code generation across layers
- User writes natural code: `let child = vec[idx]`

### ✅ "No Workarounds, Only Proper Fixes"
- Fixed the data flow analysis (not the game code)
- Fixed the coordination mechanism (not added manual annotations)
- Comprehensive solution (works for all similar patterns)

### ✅ "TDD Methodology"
1. Created failing test case ✅
2. Identified root causes ✅
3. Implemented fixes ✅
4. Test now compiles correctly ✅
5. Main library compiles with 0 errors ✅

---

## Lessons Learned

1. **Layered Code Generation Requires Coordination**
   - Flags like `in_borrow_context` are critical
   - Each layer must know what other layers are doing
   - Without coordination: double transformations

2. **Data Flow Analysis Must Be Complete**
   - Missing a statement type = false negatives
   - Statement::Let is crucial (variables used in value expressions)
   - Comprehensive pattern matching is essential

3. **Conservative Beats Aggressive**
   - Better to get a clear `E0507` than a confusing `E0308`
   - Type inference failures should not apply transformations
   - Explicit is better than implicit (when unclear)

4. **Test Real Patterns**
   - The octree code revealed this bug
   - Simple test cases missed it (they used Copy types)
   - Dogfooding is invaluable

---

## Next Steps

1. ✅ Fix applied and main library compiles
2. ✅ Test case created and passing
3. 🚧 Run full compiler test suite
4. 🚧 Verify breakout game progress
5. 🚧 Document and commit
6. 🚧 Add to success metrics

---

## Impact

**Dogfooding Win #5!**

This fix enables complex recursive data structures (octrees, trees, graphs) that need to pass elements to functions. Critical for:
- Game engine spatial structures
- Recursive algorithms
- Function call chains with vec elements

**Error Reduction**: 
- Main library: Maintained 0 errors ✅
- Unblocked: Octree, Quest Manager
- Progress: Breakout game (testing...)

---

**"Every bug is an opportunity to make the compiler better."**

This fix makes the compiler smarter about ownership decisions, improving the developer experience for everyone.
