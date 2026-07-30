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

//! FAILING REPRO (dogfood): `x + ""` temps must not be
//! auto-borrowed into owned `string` helper parameters when the call is nested
//! in `Vec::push` (`&String` vs `String`).
//! Workaround in product code: push struct literals instead of helper results.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn owned_string_temp_helper_arg_in_vec_push_should_not_auto_borrow() {
    let source = r##"
pub struct TagAssignment {
    tag_id: string,
    line_index: int,
}

pub fn make_tag_assignment(tag_id: string, line_index: int) -> TagAssignment {
    TagAssignment {
        tag_id: tag_id + "",
        line_index: line_index,
    }
}

fn main() {
    let mut out: Vec<TagAssignment> = Vec::new()
    let class_id = "tag~Operations~class" + ""
    out.push(make_tag_assignment(class_id + "", 0))
    println!("{}", out[0].tag_id)
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("tag~Operations~class")
        && !result.contains("error[E0308]")
        && !result.contains("expected `String`, found `&String`");
    assert!(
        ok,
        "owned string temp helper args inside Vec::push should codegen without & borrow. Got:\n{}",
        result
    );
}
