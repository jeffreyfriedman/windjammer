#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Bug: `self.color` (a String field) is used in a conditional expression inside
/// a `while` loop. The compiler generates code that moves `self.color` out of
/// `&mut self` on each iteration, which fails because String doesn't implement Copy.
///
/// The compiler must auto-clone `self.field` when the field is a non-Copy type
/// used in a context that would move it out of a mutable reference inside a loop.
#[test]
fn test_self_string_field_in_loop_conditional() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "rating.wj",
        r##"
pub struct Rating {
    value: f32,
    max: i32,
    color: string,
}

impl Rating {
    pub fn new(value: f32) -> Rating {
        Rating {
            value: value,
            max: 5,
            color: "#fbbf24".to_string(),
        }
    }

    pub fn render(self) -> string {
        let mut html = String::new()
        let mut i = 1
        while i <= self.max {
            let filled = i as f32 <= self.value
            let star_color = if filled {
                self.color
            } else {
                "#e2e8f0".to_string()
            }
            html.push_str(star_color)
            i = i + 1
        }
        html
    }
}
"##,
    );

    // self.color in the loop must be cloned, not moved
    t.assert_compiles_without_error();
}
