# Error System - Remaining Work & Advanced Capabilities

**Status**: Core implementation complete (Phases 1-5 ✅)  
**Date**: November 8, 2025

---

## 🎯 HIGH PRIORITY - Integration & Polish

### 1. **Full Pipeline Integration** ⚠️ CRITICAL
**Status**: Partially implemented  
**Priority**: P0  
**Effort**: 4-6 hours

**Current State**:
- `check_with_cargo()` exists but is marked `#[allow(dead_code)]`
- Parses rustc JSON but doesn't use ErrorMapper
- No source map loading

**TODO**:
- [ ] Load source map in `check_with_cargo()`
- [ ] Use `ErrorMapper::map_rustc_output()` instead of raw parsing
- [ ] Display `WindjammerDiagnostic::format()` output
- [ ] Add colored output support
- [ ] Make `check_with_cargo()` the default in `wj build`
- [ ] Add `--raw-errors` flag to show Rust errors (for debugging)

**Code Location**: `src/main.rs:1306-1400`

### 2. **End-to-End Testing** ⚠️ CRITICAL
**Status**: Not started  
**Priority**: P0  
**Effort**: 6-8 hours

**TODO**:
- [ ] Create test Windjammer files with intentional errors
- [ ] Verify source map generation
- [ ] Verify error mapping accuracy
- [ ] Test all translation patterns
- [ ] Test contextual help suggestions
- [ ] Add integration tests to CI

**Test Cases Needed**:
```windjammer
// test_type_mismatch.wj
fn main() {
    let x: int = "hello"  // Should show: Type mismatch: expected int, found string
}

// test_ownership.wj
fn main() {
    let s = "hello".to_string()
    let s2 = s
    println!("{}", s)  // Should show: Ownership error with helpful suggestion
}

// test_function_not_found.wj
fn main() {
    foo()  // Should show: Function not found: foo
}
```

### 3. **Color Support** 🎨
**Status**: Partially implemented (in check_with_cargo)  
**Priority**: P1  
**Effort**: 2-3 hours

**TODO**:
- [ ] Add colored output to `WindjammerDiagnostic::format()`
- [ ] Red for errors, yellow for warnings
- [ ] Blue for file paths
- [ ] Cyan for line numbers
- [ ] Bold for important text
- [ ] Add `--no-color` flag

**Dependencies**: `colored` crate (already in use)

---

## 🚀 ADVANCED CAPABILITIES

### 4. **Multi-File Error Tracking** 🔄
**Status**: Not started  
**Priority**: P1  
**Effort**: 8-10 hours

**Goal**: Track errors across multiple `.wj` files in a project

**TODO**:
- [ ] Load all `.rs.map` files in output directory
- [ ] Merge source maps for multi-file projects
- [ ] Handle cross-file errors (e.g., import errors)
- [ ] Show related errors in different files
- [ ] Add "also referenced in" notes

**Example**:
```
error: Function not found: calculate_tax
  --> src/billing.wj:42:5
   |
42 |     calculate_tax(amount)
   |     ^^^^^^^^^^^^^ not found in this scope
   |
   = note: Did you mean 'compute_tax' from src/tax.wj:15?
```

### 5. **Fix Suggestions** 💡
**Status**: Not started  
**Priority**: P1  
**Effort**: 10-12 hours

