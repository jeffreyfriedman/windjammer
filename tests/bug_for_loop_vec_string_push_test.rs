//! Nested `for` over a match-bound `Vec<string>` must not push `&String`.
//! Ecosystem `wj-fs-walk`: `Ok(nested) => { for p in nested { out.push(p) } }`.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn for_loop_push_string_moves_or_clones_elem() {
    let source = r#"
pub fn append_all(mut out: Vec<string>, nested: Vec<string>) -> Vec<string> {
    for p in nested {
        out.push(p)
    }
    out
}
"#;
    let generated = test_utils::compile_single(source);
    let bad_ref_push = generated.contains("for p in &nested")
        && generated.contains("out.push(p)")
        && !generated.contains("out.push(p.clone())")
        && !generated.contains("out.push((*p).clone())")
        && !generated.contains("out.push(p.to_owned())");
    assert!(
        !bad_ref_push,
        "for-loop push must not push &String from &nested without clone, got:\n{generated}"
    );
}

#[test]
fn nested_match_for_push_string_owns_or_clones() {
    let source = r#"
use std::fs

fn walk_into(dir: string) -> Result<Vec<string>, string> {
    match fs.read_dir(dir) {
        Ok(entries) => {
            let mut out = Vec::new()
            for entry in entries {
                let path = entry.path()
                if entry.is_dir() {
                    out.push("${path}")
                    match walk_into(path) {
                        Ok(nested) => {
                            for p in nested {
                                out.push(p)
                            }
                        },
                        Err(e) => return Err(e),
                    }
                } else if entry.is_file() {
                    out.push(path)
                }
            }
            Ok(out)
        },
        Err(e) => Err(e),
    }
}

pub fn walk(root: string) -> Result<Vec<string>, string> {
    walk_into(root)
}
"#;
    let generated = test_utils::compile_single(source);
    // Accept by-value iteration OR clone when iterating by shared ref.
    let ok = (generated.contains("for p in nested")
        && !generated.contains("for p in &nested")
        && generated.contains("out.push(p)"))
        || generated.contains("out.push(p.clone())")
        || generated.contains("out.push((*p).clone())")
        || generated.contains("out.push(p.to_owned())")
        || generated.contains("out.push(p.to_string())");
    assert!(
        ok,
        "nested match for-push must own String elems, got:\n{generated}"
    );
}
