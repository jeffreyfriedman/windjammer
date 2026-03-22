# E0658 Investigation Summary

## The Bug
Generated Rust calls `.as_str()` on `&str` parameters, causing:
```rust
error[E0658]: use of unstable library feature `str_as_str`
```

## Root Cause Discovery

**Original Hypothesis**: Codegen adds `.as_str()` when it sees string literals in match arms.

**Actual Cause**: The `.as_str()` is **already in the Windjammer source**!
- Windjammer: `match build_type.as_str() { "warrior" => ... }`  
- Parser preserves it as: `MethodCall { object: Identifier("build_type"), method: "as_str" }`
- Codegen generates it as-is: `match build_type.as_str() { ... }`
- But `build_type` is inferred as `&str`, so calling `.as_str()` on it is redundant

## Why The Test Passes But Game Fails

**Test scenario**: Single-file compilation of one function
- Goes through expression generation
- MethodCall detection works
- Fix applied successfully ✅

**Game scenario**: Multi-file compilation  
- SAME issue - `.as_str()` in source
- But the match statement isn't going through `generate_match_statement` debug path
- Expression is generated directly somewhere else
- Fix not reaching the right code path ❌

## The Real Fix Needed

Strip `.as_str()` in **expression generation**, specifically when generating a `MethodCall` expression where:
1. `method == "as_str"`
2. `arguments.is_empty()`
3. `object` is `Identifier` in `inferred_borrowed_params`

Then generate just `object`, not the full method call.

## Next Steps
1. Find where match value expressions are generated
2. Add stripping logic there (not in match statement generation)
3. Verify both test and game scenarios
4. Remove all debug prints
5. Commit final fix

## Files Affected
- `windjammer/src/codegen/rust/expression_generation.rs` (likely needs fix)
- `windjammer/src/codegen/rust/statement_generation.rs` (current debug code)
- `windjammer/tests/bug_redundant_as_str_test.rs` (TDD test - passes)
