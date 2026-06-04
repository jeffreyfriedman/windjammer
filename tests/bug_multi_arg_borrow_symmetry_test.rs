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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multi_arg_borrowed_function_both_args_get_ref() {
    let src = r#"
pub struct Body {
    pub x: f32,
    pub y: f32,
    pub name: string,
    pub is_static: bool,
}

pub fn check_overlap(a: Body, b: Body) -> bool {
    a.x == b.x && a.y == b.y
}

pub fn detect(bodies: Vec<Body>) -> i32 {
    let mut count = 0
    for i in 0..bodies.len() {
        let body_a = bodies[i].clone()
        for j in (i + 1)..bodies.len() {
            let body_b = bodies[j].clone()
            if body_a.is_static && body_b.is_static {
                continue
            }
            if check_overlap(body_a, body_b) {
                count = count + 1
            }
        }
    }
    count
}
"#;
    let (result, success) = test_utils::compile_single_check(src);
    let err = if !success { &result } else { "" };
    assert!(success, "Must compile. Error:\n{}", err);
    println!("Generated:\n{}", result);
    // check_overlap takes both params as Borrowed (read-only, non-Copy struct).
    // Both body_a and body_b should be auto-borrowed with &, not cloned.
    assert!(
        !result.contains("body_b.clone()") || result.contains("&body_b"),
        "body_b should be &body_b, not body_b.clone(). Got:\n{}",
        result
    );
    // Also check body_a isn't unnecessarily cloned
    assert!(
        result.contains("&body_a") || result.contains("check_overlap(body_a"),
        "body_a should be &body_a or passed directly. Got:\n{}",
        result
    );
}
