/// Test: Copy type arguments should not be auto-referenced in method calls
///
/// Bug #8: Transpiler incorrectly adds & to Copy type arguments
///
/// Example:
/// ```
/// struct Stack {
///     quantity: i32,
/// }
///
/// impl Stack {
///     pub fn remove(&mut self, amount: i32) -> bool { ... }
/// }
///
/// let quantity = 5;
/// stack.remove(quantity);  // Should be: remove(quantity), NOT remove(&quantity)
/// ```
///
/// The transpiler was generating:
/// ```
/// stack.remove(&quantity)  ❌ WRONG (i32 is Copy, no & needed)
/// ```
///
/// This causes type mismatches when the method signature expects by-value Copy types.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_i32_method_arg_no_ref() {
    let source = r#"
struct Stack {
    quantity: i32,
}

impl Stack {
    pub fn remove(&mut self, amount: i32) -> bool {
        if self.quantity < amount {
            return false
        }
        self.quantity = self.quantity - amount
        true
    }
}

pub fn test() {
    let mut stack = Stack { quantity: 10 };
    let quantity = 5;
    stack.remove(quantity);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // Should NOT add & to Copy type argument
    assert!(
        generated.contains("stack.remove(quantity)"),
        "Expected 'stack.remove(quantity)' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("stack.remove(&quantity)"),
        "Should NOT generate 'stack.remove(&quantity)', found in:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_i32_method_arg_if_let() {
    let source = r#"
struct Stack {
    quantity: i32,
}

impl Stack {
    pub fn remove(&mut self, amount: i32) -> bool {
        if self.quantity < amount {
            return false
        }
        self.quantity = self.quantity - amount
        true
    }
}

pub fn test() {
    let mut opt_stack: Option<Stack> = Some(Stack { quantity: 10 });
    let quantity = 5;
    
    if let Some(stack) = &mut opt_stack {
        stack.remove(quantity);
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // Should NOT add & even in if let context
    assert!(
        generated.contains("stack.remove(quantity)"),
        "Expected 'stack.remove(quantity)' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("stack.remove(&quantity)"),
        "Should NOT generate 'stack.remove(&quantity)', found in:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_bool_method_arg_no_ref() {
    let source = r#"
struct Config {
    enabled: bool,
}

impl Config {
    pub fn set_enabled(&mut self, value: bool) {
        self.enabled = value;
    }
}

pub fn test() {
    let mut config = Config { enabled: false };
    let flag = true;
    config.set_enabled(flag);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // bool is Copy, should not add &
    assert!(
        generated.contains("config.set_enabled(flag)"),
        "Expected 'config.set_enabled(flag)' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("config.set_enabled(&flag)"),
        "Should NOT generate 'config.set_enabled(&flag)', found in:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_f32_method_arg_no_ref() {
    let source = r#"
struct Transform {
    x: f32,
}

impl Transform {
    pub fn set_x(&mut self, value: f32) {
        self.x = value;
    }
}

pub fn test() {
    let mut transform = Transform { x: 0.0 };
    let new_x = 10.5;
    transform.set_x(new_x);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    // f32 is Copy, should not add &
    assert!(
        generated.contains("transform.set_x(new_x)"),
        "Expected 'transform.set_x(new_x)' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("transform.set_x(&new_x)"),
        "Should NOT generate 'transform.set_x(&new_x)', found in:\n{}",
        generated
    );
}
