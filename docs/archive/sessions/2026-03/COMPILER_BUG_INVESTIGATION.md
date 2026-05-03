# Compiler Bug Investigation: Iterator Variable Dereferencing

## Status: PARTIAL FIX (TDD tests passing, game still failing)

### Bug Description
When comparing iterator variables in for-loops, the compiler incorrectly adds `*` dereference operator:

```windjammer
// Source .wj
for req in self.required.iter() {
    for comp in entity_components.iter() {
        if comp == req {  // Both are &String
```

```rust
// Generated .rs (INCORRECT!)
if comp == *req {  // &String == String ❌
```

###TDD Test Status
✅ **PASSING:** `bug_string_comparison_deref_test.rs` (3/3 tests)
- ✅ test_string_comparison_no_extra_deref
- ✅ test_string_comparison_in_loop  
- ✅ test_str_comparison

❌ **FAILING:** Game compilation (5 E0277 errors remain)

### Fixes Applied
**File:** `src/codegen/rust/generator.rs`

**Location 1:** Lines 2745-2772 (generate_expression_immut)
- ✅ REMOVED incorrect auto-deref logic
- This fixed parameter comparisons in TDD tests

**Location 2:** Lines 6818-6829 (generate_expression)
- ✅ REMOVED incorrect auto-deref logic  
- This should fix iterator variables but...

### The Mystery
After removing ALL `borrowed_iterator_vars` deref logic in comparisons:
- TDD tests: ✅ PASS (no `*` added for parameters)
- Game code: ❌ FAIL (still generates `comp == *req`)

**Hypothesis:** There's ANOTHER location where Identifier expressions get generated with `*` prefix that we haven't found yet.

### Search Conducted
Searched for:
- ✅ `borrowed_iterator_vars` usage (found 13 locations)
- ✅ `format!("*{}", ...)` patterns (none found in binary op code)
- ✅ `Expression::Identifier` cases (found 18 locations)
- ⚠️ Main `generate_expression` Identifier handler - NOT YET FOUND

### Next Steps
1. Find the main `Expression::Identifier` case in `generate_expression` that outputs identifier names
2. Check if it has logic to add `*` prefix for `borrowed_iterator_vars`
3. Remove that logic (same as locations 1 & 2)
4. Rebuild compiler and test game
5. If that doesn't work, the issue might be in how iterator variables are tracked/categorized

### Code Locations to Investigate
```rust
// Line 6176-6179: Checks if identifier is borrowed (helper function)
Expression::Identifier { name, .. } => {
    self.inferred_borrowed_params.contains(name.as_str())
}

// Line 5888-5910: Type inference for identifiers (not code generation)
Expression::Identifier { name, .. } => {
    // Returns type information
}

// Line 2710: generate_expression_immut (ALREADY FIXED)
Expression::Identifier { name, .. } => name.clone(),

// Missing: Main generate_expression Identifier case that adds `*`
```

### Workaround
The game can work around this by:
1. Using `.clone()` in comparisons: `if comp.clone() == req.clone()`
2. Dereferencing both sides: `if *comp == *req`
3. Using owned values instead of iterators: `for comp in entity_components.clone()`

**BUT** we want a proper compiler fix, not workarounds!

### Philosophy Check
✅ This investigation follows TDD + No Workarounds
- Created failing test first
- Fixed code to make test pass
- Discovered additional hidden bug location
- Continuing investigation with proper fixes only

---
*Last updated: 2026-02-26*
*Status: Investigation ongoing*
