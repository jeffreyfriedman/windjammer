#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// A method with an implicit `self` has the receiver in slot 0 of its registry signature but not in its declared
/// parameters. The first declared formal read slot 0 and was emitted as `&mut Option<i32>` (rustc E0308 at every
/// caller passing the value). A param stored into a field is owned.
#[test]
fn test_implicit_self_first_option_param_stored_in_field_is_owned() {
    let source = r#"
pub struct Insp { pub sel: Option<i32>, pub n: i32 }

impl Insp {
    pub fn set_sel(v: Option<i32>) {
        self.sel = v
        self.n = 1
    }
}
"#;
    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("fn set_sel(&mut self, v: Option<i32>)"),
        "the stored param is owned, not the receiver's &mut. Got:\n{}",
        generated
    );
}
