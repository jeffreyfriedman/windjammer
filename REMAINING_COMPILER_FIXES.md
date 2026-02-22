# Remaining Compiler Fixes (Bugs #2 and #3)

**Date:** February 20, 2026  
**Status:** Bug #1 FIXED ✅ | Bugs #2 and #3 require additional work

## ✅ Bug #1: Dependency Tracking - FIXED!

**Proper fix implemented with TDD:**
- Scans generated code for `use` statements
- Extracts external crates automatically
- Generates Cargo.toml with correct dependencies
- Tests passing, committed, pushed

**No workarounds!** Clean solution that works automatically.

---

## ❌ Bug #2: Test File Detection

### Problem
Test files with `#[test]` functions are declared as `[[bin]]` targets in Cargo.toml instead of `[[test]]` targets. This causes "main function not found" errors.

### Root Cause
The compiler doesn't distinguish between:
- **Executable files** (games, demos) → should be `[[bin]]`
- **Test files** (unit tests, integration tests) → should be `[[test]]`

### Current Behavior
```toml
[[bin]]
name = "minimal_test"
path = "minimal_test.rs"
```

### Expected Behavior
```toml
[[test]]
name = "minimal_test"
path = "minimal_test.rs"
```

### Proper Fix (TDD Approach)

#### Step 1: Write Failing Tests (RED)
Add to `src/codegen/rust/backend.rs`:

```rust
#[test]
fn test_detect_test_file() {
    let code_with_tests = r#"
#[test]
fn test_something() {
    assert_eq!(1, 1);
}

#[test]
fn test_another() {
    assert!(true);
}
"#;
    
    assert!(RustBackend::is_test_file(code_with_tests), 
            "Should detect code with #[test] as a test file");
    
    let code_without_tests = r#"
fn main() {
    println!("Hello");
}
"#;
    
    assert!(!RustBackend::is_test_file(code_without_tests), 
            "Should NOT detect executable as test file");
}

#[test]
fn test_generate_test_target_in_cargo_toml() {
    let backend = RustBackend::new();
    let test_code = r#"
#[test]
fn test_basic() {
    assert_eq!(2 + 2, 4);
}
"#;
    
    let cargo_entry = backend.generate_cargo_target("my_test", "my_test.rs", test_code);
    
    assert!(cargo_entry.contains("[[test]]"), 
            "Should generate [[test]] target for test files");
    assert!(cargo_entry.contains("name = \"my_test\""), 
            "Should include test name");
    assert!(cargo_entry.contains("path = \"my_test.rs\""), 
            "Should include test path");
}
```

#### Step 2: Implement Detection (GREEN)
```rust
impl RustBackend {
    /// Detect if generated code is a test file (contains #[test] functions)
    pub fn is_test_file(code: &str) -> bool {
        // Look for #[test] attribute on functions
        code.contains("#[test]")
    }
    
    /// Generate appropriate Cargo.toml target ([[bin]] or [[test]])
    pub fn generate_cargo_target(
        &self,
        name: &str,
        path: &str,
        code: &str
    ) -> String {
        if Self::is_test_file(code) {
            format!(
                "[[test]]\nname = \"{}\"\npath = \"{}\"\n\n",
                name, path
            )
        } else {
            format!(
                "[[bin]]\nname = \"{}\"\npath = \"{}\"\n\n",
                name, path
            )
        }
    }
}
```

#### Step 3: Integrate into Build Pipeline
Update where Cargo.toml targets are generated (likely in `wj build` command):

```rust
// In the build command handler
for file in source_files {
    let code = generate_rust_code(&file);
    let name = file.stem().unwrap().to_str().unwrap();
    let path = format!("{}.rs", name);
    
    // NEW: Detect file type and generate appropriate target
    let target = backend.generate_cargo_target(name, &path, &code);
    cargo_toml.push_str(&target);
}
```

#### Step 4: Verify
```bash
cargo test --lib codegen::rust::backend::tests
wj build examples/minimal_test.wj
cargo test --manifest-path build/Cargo.toml
```

---

## ❌ Bug #3: String/&str Coercion in format!()

### Problem
The compiler generates code that passes `format!()` (which returns `String`) directly to FFI functions expecting `&str`, causing type mismatches.

### Root Cause
In Windjammer source:
```windjammer
ctx.draw_text(format!("Score: {}", score), ...)
```

Generated Rust:
```rust
draw_text(format!("Score: {}", score), ...)  // ERROR: expected &str, found String
```

### Proper Fix (TDD Approach)

