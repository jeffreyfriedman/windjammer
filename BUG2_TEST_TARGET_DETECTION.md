# Bug #2: Test Target Detection - TDD Success

## Date: 2026-02-22

## Problem

The compiler generated Cargo.toml without proper [[bin]] or [[test]] target sections, treating all files identically regardless of whether they were:
- Test files (containing `#[test]` functions)
- Executables (containing `fn main()`)
- Library modules (neither)

This caused test files to be incorrectly treated as binaries, and library files to generate unnecessary targets.

## Solution

**TDD Approach:** RED → GREEN → REFACTOR

### RED Phase
Created `tests/bug_test_target_detection.rs` with 4 tests:
1. `test_file_with_test_functions_generates_test_target` - Files with `#[test]` → `[[test]]`
2. `test_executable_file_generates_bin_target` - Files with `main()` → `[[bin]]`
3. `test_mixed_file_with_main_and_tests_generates_bin_target` - Mixed files → `[[bin]]` (main takes precedence)
4. `test_library_file_generates_no_target` - Library code → no target

**Initial Results:** 1 passing, 3 failing ❌

### GREEN Phase
Modified `src/main.rs` to add file type detection:

**1. Added RustFileType enum:**
```rust
enum RustFileType {
    Test,    // Contains #[test] functions
    Binary,  // Contains fn main()
    Library, // Neither (just library code)
}
```

**2. Added detect_rust_file_type function:**
```rust
fn detect_rust_file_type(path: &Path) -> RustFileType {
    if let Ok(contents) = std::fs::read_to_string(path) {
        let has_main = contents.contains("fn main()") || contents.contains("fn main(");
        let has_test = contents.contains("#[test]");
        
        // Priority: main() takes precedence (binaries can have tests)
        // Files with ONLY tests (no main) are test targets
        // Files with neither are library modules (no target needed)
        if has_main {
            RustFileType::Binary
        } else if has_test {
            RustFileType::Test
        } else {
            RustFileType::Library
        }
    } else {
        RustFileType::Library
    }
}
```

**3. Modified target generation logic:**
```rust
// Detect file type and generate appropriate target
match file_type {
    RustFileType::Test => {
        // Generate [[test]] target
        target_sections.push(format!(
            "[[test]]\nname = \"{}\"\npath = \"{}\"\n",
            target_name, filename
        ));
    }
    RustFileType::Binary => {
        // Generate [[bin]] target
        target_sections.push(format!(
            "[[bin]]\nname = \"{}\"\npath = \"{}\"\n",
            target_name, filename
        ));
    }
    RustFileType::Library => {
        // No target needed (just a module)
    }
}
```

### REFACTOR Phase
- No refactoring needed - implementation is clean and minimal
- All 239 existing unit tests still pass ✅
- All 4 new Bug #2 tests pass ✅

## Test Results

```
running 4 tests
test test_file_with_test_functions_generates_test_target ... ok
test test_executable_file_generates_bin_target ... ok
test test_mixed_file_with_main_and_tests_generates_bin_target ... ok
test test_library_file_generates_no_target ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Full test suite:
```
test result: ok. 239 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Generated Cargo.toml Examples

**Test File (my_tests.rs with #[test]):**
```toml
[[test]]
name = "my_tests"
path = "my_tests.rs"
```

**Executable (my_game.rs with main()):**
```toml
[[bin]]
name = "my_game"
path = "my_game.rs"
```

**Mixed File (my_app.rs with main() + #[test]):**
```toml
[[bin]]
name = "my_app"
path = "my_app.rs"
```
_(main() takes precedence - binaries can have tests)_

**Library File (math.rs, no main, no tests):**
```toml
# No target section generated
```
_(Library modules don't need targets - they're just code)_

## Implementation Details

**Detection Logic:**
1. Scan generated .rs files in build directory
2. Check for `fn main()` → Binary
3. Check for `#[test]` → Test
4. Neither → Library (no target)

**Priority Rules:**
- **main() wins** - Executables can have tests alongside main()
- **#[test] only** - Pure test files get [[test]] targets
- **Neither** - Library code gets no target (imported by others)

## Files Changed

- `src/main.rs` - Added file type detection and target generation logic
- `tests/bug_test_target_detection.rs` - 4 new TDD tests

## Correctness

This solution correctly handles:
- ✅ **Test files** - Generate `[[test]]` for cargo test
- ✅ **Executables** - Generate `[[bin]]` for cargo run
- ✅ **Mixed files** - Treat as executables (tests run with cargo test)
- ✅ **Library modules** - No target (imported by other files)

## TDD Methodology Validated

✅ **Write tests first** - All 4 tests written before fix  
✅ **See them fail (RED)** - 3/4 failing initially  
✅ **Make them pass (GREEN)** - All 4 passing after implementation  
✅ **Refactor** - Code is clean, no refactoring needed  
✅ **No regressions** - All 239 existing tests still pass  

## Next Steps

1. ✅ Bug #2 COMPLETE
2. Continue dogfooding: compile windjammer-game
3. Add `#[test]` attribute support to parser (future enhancement)
4. Fix remaining compiler issues as discovered

---

**"If it's worth doing, it's worth doing right."** ✅ DONE RIGHT.
