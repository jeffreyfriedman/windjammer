# Windjammer-UI Fix Plan

**Status:** IN PROGRESS  
**Errors Remaining:** ~100+ (down from 356!)

---

## ✅ Progress So Far

### Fixed Issues
1. **Duplicate Module Declarations** - Removed 27 hand-written files that were incorrectly in `generated/`
2. **Missing `lib.rs`** - Deleted auto-generated `lib.rs` that conflicted with `mod.rs`
3. **Module Path Issues** - Cleaned up `generated/mod.rs` to only reference actually generated files

### Error Reduction
- **Before:** 356 errors
- **After fixes:** ~100 errors
- **Main remaining issue:** E0424 - Missing `self` parameters in methods

---

## 🐛 Current Issue: E0424 - Missing `self` Parameters

### Root Cause
Windjammer source files (`.wj`) have methods that use `self` in the body but don't declare `self` in parameters:

```windjammer
// avatar.wj (INCORRECT)
pub fn alt(alt: string) -> Avatar {
    self.alt = alt  // ERROR: Uses self but doesn't declare it!
    self
}
```

### Expected Behavior
Either:
1. **Option A:** User must explicitly declare `self`:
   ```windjammer
   pub fn alt(self, alt: string) -> Avatar {
       self.alt = alt
       self
   }
   ```

2. **Option B:** Compiler auto-infers `self` when it's used in body (THE WINDJAMMER WAY)
   - Analyzer detects `self` usage
   - Automatically adds `self` parameter
   - Chooses ownership: `self`, `&self`, or `&mut self`

---

## 🔧 Solution Options

### Option 1: Fix All `.wj` Source Files (Manual)
**Pros:**
- Explicit is clear
- Matches current compiler behavior

**Cons:**
- Must fix ~50+ component files manually
- Goes against Windjammer philosophy (compiler should infer)
- Not scalable

### Option 2: Implement Auto-Self Inference (Compiler Fix)
**Pros:**
- THE WINDJAMMER WAY - compiler does the work
- Fixes all files automatically
- More ergonomic for users
- Consistent with other inference (ownership, mutability)

**Cons:**
- Requires compiler changes
- Must detect `self` usage in method body
- Must infer correct ownership mode

---

## 📋 Recommended Approach: Option 2 (Auto-Self Inference)

### Implementation Steps

#### 1. Analyzer Phase
In `src/analyzer.rs`:
```rust
// When analyzing function, check if 'self' is used in body
fn analyze_function(&mut self, func: &FunctionDecl) -> Result<AnalyzedFunction> {
    // NEW: Check if function uses 'self' but doesn't declare it
    let uses_self = self.function_uses_self(&func.body);
    let declares_self = func.parameters.iter().any(|p| p.name == "self");
    
    if uses_self && !declares_self {
        // Auto-inject self parameter
        let self_ownership = self.infer_self_ownership(&func.body)?;
        // ... add to inferred_parameters
    }
    // ... rest of analysis
}
```

#### 2. Codegen Phase
In `src/codegen/rust/generator.rs`:
```rust
// When generating function signature
fn generate_function_signature(&mut self, func: &FunctionDecl, analyzed: &AnalyzedFunction) {
    // Check if analyzer inferred self parameter
    if let Some(self_param) = analyzed.inferred_self {
        // Generate appropriate self: self, &self, or &mut self
        match self_param.ownership {
            OwnershipMode::Owned => write!("self"),
            OwnershipMode::Borrowed => write!("&self"),
            OwnershipMode::MutBorrowed => write!("&mut self"),
        }
    }
    // ... rest of parameters
}
```

#### 3. Helper Functions
```rust
fn function_uses_self(&self, body: &[Statement]) -> bool {
    // Recursively check if 'self' identifier appears in body
    for stmt in body {
        if self.statement_uses_self(stmt) {
            return true;
        }
    }
    false
}

fn infer_self_ownership(&self, body: &[Statement]) -> Result<OwnershipMode> {
    // Check if self is mutated
    if self.self_is_mutated(body) {
        return Ok(OwnershipMode::MutBorrowed); // &mut self
    }
    
    // Check if self is returned/moved
    if self.self_is_moved(body) {
        return Ok(OwnershipMode::Owned); // self
    }
    
    // Default: immutable borrow
    Ok(OwnershipMode::Borrowed) // &self
}
```

---

## 🎯 Implementation Plan

### Phase 1: Implement Auto-Self Inference (2-3 hours)
1. Add `function_uses_self()` to analyzer
2. Add `infer_self_ownership()` logic
3. Update `analyze_function()` to inject self
4. Update code generator to use inferred self
5. Write TDD tests for auto-self inference

### Phase 2: Rebuild windjammer-ui (30 min)
1. Rebuild Windjammer compiler with fix
2. Clean windjammer-ui generated code
3. Regenerate with new compiler
4. Verify all E0424 errors gone

### Phase 3: Fix Remaining Issues (1-2 hours)
1. Address any remaining compile errors
2. Fix Signal lifetime issues (E0310)
3. Test all components

### Phase 4: Build Editor (30 min)
1. Build windjammer-game-editor
2. Run editor
3. Verify all panels work

---

## 📊 Expected Outcome

**After Auto-Self Inference:**
- ✅ All E0424 errors fixed (~100 errors)
- ✅ windjammer-ui compiles successfully
- ✅ windjammer-game-editor builds
- ✅ Full dogfooding ecosystem working

**Bonus:**
- ✅ More ergonomic Windjammer code
- ✅ Compiler handles mechanical details
- ✅ Users focus on logic, not boilerplate

---

## 🚀 Alternative: Quick Fix (If Needed)

If we need windjammer-ui working **immediately**, we can:

1. **Manually fix the `.wj` files** (add explicit `self` parameters)
2. **Estimate:** 50 files × 5 methods/file × 1 min = ~4 hours
3. **Trade-off:** Works now, but not THE WINDJAMMER WAY

**Recommendation:** Spend 2-3 hours on compiler fix instead of 4 hours on manual fixes. Benefits:
- Scales to all future code
- Follows Windjammer philosophy
- Better long-term solution

---

## 📝 Next Actions

1. **Immediate:** Implement auto-self inference in analyzer
2. **Then:** Regenerate windjammer-ui
3. **Finally:** Build and test editor

**The proper fix benefits the entire ecosystem, not just this one package.**















