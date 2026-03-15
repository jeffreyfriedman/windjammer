//! TDD: Ownership-based ref/ref mut pattern generation
//!
//! Replaces guessing with systematic ownership queries. Fixes E0596/E0594 permanently.
//!
//! Philosophy: "Safety Without Ceremony" - automatic ref/ref mut, correct by construction.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile(src: &str) -> String {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, src).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
    let wj_binary = if wj_binary.exists() {
        wj_binary
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/wj")
    };

    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated_file = test_dir.join("build/test.rs");
    std::fs::read_to_string(&generated_file).expect("Failed to read generated file")
}

fn compile_and_rustc(src: &str) -> (String, bool) {
    let rs = compile(src);
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let main_rs = test_dir.join("main.rs");
    std::fs::write(&main_rs, &rs).expect("Failed to write Rust");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            test_dir.join("test.rlib").to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");
    if !output.status.success() {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&output.stderr));
    }
    (rs, output.status.success())
}

// =============================================================================
// Shared borrow (&T) → ref only, never ref mut
// =============================================================================

#[test]
fn test_ref_for_shared_borrow_scrutinee() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    let r = &opt
    if let Some(val) = r {
        val
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref val)"),
        "Should use ref for &Option scrutinee. Got:\n{}",
        result
    );
    assert!(
        !result.contains("ref mut val"),
        "Should NOT use ref mut for shared borrow. Got:\n{}",
        result
    );
}

#[test]
fn test_ref_for_borrowed_option_param() {
    let src = r#"
pub fn read(opt: Option<i32>) -> i32 {
    let r = &opt
    if let Some(v) = r {
        v
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref v)"),
        "Should use ref for &Option. Got:\n{}",
        result
    );
}

// =============================================================================
// Mutable borrow (&mut T) → ref mut when mutated
// =============================================================================

#[test]
fn test_ref_mut_for_mut_borrow_when_mutated() {
    let src = r#"
pub struct Counter { pub value: i32 }
pub fn increment(opt: Option<Counter>) {
    let mut o = opt
    let r = &mut o
    if let Some(c) = r {
        c.value = c.value + 1
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref mut c)"),
        "Should use ref mut when mutating through &mut. Got:\n{}",
        result
    );
}

#[test]
fn test_ref_mut_when_body_mutates() {
    let src = r#"
pub struct Slot { pub q: i32 }
impl Slot { pub fn add(self, n: i32) {} }
pub struct Container { pub slots: Vec<Option<Slot>> }
impl Container {
    pub fn update(self, i: usize, n: i32) {
        if let Some(s) = self.slots[i] {
            s.q = s.q + n
        }
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref mut s)"),
        "Should use ref mut when mutating through &mut self. Got:\n{}",
        result
    );
}

// =============================================================================
// Owned scrutinee → mut or plain
// =============================================================================

#[test]
fn test_mut_for_owned_scrutinee_when_mutated() {
    let src = r#"
pub fn process(mut opt: Option<i32>) -> Option<i32> {
    if let Some(val) = opt {
        let new_val = val + 1
        Some(new_val)
    } else {
        None
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        !result.contains("ref mut"),
        "Owned scrutinee should use mut not ref mut. Got:\n{}",
        result
    );
}

#[test]
fn test_plain_for_owned_read_only() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    if let Some(val) = opt {
        val
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        !result.contains("ref mut"),
        "Read-only should not use ref mut. Got:\n{}",
        result
    );
}

// =============================================================================
// Index on &mut self.field → MutBorrowed
// =============================================================================

#[test]
fn test_index_on_mut_self_ref_mut() {
    let src = r#"
pub struct Item { pub id: i32 }
pub struct Stack { pub q: i32 }
impl Stack { pub fn add(self, n: i32) {} }
pub struct Inv { pub slots: Vec<Option<Stack>> }
impl Inv {
    pub fn add_item(self, item: Item, q: i32) -> bool {
        let mut i = 0
        while i < 2 {
            if let Some(stack) = self.slots[i as usize] {
                if stack.q + q <= 100 {
                    stack.add(q)
                    return true
                }
            }
            i = i + 1
        }
        false
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref mut stack)"),
        "Index on &mut self should use ref mut. Got:\n{}",
        result
    );
}

// =============================================================================
// Index on owned Vec → Borrowed (add & or &mut when needed)
// =============================================================================

#[test]
fn test_index_on_mut_vec_ref_mut() {
    // Uses &mut self.slots[i] - Index on &mut self yields &mut T
    let src = r#"
pub struct Slot { pub q: i32 }
impl Slot { pub fn add(self, n: i32) {} }
pub struct Container { pub slots: Vec<Option<Slot>> }
impl Container {
    pub fn transfer(self, i: usize, n: i32) {
        if let Some(stack) = self.slots[i] {
            stack.q = stack.q + n
        }
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref mut stack)"),
        "Should use ref mut for Index on &mut self when mutated. Got:\n{}",
        result
    );
}

#[test]
fn test_index_on_owned_vec_ref_when_read_only() {
    let src = r#"
pub struct Slot { pub q: i32 }
pub fn read(slots: Vec<Option<Slot>>, i: usize) -> i32 {
    if let Some(s) = slots[i] {
        s.q
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref s)"),
        "Should use ref for read-only Index. Got:\n{}",
        result
    );
}

// =============================================================================
// No ref mut for shared borrow (E0596 prevention)
// =============================================================================

#[test]
fn test_no_ref_mut_from_shared_borrow() {
    let src = r#"
pub fn try_mutate(opt: Option<i32>) {
    let r = &opt
    if let Some(v) = r {
        let _ = v
    }
}
pub fn main() {}
"#;
    let (result, compiles) = compile_and_rustc(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        !result.contains("ref mut v"),
        "Cannot use ref mut when scrutinee is &. Got:\n{}",
        result
    );
}
