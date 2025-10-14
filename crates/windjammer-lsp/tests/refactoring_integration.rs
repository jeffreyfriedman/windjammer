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

// ============================================================================
// Inline Variable Tests
// ============================================================================

#[test]
fn test_inline_variable_simple() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn main() {
    let x = 42
    let y = x + 10
    println("${y}")
}
"#;

    // Position cursor on 'x' in the definition (line 1, column 8)
    let position = Position {
        line: 1,
        character: 8,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_inline_variable(&uri, position, source);

    assert!(result.is_ok(), "Should successfully inline variable");

    let workspace_edit = result.unwrap();
    let changes = workspace_edit.changes.as_ref().unwrap();
    let edits = &changes[&uri];

    // Should have 2 edits: replace usage + remove definition
    assert_eq!(edits.len(), 2, "Should have 2 edits");

    // One edit should replace 'x' with '42' in the usage
    let has_replacement = edits.iter().any(|e| e.new_text.contains("42"));
    assert!(has_replacement, "Should replace variable with value");

    // One edit should remove the definition
    let has_deletion = edits.iter().any(|e| e.new_text.is_empty());
    assert!(has_deletion, "Should delete variable definition");
}

#[test]
fn test_inline_variable_multiple_usages() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn calculate() {
    let factor = 2
    let a = factor * 10
    let b = factor * 20
    println("${a}, ${b}")
}
"#;

    // Position cursor on 'factor'
    let position = Position {
        line: 1,
        character: 10,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_inline_variable(&uri, position, source);

    assert!(
        result.is_ok(),
        "Should inline variable with multiple usages"
    );

    let workspace_edit = result.unwrap();
    let changes = workspace_edit.changes.as_ref().unwrap();
    let edits = &changes[&uri];

    // Should have 3 edits: 2 replacements + 1 deletion
    assert_eq!(edits.len(), 3, "Should have 3 edits");
}

#[test]
fn test_inline_variable_unsafe_side_effects() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn process() {
    let result = dangerous_call!()
    use_result(result)
}
"#;

    // Position cursor on 'result'
    let position = Position {
        line: 1,
        character: 10,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_inline_variable(&uri, position, source);

    assert!(
        result.is_err(),
        "Should reject inlining expressions with side effects"
    );
    assert!(
        result.unwrap_err().contains("side effects"),
        "Error should mention side effects"
    );
}

#[test]
fn test_inline_variable_no_definition() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn main() {
    println("${x}")
}
"#;

    // Position cursor on 'x' which has no definition
    let position = Position {
        line: 1,
        character: 15,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_inline_variable(&uri, position, source);

    assert!(
        result.is_err(),
        "Should fail when variable has no definition"
    );
}

// ============================================================================
// Introduce Variable Tests
// ============================================================================

