#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_single_field_access_borrows_for_iteration() {
    // Baseline: for x in self.items should borrow
    let output = test_utils::compile_single(
        r#"
struct Container {
    items: Vec<i32>,
}

impl Container {
    fn sum(self) -> i32 {
        let mut total = 0
        for item in self.items {
            total = total + item
        }
        total
    }
}
"#,
    );
    assert!(
        output.contains("&self.items") || output.contains("& self.items"),
        "for x in self.items should borrow (&self.items). Got:\n{}",
        output
    );
}

#[test]
fn test_nested_field_access_borrows_for_iteration() {
    // Bug: for x in self.renderer.items should also borrow, but doesn't
    // because should_borrow_for_iteration only checks one level of FieldAccess
    let output = test_utils::compile_single(
        r#"
struct Renderer {
    items: Vec<i32>,
}

struct Engine {
    renderer: Renderer,
}

impl Engine {
    fn total(self) -> i32 {
        let mut sum = 0
        for item in self.renderer.items {
            sum = sum + item
        }
        sum
    }
}
"#,
    );
    assert!(
        output.contains("&self.renderer.items") || output.contains("& self.renderer.items"),
        "for x in self.renderer.items should borrow. Got:\n{}",
        output
    );
}

#[test]
fn test_triple_nested_field_access_borrows() {
    let output = test_utils::compile_single(
        r#"
struct Inner {
    values: Vec<i32>,
}

struct Middle {
    inner: Inner,
}

struct Outer {
    middle: Middle,
}

impl Outer {
    fn count(self) -> i32 {
        let mut n = 0
        for val in self.middle.inner.values {
            n = n + 1
        }
        n
    }
}
"#,
    );
    assert!(
        output.contains("&self.middle.inner.values"),
        "for x in self.middle.inner.values should borrow. Got:\n{}",
        output
    );
}
