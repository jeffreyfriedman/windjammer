/// TDD: Method Call Argument Borrow Inconsistency Bug
///
/// Bug: When calling a method with multiple f32 arguments from local variables,
/// the compiler inconsistently adds `&` to some arguments but not others.
///
/// Example:
///   Windjammer: particle.update(delta, gx, gy)
///   Generated:  particle.update(delta, &gx, gy)  // WRONG! Inconsistent borrowing
///
/// Expected:   particle.update(delta, gx, gy)     // All Copy types, no borrowing
use std::fs;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_with_multiple_f32_args_from_locals() {
    let source = r#"
struct Particle {
    x: f32,
    y: f32,
}

impl Particle {
    fn update(&mut self, delta: f32, gx: f32, gy: f32) {
        self.x += delta
        self.y += gx + gy
    }
}

fn main() {
    let mut p = Particle { x: 0.0, y: 0.0 }
    let delta = 0.1
    let gx = 1.0
    let gy = 2.0
    p.update(delta, gx, gy)
}
"#;

    fs::write("test_borrow_inconsistency.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_borrow_inconsistency.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(output.status.success(), "wj build should succeed");

    let rust_code = fs::read_to_string("./build/test_borrow_inconsistency.rs")
        .expect("Failed to read generated Rust code");

    // Should NOT add `&` to any f32 arguments (they are Copy)
    assert!(
        !rust_code.contains("&gx"),
        "Should not borrow gx (it's Copy type f32): \n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("&gy"),
        "Should not borrow gy (it's Copy type f32): \n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("&delta"),
        "Should not borrow delta (it's Copy type f32): \n{}",
        rust_code
    );

    // Cleanup
    fs::remove_file("test_borrow_inconsistency.wj").ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_copy_type_no_borrow() {
    let source = r#"
struct Test {
    value: i32,
}

impl Test {
    fn add(&mut self, a: i32, b: i32, c: i32) {
        self.value = a + b + c
    }
}

fn main() {
    let mut t = Test { value: 0 }
    let x = 10
    let y = 20
    let z = 30
    t.add(x, y, z)
}
"#;

    fs::write("test_copy_no_borrow.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_copy_no_borrow.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(output.status.success(), "wj build should succeed");

    let rust_code = fs::read_to_string("./build/test_copy_no_borrow.rs")
        .expect("Failed to read generated Rust code");

    // i32 is Copy, should not borrow
    assert!(
        !rust_code.contains("&x"),
        "Should not borrow x (Copy type): \n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("&y"),
        "Should not borrow y (Copy type): \n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("&z"),
        "Should not borrow z (Copy type): \n{}",
        rust_code
    );

    // Cleanup
    fs::remove_file("test_copy_no_borrow.wj").ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_borrow_consistency() {
    let source = r#"
struct Physics {
    x: f32,
    y: f32,
}

impl Physics {
    fn apply_forces(&mut self, fx: f32, fy: f32, fz: f32) {
        self.x += fx + fy + fz
    }
}

fn simulate() {
    let mut obj = Physics { x: 0.0, y: 0.0 }
    let force_x = 1.0
    let force_y = 2.0
    let force_z = 3.0
    
    // All three should be treated consistently
    obj.apply_forces(force_x, force_y, force_z)
}
"#;

    fs::write("test_borrow_consistency.wj", source).unwrap();

    let output = Command::new("./target/release/wj")
        .args(["build", "test_borrow_consistency.wj", "--no-cargo"])
        .output()
        .expect("Failed to execute wj");

    assert!(output.status.success(), "wj build should succeed");

    let rust_code = fs::read_to_string("./build/test_borrow_consistency.rs")
        .expect("Failed to read generated Rust code");

    // Check that ALL f32 args are handled consistently
    let has_borrow_fx = rust_code.contains("&force_x");
    let has_borrow_fy = rust_code.contains("&force_y");
    let has_borrow_fz = rust_code.contains("&force_z");

    // Either ALL should be borrowed or NONE should be borrowed (consistency)
    // For Copy types, NONE should be borrowed
    assert_eq!(
        has_borrow_fx, has_borrow_fy,
        "force_x and force_y should be handled consistently\n{}",
        rust_code
    );
    assert_eq!(
        has_borrow_fy, has_borrow_fz,
        "force_y and force_z should be handled consistently\n{}",
        rust_code
    );

    // For f32 (Copy type), none should be borrowed
    assert!(
        !has_borrow_fx && !has_borrow_fy && !has_borrow_fz,
        "No f32 arguments should be borrowed (they are Copy)\n{}",
        rust_code
    );

    // Cleanup
    fs::remove_file("test_borrow_consistency.wj").ok();
}
