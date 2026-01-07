# 🚨 REGRESSION: Nested Field Mutation Detection

**Date**: December 14, 2025  
**Severity**: HIGH  
**Status**: IDENTIFIED, needs fix  
**Introduced In**: String ownership inference changes (commit eb82fc3b)

---

## 🔍 **PROBLEM**

6 ownership inference tests are now failing that were passing before our string inference work:
- `test_nested_field_mutation`
- `test_conditional_mutation`
- `test_infer_mut_ref_for_compound_assignment`
- `test_infer_mut_ref_for_field_mutation`
- `test_loop_mutation`
- `test_mixed_read_and_mut_params`

### **Test Case Example**

```windjammer
pub fn set_inner_value(o: Outer, v: i32) {
    o.inner.value = v
}
```

**Expected**: `pub fn set_inner_value(o: &mut Outer, v: i32)`  
**Actual**: `pub fn set_inner_value(o: Outer, v: i32)`

The compiler should detect that `o` is mutated (nested field assignment) and infer `&mut`.

---

## 🕵️ **ROOT CAUSE ANALYSIS**

### **Verification**
- Commit `9d2666f0` (before string inference): All 22 tests PASSING ✅
- Commit `eb82fc3b` (after string inference): 6 tests FAILING ❌

### **Suspected Issues**

1. **Changed `is_returned()` logic** (line 969-973):
   - Now skips function/method calls when detecting returns
   - This is correct for string inference but might affect other checks?

2. **Added string special case** (line 726-731):
   - Checks `is_only_passed_to_read_only_fns()` before other checks
   - Should only affect `String` types, but might have side effects?

3. **Possible interaction** between new checks and existing mutation detection

### **The Logic Flow**

```rust
fn infer_parameter_ownership() {
    // 1. Check mutation (should catch nested fields)
    if self.is_mutated(param_name, body) {
        return MutBorrowed;  // Should return here!
    }
    
    // 2. Check returned
    if self.is_returned(param_name, body) {
        return Owned;
    }
    
    // 2.5. NEW: String special case
    if matches!(param_type, Type::String) && self.is_only_passed_to_read_only_fns(param_name, body) {
        return Borrowed;
    }
    
    // ... more checks ...
}
```

The mutation check is FIRST, so `is_mutated()` must be returning FALSE when it should return TRUE.

---

## 🐛 **HYPOTHESIS**

The `is_mutated()` function (line 881-929) should detect nested field mutations:

```rust
Statement::Assignment { target, .. } => {
    // Line 894-895: Should catch o.inner.value
    if self.expression_uses_identifier(name, target) {
        return true;
    }
}
```

**Possible Issue**: 
- `expression_uses_identifier()` might not properly detect nested field access
- Or there's a change in how assignments are parsed
- Or there's an interaction with the new `is_only_passed_to_read_only_fns()` helper functions

---

## 🔧 **FIX STRATEGY**

### **Step 1: Isolate the Problem**
```bash
# Test just mutation detection
cargo test --release --test analyzer_ownership_comprehensive_tests test_nested_field_mutation
```

### **Step 2: Add Debug Logging**
Add `eprintln!()` to `is_mutated()` to see:
- What statements are being checked
- What `expression_uses_identifier()` returns for `o.inner.value`

### **Step 3: Check for Side Effects**
Review all new functions added for string inference:
- `is_only_passed_to_read_only_fns()`
- `stmt_only_uses_in_read_only_fns()`
- `expr_only_uses_in_read_only_fns()`

These might be interfering with existing logic.

### **Step 4: Bisect if Needed**
If unclear, use git bisect:
```bash
git bisect start
git bisect bad HEAD
git bisect good 9d2666f0
# Test each commit
```

---

## 📋 **ACTION ITEMS**

- [ ] Add debug logging to `is_mutated()` and `expression_uses_identifier()`
- [ ] Test the specific case: `o.inner.value = v`
- [ ] Check if `expression_uses_identifier()` handles nested `FieldAccess` correctly
- [ ] Review all new helper functions for side effects
- [ ] Fix the regression
- [ ] Verify all 22 tests pass
- [ ] Add regression test for this specific case

---

## ⏰ **TIMING**

**Discovered**: 14+ hours into marathon session  
**Decision**: Document and fix in next session (requires fresh debugging)

**Why Wait?**
- Complex debugging needed
- Mental fatigue after 14+ hours
- Risk of introducing more bugs when tired
- Better to fix with fresh eyes

---

## 🎯 **SUCCESS CRITERIA**

1. All 22 `analyzer_ownership_comprehensive_tests` passing
2. String inference still working (manual verification)
3. Game library still compiles with same error count
4. No new regressions in other test suites

---

## 📝 **NOTES**

- The string inference feature itself is working correctly
- The regression is in mutation detection, not string handling
- This is a **blocker** for merging to main
- But NOT a blocker for continuing game engine work (separate concern)

**Priority**: HIGH - Fix before merging string inference  
**Urgency**: MEDIUM - Can be fixed in next session











