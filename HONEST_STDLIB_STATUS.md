# Honest Stdlib Status - v0.34.0

## Critical Finding
**The Windjammer stdlib is NOT functional for real-world use.**

While we have Rust implementations in `windjammer-runtime`, the Windjammer-to-Rust compiler has critical bugs that prevent stdlib usage.

## Compiler Bugs Blocking Stdlib Usage

### 1. **assert() Codegen Bug** 🔴 CRITICAL
- **Issue:** `assert(condition)` generates `assert(condition)` instead of `assert!(condition)`
- **Impact:** ALL assertions fail to compile
- **Status:** BROKEN

### 2. **String Literal Conversion** 🔴 CRITICAL  
- **Issue:** `"hello"` doesn't auto-convert to `String` when needed
- **Impact:** Cannot pass string literals to functions expecting `String`
- **Status:** BROKEN

### 3. **String Interpolation println!** 🔴 CRITICAL
- **Issue:** `print("text: ${var}")` generates `println!(format!("text: {}", var))` instead of `println!("text: {}", var)`
- **Impact:** Unnecessary double formatting
- **Status:** WORKS but inefficient

### 4. **Missing String Methods** 🔴 CRITICAL
- **Issue:** `.substring()` doesn't exist in Rust
- **Impact:** Cannot manipulate strings
- **Status:** BROKEN

### 5. **Function Parameter Borrowing** 🔴 CRITICAL
- **Issue:** Function signatures don't match (value vs reference)
- **Impact:** serve_fn and other callbacks fail
- **Status:** BROKEN

### 6. **MIME Module Privacy** 🟡 MEDIUM
- **Issue:** `mime::from_path()` is private
- **Impact:** Cannot determine MIME types from Windjammer
- **Status:** BROKEN (easy fix)

## Stdlib Module Status

| Module | Rust Impl | Windjammer Usable | Tests | Status |
|--------|-----------|-------------------|-------|--------|
| std::http | ✅ | ❌ | ❌ | BROKEN - compiler bugs |
| std::fs | ✅ | ❌ | ❌ | UNTESTED |
| std::json | ✅ | ❌ | ❌ | UNTESTED |
| std::mime | ✅ | ❌ | ❌ | BROKEN - private API |
| std::time | ✅ | ❌ | ❌ | UNTESTED |
| std::math | ✅ | ❌ | ❌ | UNTESTED |
| std::random | ✅ | ❌ | ❌ | UNTESTED |
| std::crypto | ✅ | ❌ | ❌ | UNTESTED |
| std::csv | ✅ | ❌ | ❌ | UNTESTED |
| std::db | ✅ | ❌ | ❌ | UNTESTED |
| std::log | ✅ | ❌ | ❌ | UNTESTED |
| std::regex | ✅ | ❌ | ❌ | UNTESTED |
| std::url | ✅ | ❌ | ❌ | UNTESTED |
| std::env | ✅ | ❌ | ❌ | UNTESTED |
| std::async_runtime | ✅ | ❌ | ❌ | UNTESTED |

## What Actually Works?

### ✅ Working:
- Basic Windjammer syntax (functions, variables, control flow)
- Compilation to Rust (with bugs)
- UI framework (WASM)
- Game framework (native)
- LSP server
- MCP server

### ❌ Broken:
- **ALL stdlib usage from Windjammer code**
- String manipulation
- HTTP servers in Windjammer
- File I/O from Windjammer
- Any real-world Windjammer programs

## Required Fixes (Priority Order)

### Phase 1: Critical Compiler Fixes (MUST DO NOW)
1. Fix `assert()` to generate `assert!()`
2. Auto-convert `&str` to `String` when needed
3. Add string slicing support (`.substring()` or `[start..end]`)
4. Fix function parameter borrowing inference

### Phase 2: Stdlib API Fixes
1. Make `mime::from_path()` public
2. Add `ServerResponse::html()` helper
3. Add `ServerResponse::with_headers()` builder
4. Review all stdlib APIs for Windjammer ergonomics

### Phase 3: Comprehensive Testing
1. Create Windjammer test for EVERY stdlib function
2. Ensure all tests compile AND run
3. Add integration tests for common patterns
4. Document what works and what doesn't

## Estimated Time to Fix
- **Phase 1:** 4-6 hours (critical compiler fixes)
- **Phase 2:** 2-3 hours (stdlib API improvements)
- **Phase 3:** 3-4 hours (comprehensive testing)
- **Total:** ~10-13 hours of focused work

## Recommendation
**DO NOT claim stdlib works until:**
1. All compiler bugs are fixed
2. Every stdlib module has passing Windjammer tests
3. We can write and run a real HTTP server in pure Windjammer
4. We can write and run file I/O examples in pure Windjammer

## User Impact
**Current state:** Users CANNOT write real Windjammer programs that use the stdlib. The language is essentially a toy until these issues are fixed.

**This is a showstopper for v0.34.0 release.**

