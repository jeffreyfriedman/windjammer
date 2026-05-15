/// TDD Tests: Auto-borrow &mut for method arguments
///
/// Bug: When passing a local variable to a method that takes &mut T,
/// the codegen generates `.clone()` (or raw pass) instead of `&mut x`.
///
/// Example WJ:
///   let mut buf = Vec::new()
///   self.fill_buffer(buf)  // fill_buffer takes &mut Vec<f32>
///   buf.len()  // buf used after call
///
/// Expected Rust: self.fill_buffer(&mut buf);
/// Actual (broken): self.fill_buffer(buf.clone());
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_local_var_passed_to_mut_param_gets_mut_borrow() {
    let generated = test_utils::compile_single(
        r#"
pub struct Filler {
    pub count: i32,
}

impl Filler {
    pub fn new() -> Filler {
        Filler { count: 0 }
    }

    pub fn fill(self, buf: Vec<f32>) {
        buf.push(1.0)
        self.count = self.count + 1
    }

    pub fn make_buffer(self) -> Vec<f32> {
        let mut buf = Vec::new()
        self.fill(buf)
        buf
    }
}
"#,
    );

    assert!(
        generated.contains("self.fill(&mut buf)"),
        "Expected `self.fill(&mut buf)` when fill() takes &mut Vec<f32> \
         and buf is used after the call.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("buf.clone()"),
        "Should NOT generate buf.clone() for &mut parameter - should use &mut buf instead.\nGenerated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_local_var_passed_to_owned_param_clones_when_reused() {
    let generated = test_utils::compile_single(
        r#"
pub fn consume(items: Vec<i32>) -> i32 {
    items.len() as i32
}

pub fn test_fn() -> i32 {
    let items = Vec::new()
    let count = consume(items)
    let len = items.len()
    count
}
"#,
    );

    // For owned parameters where the variable is reused, clone is correct
    assert!(
        generated.contains("items.clone()") || generated.contains("items)"),
        "For owned params, clone or move is expected.\nGenerated:\n{}",
        generated
    );
}
