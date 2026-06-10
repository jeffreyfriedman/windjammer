#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Tests: enumerate index must not be treated as borrowed
///
/// Bug: `for (i, item) in self.items.iter().enumerate()` adds BOTH `i` and `item`
/// to borrowed_iterator_vars. But `i` is always `usize` (owned, Copy) — never a reference.
/// This causes E0614 "type `usize` cannot be dereferenced" when `i` is used in:
/// - if conditions: `if *i > 0` (WRONG, should be `if i > 0`)
/// - casts: `*i as i32` (WRONG, should be `i as i32`)
///
/// Related: push_str with borrowed iteration variable should not clone.
/// When iterating `&Vec<String>` (or `for x in self.items`), the loop var is `&String`.
/// `push_str(child)` is correct since `&String` auto-derefs to `&str`.
/// The compiler should NOT generate `push_str(child.clone())`.
#[path = "common/test_utils.rs"]
mod test_utils;
#[allow(unused_imports)]
use test_utils::compile_single;

#[test]
fn test_enumerate_index_not_dereferenced_in_if_condition() {
    let code = compile_single(
        r#"
struct BreadcrumbItem {
    label: string,
    active: bool,
}

struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
    separator: string,
}

impl Breadcrumb {
    fn render(self) -> string {
        let mut html = ""
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                html.push_str(self.separator)
            }
            html.push_str(item.label)
        }
        html
    }
}

pub fn main() {
    let b = Breadcrumb { items: Vec::new(), separator: "/".to_string() }
    println!("{}", b.render())
}
"#,
    );

    assert!(
        !code.contains("*i >") && !code.contains("*i>"),
        "enumerate index `i` should NOT be dereferenced! Generated:\n{}",
        code
    );
    assert!(
        code.contains("if i > 0") || code.contains("if i > 0_"),
        "Should generate `if i > 0` without dereference! Generated:\n{}",
        code
    );
}

#[test]
fn test_enumerate_index_not_dereferenced_in_cast() {
    let code = compile_single(
        r#"
struct Step {
    label: string,
    completed: bool,
}

struct Stepper {
    steps: Vec<Step>,
    current_step: i32,
}

impl Stepper {
    fn render(self) -> string {
        let mut html = ""
        for (step_index, step) in self.steps.iter().enumerate() {
            let step_index = step_index as i32
            if step_index == self.current_step {
                html.push_str("current")
            }
        }
        html
    }
}

pub fn main() {
    let s = Stepper { steps: Vec::new(), current_step: 0 }
    println!("{}", s.render())
}
"#,
    );

    assert!(
        !code.contains("*step_index as"),
        "enumerate index should NOT be dereferenced before cast! Generated:\n{}",
        code
    );
    assert!(
        code.contains("step_index as i32"),
        "Should generate `step_index as i32` without dereference! Generated:\n{}",
        code
    );
}

#[test]
fn test_borrowed_iteration_push_str_no_clone() {
    let code = compile_single(
        r#"
struct Column {
    children: Vec<string>,
}

impl Column {
    fn render(self) -> string {
        let mut html = ""
        html.push_str("<div>")
        for child in self.children {
            html.push_str(child)
        }
        html.push_str("</div>")
        html
    }
}

pub fn main() {
    let c = Column { children: Vec::new() }
    println!("{}", c.render())
}
"#,
    );

    // When iterating borrowed Vec<String>, push_str(child) should work directly
    // since &String auto-derefs to &str. No .clone() needed.
    let has_clone_in_push = code.contains("push_str(child.clone())")
        || code.contains("push_str(item.clone())")
        || code.contains("push_str(cell.clone())");
    assert!(
        !has_clone_in_push,
        "push_str on borrowed iterator variable should NOT clone! \
         &String auto-derefs to &str. Generated:\n{}",
        code
    );
}