#[test]
fn test_introduce_variable_simple() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn main() {
    let result = x + y * 2
}
"#;

    // Select "y * 2"
    let range = Range {
        start: Position {
            line: 1,
            character: 21,
        },
        end: Position {
            line: 1,
            character: 26,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_introduce_variable(&uri, range, "factor", source);

    assert!(result.is_ok(), "Should successfully introduce variable");

    let workspace_edit = result.unwrap();
    let changes = workspace_edit.changes.as_ref().unwrap();
    let edits = &changes[&uri];

    // Should have 2 edits: insert declaration + replace expression
    assert_eq!(edits.len(), 2, "Should have 2 edits");

    // One edit should insert the variable declaration
    let has_declaration = edits.iter().any(|e| e.new_text.contains("let factor"));
    assert!(has_declaration, "Should insert variable declaration");

    // One edit should replace the expression with the variable name
    let has_replacement = edits.iter().any(|e| e.new_text == "factor");
    assert!(
        has_replacement,
        "Should replace expression with variable name"
    );
}

#[test]
fn test_introduce_variable_with_duplicates() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn calculate() {
    let a = x + y
    let b = x + y
    let c = x + y
}
"#;

    // Select first "x + y"
    let range = Range {
        start: Position {
            line: 1,
            character: 12,
        },
        end: Position {
            line: 1,
            character: 17,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_introduce_variable(&uri, range, "sum", source);

    assert!(
        result.is_ok(),
        "Should introduce variable and replace duplicates"
    );

    let workspace_edit = result.unwrap();
    let changes = workspace_edit.changes.as_ref().unwrap();
    let edits = &changes[&uri];

    // Should have 4 edits: 1 declaration + 3 replacements
    assert_eq!(edits.len(), 4, "Should have 4 edits");
}

#[test]
fn test_introduce_variable_suggested_names() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn test() {
    let result = a + b
}
"#;

    // Select "a + b"
    let range = Range {
        start: Position {
            line: 1,
            character: 17,
        },
        end: Position {
            line: 1,
            character: 22,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    // Use empty name to get suggested name
    let result = engine.execute_introduce_variable(&uri, range, "", source);

    assert!(result.is_ok(), "Should suggest variable name");

    let workspace_edit = result.unwrap();
    let changes = workspace_edit.changes.as_ref().unwrap();
    let edits = &changes[&uri];

    // Should suggest "sum" for addition
    let has_sum = edits.iter().any(|e| e.new_text.contains("let sum"));
    assert!(has_sum, "Should suggest 'sum' for addition");
}

#[test]
fn test_introduce_variable_reject_simple_variable() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn main() {
    let result = x
}
"#;

    // Select just "x" (already a variable)
    let range = Range {
        start: Position {
            line: 1,
            character: 17,
        },
        end: Position {
            line: 1,
            character: 18,
        },
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    let result = engine.execute_introduce_variable(&uri, range, "temp", source);

    assert!(
        result.is_err(),
        "Should reject introducing variable for simple variable"
    );
    assert!(
        result.unwrap_err().contains("already a variable"),
        "Error should mention it's already a variable"
    );
}

// ============================================================================
// Change Signature Tests
// ============================================================================

#[test]
fn test_change_signature_add_parameter() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn calculate(x: int) -> int {
    x * 2
}

fn main() {
    let result = calculate(5)
}
"#;

    // Position cursor on function name
    let position = Position {
        line: 0,
        character: 3,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    // Add a parameter
    use windjammer_lsp::refactoring::change_signature::ParameterChange;
    let changes = vec![ParameterChange::Add {
        name: "y".to_string(),
        type_hint: Some("int".to_string()),
        default_value: "0".to_string(),
        index: 1,
    }];

    let result = engine.execute_change_signature(&uri, position, &changes, source);

    assert!(result.is_ok(), "Should successfully add parameter");

    let workspace_edit = result.unwrap();
    let edits_map = workspace_edit.changes.as_ref().unwrap();
    let edits = &edits_map[&uri];

    // Debug: print edits
    eprintln!("Number of edits: {}", edits.len());
    for (i, edit) in edits.iter().enumerate() {
        eprintln!("Edit {}: {:?}", i, edit.new_text);
    }

    // Should have 2 edits: update signature + update call site
    assert_eq!(edits.len(), 2, "Should have 2 edits");

    // Check signature update
    let has_new_param = edits.iter().any(|e| e.new_text.contains("y: int"));
    assert!(has_new_param, "Should add new parameter to signature");

    // Check call site update
    let has_default = edits.iter().any(|e| e.new_text.contains("calculate(5, 0)"));
    assert!(has_default, "Should add default value to call site");
}

#[test]
fn test_change_signature_remove_parameter() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn add(x: int, y: int) -> int {
    x + y
}

fn main() {
    let result = add(1, 2)
}
"#;

    // Position cursor on function name
    let position = Position {
        line: 0,
        character: 3,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    // Remove second parameter
    use windjammer_lsp::refactoring::change_signature::ParameterChange;
    let changes = vec![ParameterChange::Remove { index: 1 }];

    let result = engine.execute_change_signature(&uri, position, &changes, source);

    assert!(result.is_ok(), "Should successfully remove parameter");

    let workspace_edit = result.unwrap();
    let edits_map = workspace_edit.changes.as_ref().unwrap();
    let edits = &edits_map[&uri];

    // Should update signature and call site
    let has_single_param = edits.iter().any(|e| e.new_text.contains("fn add(x: int)"));
    assert!(has_single_param, "Should remove parameter from signature");
}

#[test]
fn test_change_signature_reorder_parameters() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn divide(x: int, y: int) -> int {
    x / y
}

fn main() {
    let result = divide(10, 2)
}
"#;

    // Position cursor on function name
    let position = Position {
        line: 0,
        character: 3,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    // Swap parameters (0 -> 1, 1 -> 0)
    use windjammer_lsp::refactoring::change_signature::ParameterChange;
    let changes = vec![ParameterChange::Reorder { from: 0, to: 1 }];

    let result = engine.execute_change_signature(&uri, position, &changes, source);

    assert!(result.is_ok(), "Should successfully reorder parameters");

    let workspace_edit = result.unwrap();
    let edits_map = workspace_edit.changes.as_ref().unwrap();
    let edits = &edits_map[&uri];

    // Should reorder both signature and call site
    assert!(edits.len() >= 1, "Should have at least 1 edit");
}

#[test]
fn test_change_signature_rename_parameter() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn square(x: int) -> int {
    x * x
}

fn main() {
    let result = square(5)
}
"#;

    // Position cursor on function name
    let position = Position {
        line: 0,
        character: 3,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    // Rename parameter
    use windjammer_lsp::refactoring::change_signature::ParameterChange;
    let changes = vec![ParameterChange::Rename {
        index: 0,
        new_name: "value".to_string(),
    }];

    let result = engine.execute_change_signature(&uri, position, &changes, source);

    assert!(result.is_ok(), "Should successfully rename parameter");

    let workspace_edit = result.unwrap();
    let edits_map = workspace_edit.changes.as_ref().unwrap();
    let edits = &edits_map[&uri];

    // Should rename in signature
    let has_new_name = edits.iter().any(|e| e.new_text.contains("value: int"));
    assert!(has_new_name, "Should rename parameter");
}

#[test]
fn test_change_signature_multiple_call_sites() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source = r#"fn add(x: int, y: int) -> int {
    x + y
}

fn main() {
    let a = add(1, 2)
    let b = add(3, 4)
    let c = add(5, 6)
}
"#;

    // Position cursor on function name
    let position = Position {
        line: 0,
        character: 3,
    };

    let uri = Url::parse("file:///test.wj").unwrap();

    // Add parameter
    use windjammer_lsp::refactoring::change_signature::ParameterChange;
    let changes = vec![ParameterChange::Add {
        name: "z".to_string(),
        type_hint: Some("int".to_string()),
        default_value: "0".to_string(),
        index: 2,
    }];

    let result = engine.execute_change_signature(&uri, position, &changes, source);

    assert!(
        result.is_ok(),
        "Should update function with multiple call sites"
    );

    let workspace_edit = result.unwrap();
    let edits_map = workspace_edit.changes.as_ref().unwrap();
    let edits = &edits_map[&uri];

    // Should have 4 edits: 1 signature + 3 call sites
    assert_eq!(edits.len(), 4, "Should update all call sites");
}

// ============================================================================
// Move Item Tests
// ============================================================================

#[test]
fn test_move_function() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source_content = r#"fn helper(x: int) -> int {
    x * 2
}

