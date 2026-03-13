//! TDD Tests: Compiler Error Message Quality
//!
//! These tests verify that Windjammer compiler errors are helpful, actionable,
//! and user-friendly. We test the ErrorMapper's translation of Rust errors
//! to Windjammer diagnostics.
//!
//! Run with: cargo test --test error_messages_test

use windjammer::error_mapper::{DiagnosticLevel, ErrorMapper, WindjammerDiagnostic};
use windjammer::source_map::SourceMap;

/// Helper to create a rustc JSON diagnostic line
fn rustc_diagnostic_json(
    message: &str,
    level: &str,
    file: &str,
    line: usize,
    column: usize,
    label: Option<&str>,
    code: Option<&str>,
    children: &[(&str, &str)], // (level, message)
) -> String {
    let label_json = label
        .map(|l| format!(r#","label":"{}""#, l.replace('"', "\\\"")))
        .unwrap_or_default();
    let code_obj = code
        .map(|c| format!(r#"{{"code":"{}"}}"#, c))
        .unwrap_or_else(|| "null".to_string());
    let children_json: String = children
        .iter()
        .map(|(lvl, msg)| {
            format!(
                r#"{{"message":"{}","level":"{}","spans":[],"children":[]}}"#,
                msg.replace('"', "\\\""),
                lvl
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"reason":"compiler-message","message":{{"message":"{}","level":"{}","spans":[{{"file_name":"{}","line_start":{},"line_end":{},"column_start":{},"column_end":{},"is_primary":true{}}}],"code":{},"children":[{}],"rendered":null}}}}"#,
        message.replace('"', "\\\""),
        level,
        file,
        line,
        line,
        column,
        column + 4,
        label_json,
        code_obj,
        children_json
    )
}

/// Helper to run ErrorMapper and get first diagnostic
fn map_first_diagnostic(json: &str) -> Option<WindjammerDiagnostic> {
    let mapper = ErrorMapper::new(SourceMap::new());
    let diagnostics = mapper.map_rustc_output(json);
    diagnostics.into_iter().next()
}

#[test]
fn test_type_mismatch_shows_both_types() {
    // Type mismatch should show expected and found types in Windjammer terminology
    let json = rustc_diagnostic_json(
        "mismatched types",
        "error",
        "test.rs",
        10,
        5,
        Some("expected `i32`, found `f64`"),
        Some("E0308"),
        &[],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    assert_eq!(diag.level, DiagnosticLevel::Error);
    assert!(
        diag.message.contains("int") || diag.message.contains("i32"),
        "Should show expected type (int). Got: {}",
        diag.message
    );
    assert!(
        diag.message.contains("float") || diag.message.contains("f64"),
        "Should show found type (float). Got: {}",
        diag.message
    );
    assert!(
        diag.message.contains("Type mismatch"),
        "Should say 'Type mismatch'. Got: {}",
        diag.message
    );
}

#[test]
fn test_type_mismatch_suggests_conversion() {
    // int/float mismatch should suggest conversion
    let json = rustc_diagnostic_json(
        "mismatched types",
        "error",
        "test.wj",
        10,
        5,
        Some("expected `i32`, found `f64`"),
        Some("E0308"),
        &[],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    let formatted = diag.format();
    // format() includes contextual help - conversion suggestion for int/float
    let has_conversion_help = formatted.contains("as int")
        || formatted.contains("as float")
        || formatted.contains("convert")
        || formatted.contains(".0")
        || formatted.contains("suggestion");
    assert!(
        has_conversion_help,
        "Should suggest conversion. Formatted output:\n{}",
        formatted
    );
}

#[test]
fn test_ownership_error_shows_move_and_borrow() {
    // Ownership errors should explain move/borrow
    let json = rustc_diagnostic_json(
        "cannot move out of `data` which is behind a shared reference",
        "error",
        "test.rs",
        15,
        5,
        None,
        Some("E0507"),
        &[],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    assert!(
        diag.message.contains("data") || diag.message.contains("move") || diag.message.contains("borrow"),
        "Should mention the value or move/borrow. Got: {}",
        diag.message
    );
    assert!(
        diag.message.contains("Ownership") || diag.message.contains("moved") || diag.message.contains("borrow"),
        "Should explain ownership. Got: {}",
        diag.message
    );
}

#[test]
fn test_ownership_error_suggests_clone() {
    // Ownership errors should suggest clone when appropriate
    let json = rustc_diagnostic_json(
        "use of moved value: `data`",
        "error",
        "test.rs",
        15,
        5,
        None,
        Some("E0382"),
        &[],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    let formatted = diag.format();
    let has_clone_suggestion = formatted.contains("clone")
        || formatted.contains("Clone")
        || formatted.contains("suggestion");
    assert!(
        has_clone_suggestion,
        "Should suggest clone for ownership errors. Formatted:\n{}",
        formatted
    );
}

#[test]
fn test_missing_field_lists_available_fields() {
    // "no field X" should list available fields when we can extract them
    // Rustc format: "no field `name` on type `User`"
    let json = rustc_diagnostic_json(
        "no field `name` on type `User`",
        "error",
        "test.rs",
        20,
        10,
        Some("field not found"),
        Some("E0609"),
        &[(
            "note",
            "struct `User` has fields `username`, `email`, `age`",
        )],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    // Should mention the field name and struct
    assert!(
        diag.message.contains("name") || diag.message.contains("field"),
        "Should mention field. Got: {}",
        diag.message
    );
    // Notes from rustc children should be preserved
    let has_field_info = diag.notes.iter().any(|n| n.contains("username") || n.contains("User"));
    assert!(
        has_field_info || diag.message.contains("User"),
        "Should list available fields or struct name. Notes: {:?}",
        diag.notes
    );
}

#[test]
fn test_missing_field_fuzzy_matches() {
    // When field is typo, should suggest "did you mean?"
    // "nam" is 1 edit from "name" (Levenshtein distance 1)
    let json = rustc_diagnostic_json(
        "no field `nam` on type `User`",
        "error",
        "test.rs",
        20,
        10,
        None,
        Some("E0609"),
        &[("note", "struct `User` has fields `username`, `name`, `email`")],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    let formatted = diag.format();
    let has_did_you_mean = formatted.contains("did you mean")
        || formatted.contains("Did you mean")
        || diag.help.iter().any(|h| h.contains("did you mean"));
    let suggests_name = diag.help.iter().any(|h| h.contains("name"));
    assert!(
        has_did_you_mean && suggests_name,
        "Should suggest 'did you mean `name`?'. Help: {:?}, Formatted:\n{}",
        diag.help,
        formatted
    );
}

#[test]
fn test_trait_not_impl_shows_impl_template() {
    // Trait not implemented should show helpful impl template
    let json = rustc_diagnostic_json(
        "the trait `Display` is not implemented for `MyType`",
        "error",
        "test.rs",
        25,
        5,
        None,
        Some("E0277"),
        &[],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    assert!(
        diag.message.contains("Display") || diag.message.contains("trait"),
        "Should mention trait. Got: {}",
        diag.message
    );
    assert!(
        diag.message.contains("MyType") || diag.message.contains("type"),
        "Should mention type. Got: {}",
        diag.message
    );
    // format() includes contextual help - should suggest impl
    let formatted = diag.format();
    let has_impl_suggestion = formatted.contains("impl")
        || formatted.contains("Implement")
        || formatted.contains("suggestion");
    assert!(
        has_impl_suggestion,
        "Should suggest implementing the trait. Formatted:\n{}",
        formatted
    );
}

#[test]
fn test_parse_error_shows_context() {
    // Parse errors - test via CompileError/ParseError display
    // Parse errors come from parser, not ErrorMapper. Test error.rs format.
    use windjammer::error::CompileError;

    let err = CompileError::parse_error("expected `)`, found `]`", "file.wj", 30, 15)
        .with_snippet("    my_func(a, b]")
        .suggest("Add `)` to close the function call");

    let output = format!("{}", err);
    assert!(output.contains("expected"), "Should show expected. Got: {}", output);
    assert!(output.contains(")"), "Should mention ). Got: {}", output);
    assert!(output.contains("file.wj:30:15"), "Should show location. Got: {}", output);
    assert!(output.contains("help:") || output.contains("suggestion"), "Should have suggestion. Got: {}", output);
}

#[test]
fn test_diagnostic_includes_line_column_info() {
    // All diagnostics should include file:line:column
    let json = rustc_diagnostic_json(
        "mismatched types",
        "error",
        "src/main.wj",
        42,
        7,
        Some("expected `int`, found `string`"),
        Some("E0308"),
        &[],
    );

    let diag = map_first_diagnostic(&json).expect("Should parse diagnostic");
    let formatted = diag.format();
    assert!(
        formatted.contains("42") && formatted.contains("7"),
        "Should include line and column. Got:\n{}",
        formatted
    );
    assert!(
        formatted.contains(".wj") || formatted.contains("main"),
        "Should reference Windjammer file. Got:\n{}",
        formatted
    );
}

#[test]
fn test_multiple_errors_dont_cascade() {
    // Multiple distinct errors should each be reported separately
    let json1 = rustc_diagnostic_json(
        "mismatched types",
        "error",
        "a.rs",
        1,
        1,
        Some("expected `int`, found `string`"),
        Some("E0308"),
        &[],
    );
    let json2 = rustc_diagnostic_json(
        "cannot find function `foo`",
        "error",
        "b.rs",
        2,
        1,
        None,
        Some("E0425"),
        &[],
    );

    let mapper = ErrorMapper::new(SourceMap::new());
    let combined = format!("{}\n{}", json1, json2);
    let diagnostics = mapper.map_rustc_output(&combined);

    assert_eq!(diagnostics.len(), 2, "Should have 2 distinct diagnostics");
    assert!(
        diagnostics[0].message.contains("Type mismatch") || diagnostics[0].message.contains("mismatched"),
        "First should be type error"
    );
    assert!(
        diagnostics[1].message.contains("foo") || diagnostics[1].message.contains("Function"),
        "Second should be function not found"
    );
}