#### Step 1: Write Failing Tests (RED)
Add to appropriate codegen test file:

```rust
#[test]
fn test_format_as_function_argument_extracts_to_variable() {
    let windjammer_code = r#"
fn render(score: i32) {
    draw_text(format!("Score: {}", score), 10.0, 10.0);
}
"#;
    
    let rust_code = compile_to_rust(windjammer_code);
    
    // Should extract format!() to a variable
    assert!(rust_code.contains("let"), 
            "Should use variable for format! result");
    assert!(rust_code.contains("score_text") || rust_code.contains("_temp"), 
            "Should have generated variable name");
    assert!(rust_code.contains("&"), 
            "Should pass reference to variable");
    
    // Should NOT pass format!() directly
    assert!(!rust_code.contains("draw_text(format!("), 
            "Should NOT pass format!() directly as argument");
}

#[test]
fn test_format_as_variable_assignment_no_change() {
    let windjammer_code = r#"
fn test() {
    let msg = format!("Hello {}", name);
    draw_text(msg, 10.0, 10.0);
}
"#;
    
    let rust_code = compile_to_rust(windjammer_code);
    
    // When already assigned to variable, no changes needed
    assert!(rust_code.contains("let msg = format!"), 
            "Should keep variable assignment");
    assert!(rust_code.contains("draw_text(msg,") || rust_code.contains("draw_text(&msg,"), 
            "Should pass variable (with or without &)");
}
```

#### Step 2: Implement Transform (GREEN)

Find where function call arguments are generated (likely in `src/codegen/rust/expressions.rs`):

```rust
// In function call generation
fn generate_function_call(&mut self, func: &FunctionCall) -> String {
    let func_name = &func.name;
    let mut args = Vec::new();
    let mut temp_vars = Vec::new();
    
    for (i, arg) in func.arguments.iter().enumerate() {
        // Check if argument is a format! macro call
        if self.is_format_macro(arg) {
            // Extract to temporary variable
            let temp_name = format!("_temp_{}", i);
            let format_expr = self.generate_expression(arg);
            temp_vars.push(format!("let {} = {};", temp_name, format_expr));
            
            // Pass reference to temp variable
            args.push(format!("&{}", temp_name));
        } else {
            // Normal argument handling
            args.push(self.generate_expression(arg));
        }
    }
    
    // Generate temp variables before the call
    let mut result = String::new();
    for temp_var in temp_vars {
        result.push_str(&format!("{};\n", temp_var));
    }
    
    // Generate the function call
    result.push_str(&format!("{}({})", func_name, args.join(", ")));
    result
}

fn is_format_macro(&self, expr: &Expression) -> bool {
    // Check if expression is a format! macro call
    matches!(expr, Expression::MacroCall(m) if m.name == "format")
}
```

#### Step 3: Handle Edge Cases
- format! inside complex expressions
- Multiple format! calls in same function
- format! with method calls (e.g., `format!(...).as_str()`)
- Nested function calls with format!

#### Step 4: Verify
```bash
cargo test --lib codegen::rust::expressions::tests
wj build examples/breakout.wj
cargo check --manifest-path build/Cargo.toml
```

---

## Implementation Priority

1. **Bug #3 (String/&str)** - HIGHEST
   - Blocks ALL game execution
   - Most visible impact
   - Affects every game with UI text

2. **Bug #2 (Test detection)** - MEDIUM
   - Blocks test execution
   - Less critical than games
   - Can run tests manually as workaround

3. **Performance optimizations** - AFTER bugs fixed
   - Current 64.9 FPS @ 1M is excellent
   - Focus on correctness first

---

## Test Coverage Requirements

Each bug fix MUST include:
- ✅ RED phase: Failing test demonstrating the bug
- ✅ GREEN phase: Minimal implementation to pass the test
- ✅ REFACTOR: Clean up and optimize
- ✅ Integration test: Compile real game code
- ✅ No regressions: All existing tests pass

---

## Success Criteria

### Bug #2 Fixed
```bash
wj build tests/minimal_test.wj
cd build && cargo test minimal_test
# Test should run without "main function not found" error
```

### Bug #3 Fixed
```bash
wj build examples/breakout.wj
cd build && cargo build --bin breakout
# Should compile without String/&str type errors
./target/debug/breakout
# Game should run!
```

---

## Next Steps

1. Fix Bug #3 (String/&str) - highest impact
2. Fix Bug #2 (test detection) - completeness
3. Run full dogfooding validation
4. Performance optimization (100+ FPS target)

**Remember:** NO WORKAROUNDS! Only proper fixes with TDD.
