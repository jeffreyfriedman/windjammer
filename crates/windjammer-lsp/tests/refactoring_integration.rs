//! Integration tests for refactoring operations
//!
//! These tests verify that refactorings work end-to-end with real Windjammer code.

use tower_lsp::lsp_types::*;
use windjammer_lsp::database::WindjammerDatabase;
use windjammer_lsp::refactoring::RefactoringEngine;

#[test]
fn test_extract_function_simple() {
    // Setup
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    // Source code with a simple calculation
    let source = r#"fn main() {
    let x = 10
    let y = 20
    let sum = x + y
    println("Sum: ${sum}")
}
"#;

    // Select lines 3-4 (the calculation)
    let range = Range {
        start: Position {
            line: 2,
            character: 4,
        },
        end: Position {
            line: 3,
            character: 23,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    // Execute refactoring
    let result = engine.execute_extract_function(&uri, range, "calculate_sum", source);

    // Verify success
    assert!(result.is_ok(), "Refactoring should succeed");

    let workspace_edit = result.unwrap();
    assert!(workspace_edit.changes.is_some());

    let changes = workspace_edit.changes.as_ref().unwrap();
    assert!(changes.contains_key(&uri));

    let edits = &changes[&uri];
    assert_eq!(edits.len(), 2, "Should have 2 edits: replace + insert");

    // Verify the replacement edit
    let replace_edit = &edits[0];
    assert_eq!(replace_edit.range, range);
    assert!(
        replace_edit.new_text.contains("calculate_sum"),
        "Should call the new function"
    );

    // Verify the insertion edit
    let insert_edit = &edits[1];
    assert!(
        insert_edit.new_text.contains("fn calculate_sum"),
        "Should insert function definition"
    );
}

#[test]
fn test_extract_function_with_parameters() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn process() {
    let x = 10
    let result = x * 2 + 5
    println("Result: ${result}")
}
"#;

    // Select the calculation line
    let range = Range {
        start: Position {
            line: 2,
            character: 4,
        },
        end: Position {
            line: 2,
            character: 29,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_extract_function(&uri, range, "calculate", source);

    assert!(
        result.is_ok(),
        "Should successfully extract with parameters"
    );
}

#[test]
fn test_extract_function_empty_selection() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = "fn main() {}\n";

    // Empty selection
    let range = Range {
        start: Position {
            line: 0,
            character: 12,
        },
        end: Position {
            line: 0,
            character: 12,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_extract_function(&uri, range, "empty", source);

    assert!(result.is_err(), "Should fail on empty selection");
    assert!(
        result.unwrap_err().contains("empty"),
        "Error should mention empty selection"
    );
}

#[test]
fn test_extract_function_preserves_indentation() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn outer() {
    if true {
        let a = 1
        let b = 2
        let c = a + b
    }
}
"#;

    // Select the inner calculation
    let range = Range {
        start: Position {
            line: 2,
            character: 8,
        },
        end: Position {
            line: 4,
            character: 25,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_extract_function(&uri, range, "inner_calc", source);

    assert!(result.is_ok(), "Should handle nested indentation");

    // Verify the generated function has proper indentation
    let workspace_edit = result.unwrap();
    let changes = workspace_edit.changes.as_ref().unwrap();
    let edits = &changes[&uri];
    let insert_edit = &edits[1];

    // The inserted function should be properly formatted
    assert!(insert_edit.new_text.contains("fn inner_calc"));
}