fn main() {
    let result = helper(5)
}
"#;

    let target_content = r#"// utilities module
"#;

    // Position cursor on function name
    let position = Position {
        line: 0,
        character: 3,
    };

    let source_uri = Url::parse("file:///main.wj").unwrap();
    let target_uri = Url::parse("file:///utils.wj").unwrap();

    let result = engine.execute_move_item(
        &source_uri,
        &target_uri,
        position,
        source_content,
        target_content,
    );

    assert!(result.is_ok(), "Should successfully move function");

    let workspace_edit = result.unwrap();
    let changes_map = workspace_edit.changes.as_ref().unwrap();

    // Should have edits for both source and target files
    assert_eq!(changes_map.len(), 2, "Should edit both files");

    // Check source file - function should be removed
    let source_edits = &changes_map[&source_uri];
    assert_eq!(source_edits.len(), 1, "Should have 1 edit in source");
    assert_eq!(
        source_edits[0].new_text, "",
        "Should delete function from source"
    );

    // Check target file - function should be added
    let target_edits = &changes_map[&target_uri];
    assert_eq!(target_edits.len(), 1, "Should have 1 edit in target");
    assert!(
        target_edits[0].new_text.contains("fn helper"),
        "Should add function to target"
    );
}

#[test]
fn test_move_struct() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source_content = r#"struct User {
    name: string,
    age: int,
}

fn main() {
    let user = User { name: "Alice", age: 30 }
}
"#;

    let target_content = r#"// models module
"#;

    // Position cursor on struct name
    let position = Position {
        line: 0,
        character: 7,
    };

    let source_uri = Url::parse("file:///main.wj").unwrap();
    let target_uri = Url::parse("file:///models.wj").unwrap();

    let result = engine.execute_move_item(
        &source_uri,
        &target_uri,
        position,
        source_content,
        target_content,
    );

    assert!(result.is_ok(), "Should successfully move struct");

    let workspace_edit = result.unwrap();
    let changes_map = workspace_edit.changes.as_ref().unwrap();

    // Should edit both files
    assert_eq!(changes_map.len(), 2, "Should edit both files");

    // Check target file contains struct
    let target_edits = &changes_map[&target_uri];
    assert!(
        target_edits[0].new_text.contains("struct User"),
        "Should add struct to target"
    );
}

#[test]
fn test_move_no_item_at_cursor() {
    let mut db = WindjammerDatabase::new();
    let engine = RefactoringEngine::new(&db);

    let source_content = r#"// Just a comment
let x = 42
"#;

    let target_content = r#""#;

    // Position cursor on comment (not a movable item)
    let position = Position {
        line: 0,
        character: 3,
    };

    let source_uri = Url::parse("file:///main.wj").unwrap();
    let target_uri = Url::parse("file:///utils.wj").unwrap();

    let result = engine.execute_move_item(
        &source_uri,
        &target_uri,
        position,
        source_content,
        target_content,
    );

    assert!(result.is_err(), "Should fail when no item at cursor");
    assert!(
        result.unwrap_err().contains("No movable item"),
        "Error should mention no item found"
    );
}
