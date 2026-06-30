#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD: E0507 when matching on Option field behind &mut self
///
/// Bug: `match self.clip { Some(c) => ... }` in an `&mut self` method
/// generates code that tries to move `self.clip` out of the mutable reference.
///
/// Expected: The compiler should generate `match &self.clip { Some(c) => ... }`
/// (or equivalent) when the binding `c` is only read, not consumed.
///
/// Dogfooding source: editor/animation_timeline.wj
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_match_option_field_readonly_in_mut_method() {
    let source = r#"
pub struct AnimClip {
    pub duration: f32,
    pub name: string,
}

pub struct Timeline {
    pub clip: Option<AnimClip>,
    pub current_time: f32,
    pub is_playing: bool,
}

impl Timeline {
    pub fn update(self, dt: f32) {
        if !self.is_playing {
            return
        }
        match self.clip {
            Some(c) => {
                let dur = c.duration
                self.current_time = self.current_time + dt
                if self.current_time > dur {
                    self.current_time = dur
                    self.is_playing = false
                }
            }
            None => {}
        }
    }
}

pub fn main() {
    let mut tl = Timeline {
        clip: Some(AnimClip { duration: 2.0, name: "walk" }),
        current_time: 0.0,
        is_playing: true,
    }
    tl.update(0.5)
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // The generated Rust must NOT directly move self.clip out of &mut self (E0507).
    // Valid strategies: match &self.clip, if let Some(ref c), .as_ref(), or borrow-break clone
    let has_ref_match = rust.contains("match &self.clip")
        || rust.contains("match & self.clip")
        || rust.contains("match &mut self.clip")
        || rust.contains("if let Some(ref ")
        || rust.contains("self.clip.as_ref()")
        || rust.contains("self.clip.clone()");

    assert!(
        has_ref_match,
        "match on Option field in &mut self should use reference match or clone.\nGenerated:\n{}",
        rust
    );

    // The method should be inferred as &mut self (it mutates self.current_time)
    assert!(
        rust.contains("fn update(&mut self"),
        "update should be &mut self.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_match_option_field_in_ref_method() {
    let source = r#"
pub struct Config {
    pub label: Option<string>,
}

impl Config {
    pub fn get_label(self) -> string {
        match self.label {
            Some(l) => l,
            None => "default",
        }
    }
}

pub fn main() {
    let c = Config { label: Some("test") }
    let l = c.get_label()
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // For owned self methods, `match self.label` is fine (no reference issues)
    assert!(
        rust.contains("fn get_label("),
        "get_label should compile.\nGenerated:\n{}",
        rust
    );
}
