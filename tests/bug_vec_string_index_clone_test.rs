#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! TDD Test: Vec<String> index access must clone
//!
//! Bug: `vec[i]` where vec is Vec<String> generates `vec[i]` in Rust, which
//! tries to move a String out of the Vec. Rust requires `.clone()` for
//! non-Copy types accessed by index.
//!
//! Root cause: Codegen doesn't add .clone() when indexing non-Copy elements.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_string_index_needs_clone() {
    let code = r#"
pub fn get_label(labels: Vec<string>) -> string {
    if labels.len() > 0 {
        labels[0]
    } else {
        ""
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);

    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("labels[0].clone()") || generated.contains("(&labels[0]).clone()"),
        "Vec<String> index must .clone() (can't move out of index). Got:\n{}",
        generated
    );
    assert!(
        success,
        "Generated Rust must compile without E0507. Error:\n{}",
        generated
    );
}

#[test]
fn test_vec_string_index_in_conditional() {
    let code = r#"
pub fn pick_item(items: Vec<string>, index: i32) -> string {
    if items.len() > index as usize {
        items[index as usize]
    } else {
        ""
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);

    println!("Generated:\n{}", generated);

    assert!(
        success,
        "Vec<String> conditional index must compile. Error:\n{}",
        generated
    );
}

#[test]
fn test_method_return_vec_string_index_needs_clone() {
    let code = r#"
pub struct Container {
    items: Vec<string>,
}

impl Container {
    pub fn get_labels(self) -> Vec<string> {
        self.items
    }

    pub fn display(self) {
        let labels = self.get_labels()
        let l0 = if labels.len() > 0 { labels[0] } else { "" }
        let l1 = if labels.len() > 1 { labels[1] } else { "" }
        let _ = l0
        let _ = l1
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);

    println!("Generated:\n{}", generated);

    assert!(
        success,
        "Method returning Vec<String> then indexing must compile. Error:\n{}",
        generated
    );
}

/// Cross-file: method in module A returns Vec<string>, module B indexes it
#[test]
fn test_library_vec_string_index_needs_clone() {
    let files: &[(&str, &str)] = &[
        ("provider.wj", r#"
pub struct Provider {
    entries: Vec<string>,
}

impl Provider {
    pub fn get_entries(self) -> Vec<string> {
        self.entries
    }
}
"#),
        ("consumer.wj", r#"
use super::provider::Provider

pub struct Consumer {
    source: Provider,
}

impl Consumer {
    pub fn display(self) {
        let items = self.source.get_entries()
        let a = if items.len() > 0 { items[0] } else { "" }
        let b = if items.len() > 1 { items[1] } else { "" }
        let _ = a
        let _ = b
    }
}
"#),
    ];

    let (generated, success) = test_utils::compile_project_dir(files);

    for (name, code) in &generated {
        println!("=== {} ===\n{}", name, code);
    }

    assert!(
        success,
        "Library Vec<String> index must compile (E0507 fix)"
    );
}

/// Vec<String> index used in field assignment: `self.name = parts[1]`
#[test]
fn test_vec_string_index_in_field_assignment() {
    let code = r#"
pub struct Config {
    name: string,
    label: string,
}

impl Config {
    pub fn parse(self, parts: Vec<string>) {
        self.name = parts[1]
        self.label = parts[2]
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);

    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("parts[1].clone()") || generated.contains("(&parts[1]).clone()"),
        "Vec<String> field assignment must .clone(). Got:\n{}",
        generated
    );
    assert!(
        success,
        "Vec<String> field assignment must compile. Error:\n{}",
        generated
    );
}

/// Vec<String> index used in struct literal field init
#[test]
fn test_vec_string_index_in_struct_init() {
    let code = r#"
pub struct Entry {
    kind: i32,
    label: string,
}

pub fn make_entry(parts: Vec<string>) -> Entry {
    Entry { kind: 0, label: parts[1] }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);

    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("parts[1].clone()") || generated.contains("(&parts[1]).clone()"),
        "Vec<String> in struct init must .clone(). Got:\n{}",
        generated
    );
    assert!(
        success,
        "Vec<String> struct init index must compile. Error:\n{}",
        generated
    );
}

#[test]
fn test_vec_i32_index_no_clone_needed() {
    let code = r#"
pub fn get_value(values: Vec<i32>) -> i32 {
    if values.len() > 0 {
        values[0]
    } else {
        0
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);

    println!("Generated:\n{}", generated);

    assert!(
        !generated.contains("values[0].clone()"),
        "Vec<i32> index should NOT clone (i32 is Copy). Got:\n{}",
        generated
    );
    assert!(
        success,
        "Vec<i32> index must compile. Error:\n{}",
        generated
    );
}
