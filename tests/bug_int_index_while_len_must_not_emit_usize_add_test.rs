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
    feature = "integration_tests",
))]

//! `wj-glob` match_segs: `int` index params + `while k <= texts.len()` must not promote
//! `ti` to usize (E0308 `ti >= texts.len()`, E0277 `ti + 1_usize` in recursive call).

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str = r#"
fn match_segs(pats: Vec<string>, pi: int, texts: Vec<string>, ti: int) -> bool {
    if pi >= pats.len() {
        return ti >= texts.len()
    }
    let pat = "${pats[pi]}"
    if pat == "**" {
        let mut k = ti
        while k <= texts.len() {
            if match_segs(pats, pi + 1, texts, k) {
                return true
            }
            k = k + 1
        }
        return false
    }
    if ti >= texts.len() {
        return false
    }
    match_segs(pats, pi + 1, texts, ti + 1)
}

pub fn ok(pats: Vec<string>, texts: Vec<string>) -> bool {
    match_segs(pats, 0, texts, 0)
}
"#;

#[test]
fn int_index_while_len_must_not_emit_usize_add() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert!(
        !rs.contains("+ 1_usize"),
        "recursive ti + 1 must not emit usize literal; generated:\n{rs}"
    );
    assert!(
        ok,
        "cargo check must succeed; generated:\n{rs}"
    );
    // int param vs Vec.len must cast len side to i64 (or compare via cast), not raw usize compare on ti
    if rs.contains("ti >=") && rs.contains("texts.len()") {
        let bad = rs.lines().any(|line| {
            line.contains("ti >=")
                && line.contains("texts.len()")
                && !line.contains("as i64")
                && !line.contains("as usize")
        });
        assert!(
            !bad,
            "ti >= texts.len() must unify int with len; generated:\n{rs}"
        );
    }
}
