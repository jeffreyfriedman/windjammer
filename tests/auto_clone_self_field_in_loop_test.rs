#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Bug: when self.field (non-Copy) is used inside a for loop and the method
/// is inferred as &mut self, the codegen must auto-clone the field access.
/// Without the clone, Rust gives E0507: cannot move out of `self.field`
/// which is behind a mutable reference.
#[test]
fn test_self_field_auto_clone_in_loop_mut_self() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "rating.wj",
        r##"
pub struct Rating {
    max: i32,
    color: string,
    value: f32,
}

impl Rating {
    pub fn new() -> Rating {
        Rating { max: 5, color: String::from("#ffd700"), value: 3.5 }
    }

    pub fn render(self) -> string {
        let mut html = String::new()
        for i in 1..=self.max {
            let filled = (i as f32) <= self.value
            let star_color = if filled {
                self.color
            } else {
                String::from("#e2e8f0")
            }
            html = html + star_color
        }
        html
    }
}
"##,
    );

    t.assert_contains("rating.rs", ".clone()");
}
