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

//! TDD Test: Vec<String> indexing inside if-expression must clone (E0507 fix)
//!
//! Bug: `let x = if cond { vec[0] } else { "" }` generates `vec[0]` without
//! `.clone()`, causing E0507: cannot move out of index of `Vec<String>`.
//!
//! Root cause: When Vec<String> is indexed inside an if-expression body (not a
//! direct `let x = vec[0]` statement), the statement-level clone handler doesn't
//! fire. The expression-level `generate_index` must add `.clone()` or `&`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_string_index_in_if_expr_clones() {
    let code = r#"
struct Game {
    labels: Vec<string>
}

impl Game {
    pub fn get_label(self) -> string {
        let items = self.labels
        if items.len() > 0 {
            items[0]
        } else {
            ""
        }
    }
}
"#;

    let (generated, _) = test_utils::compile_single_check(code);
    println!("Generated:\n{}", generated);

    // The indexing must be safe: either items[0].clone() or &items[0]
    let has_clone = generated.contains("items[0].clone()");
    let has_borrow = generated.contains("&items[0]");
    let has_to_string = generated.contains("items[0].to_string()");
    assert!(
        has_clone || has_borrow || has_to_string,
        "Vec<String> index in if-expr must use .clone(), & borrow, or .to_string() to avoid E0507.\nGot:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_string_index_from_method_call_in_if_expr() {
    let code = r#"
struct Inventory {
    slots: Vec<string>
}

impl Inventory {
    pub fn slot_labels(self) -> Vec<string> {
        self.slots
    }

    pub fn get_first_label(self) -> string {
        let labels = self.slot_labels()
        if labels.len() > 0 {
            labels[0]
        } else {
            ""
        }
    }
}
"#;

    let (generated, _) = test_utils::compile_single_check(code);
    println!("Generated:\n{}", generated);

    // Same fix needed: labels[0] must be safe
    let has_clone = generated.contains("labels[0].clone()")
        || generated.contains("labels[0]").then(|| generated.contains("(&labels[0])")).unwrap_or(false);
    let has_borrow = generated.contains("&labels[0]");
    let has_to_string = generated.contains("labels[0].to_string()");
    assert!(
        has_clone || has_borrow || has_to_string,
        "Vec<String> index from method return in if-expr must be safe.\nGot:\n{}",
        generated
    );
}

/// Test case that mimics cross-file compilation where return type is unknown.
/// When the compiler cannot infer the element type of a Vec, it should still
/// generate safe code (preferring clone over potential E0507).
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_index_unknown_type_defaults_to_clone() {
    // Use extern fn to simulate unknown return type (cross-crate method)
    let code = r#"
extern fn get_labels() -> Vec<String>

pub fn first_or_default() -> string {
    let labels = get_labels()
    if labels.len() > 0 {
        labels[0]
    } else {
        ""
    }
}
"#;

    let (generated, _) = test_utils::compile_single_check(code);
    println!("Generated:\n{}", generated);

    // Must not have bare labels[0] (would cause E0507)
    // Should have either .clone(), &, or .to_string()
    let lines: Vec<&str> = generated.lines().collect();
    let has_bare_index = lines.iter().any(|l| {
        let trimmed = l.trim();
        trimmed == "labels[0]" || trimmed == "labels[0usize]"
    });

    assert!(
        !has_bare_index,
        "Bare Vec<String> index without clone/borrow generates E0507.\nGot:\n{}",
        generated
    );
}
