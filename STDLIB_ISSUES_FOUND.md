# Critical Stdlib/Compiler Issues Found

While testing the dev server example, I discovered several critical issues:

## 1. String Interpolation Codegen Bug
**Issue:** `println!(format!("text: {}", var))` instead of `println!("text: {}", var)`
**Location:** String interpolation codegen
**Impact:** Extra unnecessary format! call

## 2. Missing String Methods
**Issue:** `.substring()` doesn't exist in Rust
**Solution:** Need to add string slicing support or use `.chars().skip().take().collect()`
**Impact:** Any string manipulation code fails

## 3. MIME Module Issues
**Issue:** `mime::from_path()` is private in windjammer-runtime
**Solution:** Make it public or provide a public wrapper
**Impact:** Cannot determine MIME types

## 4. Missing HTTP Response Methods
**Issue:** `ServerResponse::not_found_html()` doesn't exist
**Available:** Only `not_found()` exists
**Impact:** Cannot create custom 404 responses easily

## 5. Function Signature Mismatch
**Issue:** `serve_fn` expects `Fn(&Request)` but we're generating `Fn(Request)`
**Impact:** Borrow checker errors

## Priority Fixes Needed:
1. Fix string interpolation codegen (HIGH)
2. Add string slicing/substring support (HIGH)
3. Make mime::from_path public (MEDIUM)
4. Add ServerResponse helper methods (LOW - can work around)
5. Fix serve_fn signature handling (HIGH)

**Conclusion:** Our stdlib is NOT production-ready. We need to fix these issues before claiming it works.

