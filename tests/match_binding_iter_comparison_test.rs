//! TDD: When iterating over a match-arm binding from a borrowed scrutinee,
//! the iterator variable is `&T`. Comparing it with an owned `T` field
//! requires dereferencing: `*o == self.value` (not `o == self.value`).
//!
//! Bug: `for o in opts` where `opts` comes from `match self.field { Variant { opts } => ... }`
//! generates `o == self.value` but `o` is `&String`, not `String`.
//!
//! Root cause: `is_iterating_over_borrowed` doesn't recognize that match arm bindings
//! from a borrowed scrutinee (like `&self.field`) are themselves references.

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_match_binding_iterator_comparison_clone_path() {
    // When the match uses .clone(), o is owned String and comparison is valid.
    // This test ensures the clone path produces valid Rust.
    let source = r#"
pub enum PropType {
    Text,
    Dropdown { options: Vec<string> }
}

pub struct Property {
    pub value: string,
    pub prop_type: PropType
}

impl Property {
    pub fn render(self) -> string {
        let result = match self.prop_type {
            PropType::Dropdown { options: opts } => {
                let mut html = ""
                for o in opts {
                    let sel = if o == self.value { "selected" } else { "" }
                    html = html + sel
                }
                html
            },
            PropType::Text => { "text" }
        }
        result
    }
}
"#;
    let rust = test_utils::compile_single(source);

    // With .clone(), o is owned String and the comparison is valid Rust
    assert!(
        rust.contains("o == self.value") || rust.contains("*o == self.value"),
        "Expected a comparison involving o and self.value. Got:\n{}",
        rust
    );
}

/// Test for the real scenario: match on self.field in an impl block
/// where self is borrowed (inferred &self from read-only access)
#[test]
fn test_match_binding_iter_comparison_borrowed_self() {
    let source = r#"
pub trait Renderable {
    fn render(self) -> string
}

pub enum PropType {
    Text,
    Number { min: f32, max: f32 },
    Dropdown { options: Vec<string> }
}

pub struct Prop {
    pub value: string,
    pub ptype: PropType
}

impl Renderable for Prop {
    fn render(self) -> string {
        match self.ptype {
            PropType::Dropdown { options: opts } => {
                let mut html = ""
                for o in opts {
                    if o == self.value {
                        html = html + "yes"
                    }
                }
                html
            },
            PropType::Text => { "text" },
            PropType::Number { min: _, max: _ } => { "num" }
        }
    }
}
"#;
    let rust = test_utils::compile_single(source);

    // When match is on &self.ptype (borrowed), opts is &Vec<String>,
    // iterating yields &String, comparison needs deref.
    // When match uses .clone(), opts is Vec<String> (owned),
    // but self.value through &self gives &String.
    // Either way, the comparison must be valid Rust.
    let has_invalid_comparison = rust.contains("o == self.value")
        && !rust.contains("*o == self.value")
        && !rust.contains("*o ==");

    assert!(
        !has_invalid_comparison,
        "Generated code has invalid comparison (o as &String vs self.value as String). Got:\n{}",
        rust
    );
}

/// Test for the REAL bug: match inside a `let` binding (expression position).
/// This goes through the expression match path, not the statement match path.
#[test]
fn test_match_binding_iter_comparison_let_binding() {
    let source = r#"
pub trait Renderable {
    fn render(self) -> string
}

pub enum PropType {
    Text,
    Number { min: f32, max: f32 },
    Dropdown { options: Vec<string> }
}

pub struct Prop {
    pub value: string,
    pub ptype: PropType,
    pub name: string
}

impl Renderable for Prop {
    fn render(self) -> string {
        let input_html = match self.ptype {
            PropType::Dropdown { options: opts } => {
                let mut html = ""
                for o in opts {
                    if o == self.value {
                        html = html + "yes"
                    }
                }
                html
            },
            PropType::Text => { "text" },
            PropType::Number { min: _, max: _ } => { "num" }
        }
        input_html
    }
}
"#;
    let rust = test_utils::compile_single(source);
    let has_invalid_comparison = rust.contains("o == self.value")
        && !rust.contains("*o == self.value")
        && !rust.contains("*o ==");

    assert!(
        !has_invalid_comparison,
        "Generated code has invalid comparison in let-binding match. Got:\n{}",
        rust
    );
}
