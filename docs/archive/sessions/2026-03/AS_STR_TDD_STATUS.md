# `.as_str()` Prohibition - TDD Status

## ✅ COMPLETED: TDD Test (Analyzer Logic)

**Test File:** `tests/forbidden_as_str_test.rs`

**Status:** ✅ **PASSING** (100% success rate)

### Tests:
1. `test_as_str_is_forbidden` - Correctly rejects `.as_str()` with helpful error
2. `test_string_match_without_as_str_is_allowed` - Correctly allows idiomatic `match name { ... }`

### Implementation:
- **File:** `src/analyzer/mod.rs`
- **Function:** `check_forbidden_rust_patterns(&self, program: &Program) -> Result<(), String>`
- **Coverage:** Checks all AST nodes recursively:
  - `Expression::MethodCall` with `method == "as_str"`
  - `Expression::Call` where function is `FieldAccess { field: "as_str" }`
  - All statements (`If`, `Match`, `For`, `While`, `Assignment`, `Return`, etc.)
  - All items (`Function`, `Impl`, `Trait`, `Const`, `Static`, `Mod`)

**Error Message:**
```
error: `.as_str()` is forbidden in Windjammer source

Windjammer automatically handles string conversions based on context.
You don't need to call `.as_str()` - the compiler will generate the
correct Rust code automatically.

Example:
❌ match name.as_str() { ... }  // Don't do this
✅ match name { ... }            // Do this instead

This keeps Windjammer code clean and backend-agnostic (Go/JS/etc
don't have .as_str()).
```

## ⚠️ IN PROGRESS: CLI Integration

**Problem:** The analyzer check works in unit tests but doesn't run during `wj build` CLI compilation.

**Evidence:**
- TDD test: ✅ PASSES (analyzer correctly detects `.as_str()`)
- CLI build: ❌ SUCCEEDS (should fail, but doesn't - check isn't running)

**Debug Findings:**
1. Check added to `analyzer::Analyzer::analyze_program()` (line 423)
2. Check added to `analyzer::Analyzer::analyze_program_with_global_signatures()` (line 687)
3. Check added to `ModuleCompiler::compile_module()` (line 1089)
4. Check added to `compile_file_impl()` (line 1624)
5. **BUT:** Debug statements (`eprintln!`) never appear in CLI output
6. **PROOF:** Added `panic!()` to `compile_file_impl` - no panic occurred
7. **CONCLUSION:** `compile_file_impl` is NOT being called during single-file builds

**Hypothesis:**
- Single-file builds may use a different code path (ejector? direct codegen?)
- OR: There's caching/early-return logic that bypasses `compile_file_impl`
- OR: The compilation happens in a subprocess that doesn't inherit stdio

**Next Steps:**
1. Trace the ACTUAL code path for single-file `wj build`
2. Find where `TDD DEBUG MATCH START` is printed (it IS appearing)
3. Add check BEFORE that codegen happens
4. Verify with CLI test that error is raised

## 📋 Files Modified

### Analyzer
- `src/analyzer/mod.rs` (+150 lines)
  - Added `check_forbidden_rust_patterns()` method
  - Integrated into `analyze_program()` and `analyze_program_with_global_signatures()`

### Tests
- `tests/forbidden_as_str_test.rs` (new file, +55 lines)
  - Unit test for analyzer logic
  - **STATUS: ✅ PASSING**

### Main Compiler
- `src/main.rs` (multiple integration points)
  - Added checks in `compile_module()`, `compile_file_impl()`
  - **STATUS:** ⚠️ Not executing during CLI builds (needs investigation)

## 🎯 Language Design Decision

**DECISION:** Prohibit `.as_str()` in Windjammer source (hard error)

**Rationale:**
1. Cross-backend consistency (Go/JS/Interpreter don't have `.as_str()`)
2. Compiler handles string conversions automatically
3. Simpler mental model (`string` works directly in `match`)
4. Aligns with "Inference when it doesn't matter" philosophy
5. Prevents user confusion about when to use it

**See:** `STRING_AS_STR_LANGUAGE_DESIGN.md` for full analysis

## ✅ TDD Methodology

**The Windjammer Way:**
1. ✅ Write failing test first (`test_as_str_is_forbidden`)
2. ✅ Implement check in analyzer
3. ✅ Test passes
4. ⚠️ Integrate into compiler (IN PROGRESS - CLI path needs debugging)
5. 🔄 Verify with real game code
6. 📝 Document language decision

**This is TDD done right:** Test works, logic is correct, integration needs refinement.

## 🚀 Impact

**Game Code Changes Needed:**
- `windjammer-game/windjammer-game-core/src_wj/rpg/character_stats.wj`
  - Removed `.as_str()` from `new_with_build()` function
  - **Now:** `match build_type { ... }` (idiomatic Windjammer)

**Compiler Changes:**
- Statement generation already auto-adds `.as_str()` when needed (Rust backend)
- Analyzer now catches explicit `.as_str()` calls (once CLI integration works)

**Future:**
- All `.as_str()` calls in game code will be removed
- Language will enforce this via compiler error
- Other backends (Go/JS/Interpreter) will continue to work without changes