**Goal**: Provide machine-applicable fixes (like Rust's `rustfix`)

**TODO**:
- [ ] Detect fixable errors (typos, missing imports, etc.)
- [ ] Generate suggested fixes
- [ ] Add `--fix` flag to apply fixes automatically
- [ ] Show diff of proposed changes
- [ ] Require user confirmation for each fix

**Example**:
```
error: Function not found: prnt
  --> main.wj:10:5
   |
10 |     prnt("Hello")
   |     ^^^^ not found
   |
   = help: Did you mean 'print'?
   = suggestion: Run `wj build --fix` to apply this fix
```

### 6. **Error Explanations** 📚
**Status**: Not started  
**Priority**: P2  
**Effort**: 20-30 hours

**Goal**: Detailed explanations for each error type (like `rustc --explain`)

**TODO**:
- [ ] Create error code system (WJ0001, WJ0002, etc.)
- [ ] Write detailed explanations for each error
- [ ] Add examples of correct code
- [ ] Add `wj explain WJ0001` command
- [ ] Link to online documentation

**Example**:
```
error[WJ0308]: Type mismatch
  --> main.wj:10:18
   |
10 |     let x: int = "hello"
   |                  ^^^^^^^ expected int, found string
   |
   = help: Run `wj explain WJ0308` for more information
```

### 7. **LSP Integration** 🔌
**Status**: Not started  
**Priority**: P2  
**Effort**: 40-60 hours

**Goal**: Real-time error checking in editors

**TODO**:
- [ ] Implement Language Server Protocol
- [ ] Provide diagnostics as you type
- [ ] Show errors inline in editor
- [ ] Provide quick fixes
- [ ] Add hover information
- [ ] Integrate with VS Code, Vim, etc.

### 8. **Error Recovery & Suggestions** 🔧
**Status**: Not started  
**Priority**: P2  
**Effort**: 15-20 hours

**Goal**: Smarter error recovery and suggestions

**TODO**:
- [ ] Fuzzy matching for typos (Levenshtein distance)
- [ ] Suggest similar function/variable names
- [ ] Detect common patterns (e.g., missing semicolons)
- [ ] Provide context-aware suggestions
- [ ] Learn from user's codebase

**Example**:
```
error: Function not found: calcualte_total
  --> main.wj:10:5
   |
10 |     calcualte_total(items)
   |     ^^^^^^^^^^^^^^^ not found
   |
   = help: Did you mean one of these?
     - calculate_total (defined in main.wj:5)
     - calculate_subtotal (defined in billing.wj:12)
```

### 9. **Performance Optimization** ⚡
**Status**: Not started  
**Priority**: P3  
**Effort**: 8-10 hours

**Goal**: Fast error checking even for large projects

**TODO**:
- [ ] Cache source maps
- [ ] Incremental error checking
- [ ] Parallel error processing
- [ ] Optimize source map lookup (use HashMap)
- [ ] Profile and optimize hot paths

### 10. **Error Statistics & Analytics** 📊
**Status**: Not started  
**Priority**: P3  
**Effort**: 6-8 hours

**Goal**: Help users understand their error patterns

**TODO**:
- [ ] Track error frequency
- [ ] Show most common errors
- [ ] Provide learning resources for common mistakes
- [ ] Add `wj stats` command
- [ ] Generate error reports

**Example**:
```
$ wj stats
Error Statistics (last 7 days):
  - Type mismatches: 42 (35%)
  - Ownership errors: 28 (23%)
  - Function not found: 18 (15%)
  
Most common error:
  Type mismatch: expected int, found string (12 occurrences)
  → Tip: Use .parse() to convert strings to integers
```

---

## 🎨 POLISH & UX IMPROVEMENTS

### 11. **Better Source Snippets** 📝
**Status**: Basic implementation  
**Priority**: P2  
**Effort**: 4-6 hours

**TODO**:
- [ ] Show more context (configurable lines before/after)
- [ ] Syntax highlighting in error output
- [ ] Multi-line error spans
- [ ] Show multiple related locations
- [ ] Add line continuation indicators

### 12. **Interactive Error Mode** 🖱️
**Status**: Not started  
**Priority**: P3  
**Effort**: 10-15 hours

**Goal**: Interactive TUI for exploring errors

**TODO**:
- [ ] Build TUI with `ratatui` or similar
- [ ] Navigate between errors with arrow keys
- [ ] Expand/collapse error details
- [ ] Jump to error location in editor
- [ ] Apply fixes interactively

### 13. **Error Filtering & Grouping** 🔍
**Status**: Not started  
**Priority**: P2  
**Effort**: 4-6 hours

**TODO**:
- [ ] Group related errors
- [ ] Filter by error type
- [ ] Filter by file/module
- [ ] Show only first N errors
- [ ] Add `--verbose` for all errors

### 14. **Documentation Generation** 📖
**Status**: Not started  
**Priority**: P2  
**Effort**: 8-10 hours

**TODO**:
- [ ] Generate error catalog documentation
- [ ] Create searchable error database
- [ ] Add to website/docs
- [ ] Include examples for each error
- [ ] Add troubleshooting guides

---

## 📋 SUMMARY

**Completed** (Phases 1-5):
- ✅ Source location tracking
- ✅ Source map generation
- ✅ Error interception infrastructure
- ✅ Message translation (10+ patterns)
- ✅ Pretty-printing with source snippets
- ✅ Contextual help (6+ suggestion patterns)
- ✅ Comprehensive test suite (12 tests)

**High Priority** (Next 2-4 weeks):
1. Full pipeline integration (P0)
2. End-to-end testing (P0)
3. Color support (P1)
4. Multi-file error tracking (P1)
5. Fix suggestions (P1)

**Medium Priority** (Next 1-3 months):
6. Error explanations (P2)
7. LSP integration (P2)
8. Error recovery & suggestions (P2)
9. Better source snippets (P2)
10. Error filtering & grouping (P2)

**Low Priority** (Future):
11. Performance optimization (P3)
12. Error statistics (P3)
13. Interactive error mode (P3)
14. Documentation generation (P3)

---

## 🎯 RECOMMENDED NEXT STEPS

**Immediate (This Week)**:
1. Integrate ErrorMapper into build pipeline
2. Test with real examples (wjfind, http_server)
3. Add colored output
4. Create integration test suite

**Short Term (Next Month)**:
5. Multi-file error tracking
6. Fix suggestions
7. Error explanations

**Long Term (Next Quarter)**:
8. LSP integration
9. Interactive error mode
10. Performance optimization

---

**The foundation is solid. Now it's time to polish and extend!** 🚀

