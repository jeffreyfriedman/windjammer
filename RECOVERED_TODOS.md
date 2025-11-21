# Recovered TODOs - Critical Work Items

**Date:** November 2, 2025  
**Source:** Session bootstrap + comprehensive codebase discovery  
**Priority:** These were in progress or planned before session corruption

---

## 🔴 P0 - Critical: World-Class Error Experience

**Goal:** Build compiler error experience on par with (and ideally exceeding) Rust.

### Source Map System
**Reference:** `docs/ERROR_MAPPING.md`, `docs/design/error-mapping.md`

1. **Phase 1: Source Map Generation** ❌
   - Add `source_map: SourceMap` field to `CodeGenerator`
   - Track mapping: `(rust_file, rust_line) → (wj_file, wj_line, wj_column)`
   - Record during code generation in `src/codegen/rust/generator.rs`
   - Write `source_map.json` to output directory
   
2. **Phase 2: Error Interception** ❌
   - Run `cargo build --message-format=json`
   - Parse JSON diagnostic messages from rustc
   - Extract file, line, column, message
   - Look up Windjammer location in source map
   
3. **Phase 3: Error Translation** ❌
   - Translate Rust terminology → Windjammer terminology
   - `cannot find type` → "Type not found"
   - `expected &str, found &String` → "Type mismatch (use string instead)"
   - `trait bounds not satisfied` → "Missing trait implementation"
   - `cannot move out of` → "Ownership error (variable was moved)"
   
4. **Phase 4: Contextual Help** ❌
   - Add Windjammer-specific suggestions
   - Example: "Use .parse() to convert a string to an integer"
   - Example: "This value is borrowed but goes out of scope before the borrow ends. Consider cloning."
   
5. **Phase 5: Pretty Print Errors** ❌
   - Colorized output with code snippets
   - Multi-line context
   - Inline suggestions
   ```
   Error in examples/hello/main.wj:12:5
     |
   12|     let x: int = "hello"
     |                  ^^^^^^^ Type mismatch: expected int, found string
     |
     = help: Use .parse() to convert a string to an integer
   ```

**Files to Create/Modify:**
- `src/source_map.rs` - Source map data structure and generation
- `src/error_translator.rs` - Rust → Windjammer error translation
- `src/codegen/rust/generator.rs` - Track mappings during codegen (TODO on line 19)
- `src/main.rs` - Intercept cargo output and translate errors

**Impact:** Immediate UX win, professional feel, faster debugging, reduced confusion

---

## 🔴 P0 - Critical Compiler Bugs (Blocking stdlib usage)

**Source:** `CRITICAL_FINDINGS_v0_34_0.md`

6. **Fix String Interpolation (nested)** 🔄
   - Currently generates: `println!(format!("Hello, {}", name))`
   - Should generate: `println!("Hello, {}", name)`
   - File: `src/codegen/rust/generator.rs` - expression generation

7. **Auto-convert &str to String** ❌
   - When function expects `String`, auto-convert `&str` literals
   - Add type coercion in analyzer/codegen
   - File: `src/analyzer.rs`, `src/codegen/rust/generator.rs`

8. **String Slicing Support** ❌
   - Implement `string[start..end]` or `.substring(start, end)`
   - File: `src/parser/expression_parser.rs` - slice syntax
   - File: `src/codegen/rust/generator.rs` - generate `&s[start..end]`

9. **Function Parameter Borrowing** ❌
   - Fix signature mismatches (value vs reference)
   - Improve analyzer inference
   - File: `src/analyzer.rs` - ownership inference

10. **Stdlib API Ergonomics** ❌
    - Make `mime::from_path()` public
    - Add missing `ServerResponse::not_found_html()`
    - Review all stdlib APIs
    - File: `crates/windjammer-runtime/src/`

---

## 🟠 P1 - Code Quality & Recommendations

**Goal:** Help developers write better code with compiler suggestions.

11. **Performance Recommendations** ❌
    - Detect inefficient patterns and suggest improvements
    - "This Vec is cloned in a loop - consider borrowing instead"
    - "This string is allocated many times - use String::with_capacity()"
    - "This struct has inefficient padding - reorder fields"
    
12. **Style Recommendations** ❌
    - Suggest idiomatic Windjammer patterns
    - "Consider using the pipe operator: value |> func1 |> func2"
    - "This closure can be simplified: .map(|x| x + 1)"
    
13. **Security Recommendations** ❌
    - Warn about potential security issues
    - "SQL query uses string interpolation - consider parameterized queries"
    - "User input is not validated before use"
    
14. **Optimize Suggestions in Linter** ❌
    - Enhance `wj lint` with actionable suggestions
    - Integrate with error system for inline suggestions
    - File: `src/cli/lint.rs`

**Implementation:**
- Add `RecommendationEngine` in `src/recommendations.rs`
- Integrate with analyzer to detect patterns
- Display recommendations in compiler output
- Add `--explain` flag to show detailed explanations

---

## 🟠 P1 - LSP Enhancements

**Source:** `crates/windjammer-lsp/src/server.rs` (TODOs on lines 1143-1157)

15. **Code Actions** ❌
    - Add missing import
    - Implement missing trait methods
    - Fix ownership annotations
    - File: `crates/windjammer-lsp/src/server.rs:1143`

16. **Document Formatting** ❌
    - Integrate with `wj fmt`
    - Format on save
    - File: `crates/windjammer-lsp/src/server.rs:1154`

17. **Quick Fixes** ❌
    - Auto-fix common errors
    - Apply suggested changes
    - Batch fixes

---

## 🟡 P2 - Compiler Optimizations

**Source:** `docs/OPTIMIZATION_ROADMAP.md`

18. **Phase 7: Lazy Static/Const** ❌
    - Convert runtime initialization to compile-time `const fn`
    - Detect static initialization patterns
    - File: `src/analyzer.rs` (TODO line 336-338)

19. **Phase 8: SmallVec Optimization** ❌
    - Use stack allocation for small vectors
    - Detect Vec usage patterns
    - Add `smallvec` crate dependency
    - File: `src/analyzer.rs` (TODO line 1388)

20. **Phase 9: Cow Optimization** ❌
    - Reduce allocations for rarely-modified data
    - Detect conditional modification patterns
    - Generate `Cow<'_, T>` where beneficial
    - File: `src/analyzer.rs` (TODO line 338)

21. **Phase 12: Loop Unrolling** ❌
    - Unroll small fixed-size loops
    - Detect fixed-size loops (range literals, const bounds)
    - Unroll loops with < 16 iterations

---

## 🟡 P2 - Language Features

**Source:** `docs/TODO.md`, `docs/PROGRESS.md`

22. **Local Variable Ownership Tracking** ❌
    - Track local variable ownership modes
    - Detect when locals are moved vs borrowed
    - Prevent use-after-move
    - File: `src/analyzer.rs` (TODO line 1243)

23. **Closure Capture Analysis** ❌
    - Detect what variables closures capture
    - Determine if capture is by value, ref, or mut ref
    - Generate `move` keyword when needed

24. **Automatic Trait Bound Inference** ❌
    - Analyze function bodies to infer required traits
    - `.clone()` → `Clone`, `println!("{:?}")` → `Debug`
    - File: `src/analyzer.rs` (TODO line 1319)
    - Reference: `docs/design/traits.md`

25. **Better Parser Error Messages** ❌
    - Line numbers and context in parse errors
    - Recovery hints
    - File: `src/parser_impl.rs` (TODO line 58)

---

## 🟡 P2 - Testing & Validation

26. **Comprehensive Stdlib Tests** 🔄
    - Write Windjammer tests for ALL stdlib modules
    - Create `std/string_test.wj`, `std/fs_test.wj`, etc.
    - Run with `wj test` and document results
    - **Status:** Test framework implemented, tests not written

27. **Systematic Example Testing** ❌
    - Test all 122 examples
    - Document which work and which don't
    - Fix broken examples
    - Update `docs/EXAMPLE_STATUS.md`

28. **Performance Benchmarks** ❌
    - Compare to Rust and Go
    - Web server throughput, JSON parsing, concurrency
    - Document in `BENCHMARKS.md`
    - Reference: `docs/PROGRESS.md:239`

---

## 🟢 P3 - Documentation & Cleanup

29. **Remove Python References** ❌
    - Replace `python3 -m http.server` with Windjammer HTTP server
    - Update all documentation
    - Files: `docs/`, `examples/*/README.md`

30. **Rust-style Doctests** ❌
    - Parse `///` doc comments
    - Extract code blocks
    - Generate test functions
    - Integrate with `wj test`
    - Reference: `docs/ROADMAP.md:403`

31. **Complete Documentation** ❌
    - All stdlib modules need docs
    - All language features need examples
    - Update `GUIDE.md` with latest features

---

## 🔵 P4 - Advanced Features (v1.0+)

32. **Package Manager** ❌
    - `wj add`, `wj update`, `wj publish`
    - Dependency resolution
    - Reference: `docs/ROADMAP.md:481`

33. **Debugger Integration** ❌
    - Stack trace mapping (Rust → Windjammer)
    - LLDB/GDB integration
    - Breakpoint mapping

34. **IDE Quick Fixes** ❌
    - LSP code action improvements
    - Rename refactoring
    - Extract function
    - Reference: `docs/ROADMAP.md:470`

---

## Summary

**Total TODOs Recovered:** 34

**By Priority:**
- **P0 (Critical):** 10 items - Error experience + Compiler bugs
- **P1 (Important):** 7 items - Code recommendations + LSP
- **P2 (Standard):** 9 items - Optimizations + Language features + Testing
- **P3 (Nice to Have):** 3 items - Documentation
- **P4 (Future):** 3 items - Advanced features

**Immediate Next Steps:**
1. Start with error system (highest impact on DX)
2. Fix critical compiler bugs (unblock stdlib)
3. Write comprehensive tests (validate the language works)
4. Add code recommendations (help users write better code)
5. Implement optimizations (performance improvements)

---

**Last Updated:** November 2, 2025  
**Session:** Post-corruption recovery  
**Status:** Ready to execute

