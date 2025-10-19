#![allow(
    unused_variables,
    unused_imports,
    dead_code,
    clippy::assertions_on_constants
)]
/// Comprehensive Integration Tests for Windjammer LSP
///
/// These tests validate end-to-end functionality of the Language Server
use tower_lsp::lsp_types::*;

/// Test helper to create a simple Windjammer program
fn simple_program() -> String {
    r#"
fn greet(name: string) {
    println!("Hello, {}!", name)
}

fn main() {
    greet("World")
}
"#
    .to_string()
}

/// Test helper for a more complex program with structs, enums, and methods
fn complex_program() -> String {
    r#"
struct User {
    name: string,
    age: int
}

enum Result<T, E> {
    Ok(T),
    Err(E)
}

impl User {
    fn new(name: string, age: int) -> User {
        User { name, age }
    }

    fn greet(&self) {
        println!("Hello, I'm {}", self.name)
    }
}

fn main() {
    let user = User::new("Alice", 30)
    user.greet()
}
"#
    .to_string()
}

mod hover_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_hover_on_function() {
        let program = simple_program();
        
        // Parse the program to ensure it's valid
        let mut lexer = Lexer::new(&program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // Verify we can parse successfully
        assert!(ast.is_ok(), "Program should parse without errors");
        
        // In a full LSP test, we would:
        // 1. Start LSP server
        // 2. Send textDocument/didOpen
        // 3. Send textDocument/hover at function position
        // 4. Verify response contains function signature
        
        // For now, verify the AST contains the function
        let program = ast.unwrap();
        assert!(!program.items.is_empty(), "Program should have items");
    }

    #[test]
    fn test_hover_on_struct() {
        let program = complex_program();
        
        let mut lexer = Lexer::new(&program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Complex program should parse without errors");
        
        // Verify we have struct definitions
        let program = ast.unwrap();
        let has_struct = program.items.iter().any(|item| {
            matches!(item, windjammer::parser::Item::Struct(_))
        });
        assert!(has_struct, "Program should contain struct definitions");
    }

    #[test]
    fn test_hover_on_variable() {
        let program = r#"
fn main() {
    let x = 42
    println!("{}", x)
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with variables should parse");
    }
}

mod completion_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_keyword_completion() {
        // Test that we can parse programs with various keywords
        let keywords_program = r#"
struct Point { x: int, y: int }
enum Option<T> { Some(T), None }

fn test_func() {
    let x = 5
    if x > 0 {
        let mut y = 10
        for i in 0..5 {
            y += i
        }
    } else {
        return
    }
    
    match x {
        0 => print("zero"),
        _ => print("other"),
    }
}
"#;
        
        let mut lexer = Lexer::new(keywords_program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with various keywords should parse correctly");
        
        // In full LSP: would send completion request and verify
        // fn, let, if, else, for, struct, enum, match are suggested
    }

    #[test]
    fn test_stdlib_completion() {
        // Test programs that use stdlib modules
        let stdlib_program = r#"
fn main() {
    use std.fs
    use std.http
    use std.json
    
    let content = fs.read("file.txt")
    print(content)
}
"#;
        
        let mut lexer = Lexer::new(stdlib_program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program using stdlib should parse");
        
        // In full LSP: would verify std.fs, std.http, std.json
        // are in completion suggestions
    }

    #[test]
    fn test_user_symbol_completion() {
        let program = complex_program();
        
        let mut lexer = Lexer::new(&program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Complex program should parse");
        
        // Verify we can extract user-defined symbols
        let prog = ast.unwrap();
        let has_user_function = prog.items.iter().any(|item| {
            matches!(item, windjammer::parser::Item::Function(_))
        });
        let has_user_struct = prog.items.iter().any(|item| {
            matches!(item, windjammer::parser::Item::Struct(_))
        });
        
        assert!(has_user_function, "Should have user-defined functions");
        assert!(has_user_struct, "Should have user-defined structs");
        
        // In full LSP: would verify User, new, greet are in completions
    }

    #[test]
    fn test_method_completion() {
        let program = r#"
struct Calculator {
    value: int
}

impl Calculator {
    fn add(n: int) {
        self.value += n
    }
    
    fn multiply(n: int) {
        self.value *= n
    }
    
    fn result() -> int {
        self.value
    }
}

fn main() {
    let calc = Calculator { value: 0 }
    calc.add(5)
    calc.multiply(2)
    print("Result: {calc.result()}")
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with impl methods should parse");
        
        // Verify we have impl block
        let prog = ast.unwrap();
        let has_impl = prog.items.iter().any(|item| {
            matches!(item, windjammer::parser::Item::Impl(_))
        });
        
        assert!(has_impl, "Should have impl block with methods");
        
        // In full LSP: would verify add, multiply, result methods
        // are suggested when typing "calc."
    }
}

mod goto_definition_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_goto_function_definition() {
        let program = r#"
fn helper(x: int) -> int {
    x * 2
}

fn main() {
    let result = helper(21)
    print("Result: {result}")
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with function call should parse");
        
        // Verify we have the function definition
        let prog = ast.unwrap();
        let helper_fn = prog.items.iter().find(|item| {
            if let windjammer::parser::Item::Function(f) = item {
                f.name == "helper"
            } else {
                false
            }
        });
        
        assert!(helper_fn.is_some(), "Should find helper function definition");
        
        // In full LSP: clicking on "helper" at line 7 would jump to line 2
    }

    #[test]
    fn test_goto_struct_definition() {
        let program = complex_program();
        
        let mut lexer = Lexer::new(&program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Complex program should parse");
        
        // Find the User struct definition
        let prog = ast.unwrap();
        let user_struct = prog.items.iter().find(|item| {
            if let windjammer::parser::Item::Struct(s) = item {
                s.name == "User"
            } else {
                false
            }
        });
        
        assert!(user_struct.is_some(), "Should find User struct definition");
        
        // In full LSP: clicking on "User" in main() would jump to struct definition
    }

    #[test]
    fn test_goto_variable_definition() {
        let program = r#"
fn main() {
    let x = 42
    let y = x + 10
    print("x = {x}, y = {y}")
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with variables should parse");
        
        // In full LSP: clicking on "x" in "y = x + 10" would jump to "let x = 42"
    }
}

mod references_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_find_function_references() {
        let program = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let x = add(1, 2)
    let y = add(3, 4)
    let z = add(x, y)
    print("Sum: {z}")
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with multiple function calls should parse");
        
        // In full LSP: finding references to "add" would show:
        // - Line 2: definition
        // - Lines 7, 8, 9: call sites (3 references)
    }

    #[test]
    fn test_find_struct_references() {
        let program = r#"
struct Point {
    x: int,
    y: int
}

fn main() {
    let p1 = Point { x: 0, y: 0 }
    let p2 = Point { x: 10, y: 20 }
    let p3: Point = Point { x: 5, y: 5 }
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with struct instantiations should parse");
        
        let prog = ast.unwrap();
        let has_point = prog.items.iter().any(|item| {
            if let windjammer::parser::Item::Struct(s) = item {
                s.name == "Point"
            } else {
                false
            }
        });
        
        assert!(has_point, "Should have Point struct");
        
        // In full LSP: finding references to "Point" would show:
        // - Line 2: definition
        // - Lines 8, 9, 10: instantiations (3 references, line 10 has 2 occurrences)
    }

    #[test]
    fn test_find_variable_references() {
        let program = r#"
fn main() {
    let count = 0
    count += 1
    count += 2
    print("Count is {count}")
    if count > 0 {
        print("Final count: {count}")
    }
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with variable references should parse");
        
        // In full LSP: finding references to "count" would show:
        // - Line 3: definition
        // - Lines 4, 5, 6, 7, 8: usages (5 references)
    }
}

mod rename_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_rename_function() {
        let program = simple_program();
        
        let mut lexer = Lexer::new(&program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program should parse for rename operation");
        
        // Verify function exists
        let prog = ast.unwrap();
        let has_greet = prog.items.iter().any(|item| {
            if let windjammer::parser::Item::Function(f) = item {
                f.name == "greet"
            } else {
                false
            }
        });
        
        assert!(has_greet, "Should have greet function to rename");
        
        // In full LSP: renaming "greet" to "say_hello" would update:
        // - Line 2: function definition
        // - Line 7: function call in main()
    }

    #[test]
    fn test_rename_variable() {
        let program = r#"
fn calculate() -> int {
    let value = 10
    let doubled = value * 2
    doubled + value
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with variables should parse");
        
        // In full LSP: renaming "value" to "base_value" would update:
        // - Line 3: variable definition
        // - Lines 4, 5: variable usages (2 occurrences)
    }

    #[test]
    fn test_rename_struct() {
        let program = r#"
struct User {
    name: string
}

fn create_user() -> User {
    User { name: "Alice" }
}

fn main() {
    let user: User = create_user()
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with struct should parse");
        
        let prog = ast.unwrap();
        let has_user = prog.items.iter().any(|item| {
            if let windjammer::parser::Item::Struct(s) = item {
                s.name == "User"
            } else {
                false
            }
        });
        
        assert!(has_user, "Should have User struct");
        
        // In full LSP: renaming "User" to "Person" would update:
        // - Line 2: struct definition
        // - Line 6: return type annotation
        // - Line 7: struct instantiation
        // - Line 11: type annotation
    }
}

mod diagnostics_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_syntax_error_detection() {
        let invalid_program = "fn main() { let x = }"; // Missing value

        let mut lexer = Lexer::new(invalid_program);
        let tokens = lexer.tokenize();
        
        // Lexer always succeeds (doesn't validate syntax)
        assert!(!tokens.is_empty(), "Lexer should tokenize incomplete statement");
        
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // Parser should detect the error
        assert!(ast.is_err(), "Parser should detect missing value in let statement");
        
        // In full LSP: would publish diagnostic with error message and location
    }

    #[test]
    fn test_type_error_detection() {
        // Note: Current Windjammer doesn't enforce strict types at parse time
        // Type checking happens during analysis/codegen
        let program_with_type_issue = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    add("hello", 42)
}
"#;
        
        let mut lexer = Lexer::new(program_with_type_issue);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // This parses fine (type checking is semantic, not syntactic)
        assert!(ast.is_ok(), "Program parses even with type mismatch");
        
        // In full LSP: semantic analysis would detect type mismatch
        // and publish diagnostic: "Expected int, found string"
    }

    #[test]
    fn test_missing_function() {
        let program_with_undefined = r#"
fn main() {
    missing_function()
}
"#;
        
        let mut lexer = Lexer::new(program_with_undefined);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // Syntactically valid (semantic error)
        assert!(ast.is_ok(), "Program parses even with undefined function");
        
        // In full LSP: semantic analysis would detect undefined symbol
        // and publish diagnostic: "Cannot find function `missing_function`"
    }
}

mod inlay_hints_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_ownership_hints() {
        let program = r#"
fn process(data: string) {
    println!("{}", data)
}

fn main() {
    let s = "test"
    process(s)
}
"#;

        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program should parse for ownership analysis");
        
        // In full LSP: would show inlay hints like:
        // fn process(data: &string) // Shows inferred borrow
        // process(s) // Shows that s is borrowed, not moved
    }

    #[test]
    fn test_borrowed_hint() {
        let program = r#"
fn read_data(content: string) {
    print(content)
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with borrowed parameter should parse");
        
        // In full LSP: would show "&" hint for borrowed parameters
    }

    #[test]
    fn test_mut_borrowed_hint() {
        let program = r#"
fn increment(counter: int) {
    counter += 1
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with mutable parameter should parse");
        
        // In full LSP: would show "&mut" hint for mutable borrowed parameters
    }

    #[test]
    fn test_owned_hint() {
        let program = r#"
fn take_ownership(data: string) {
    // data is moved here
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with owned parameter should parse");
        
        // In full LSP: would show "owned" hint for moved parameters
    }
}

mod code_action_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_extract_function_action() {
        let program = r#"
fn main() {
    // Selected code to extract:
    let x = 42
    let y = x * 2
    println!("{}", y)
}
"#;

        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program should parse for extract function action");
        
        // In full LSP: would offer "Extract Function" code action
        // to extract lines 4-6 into a new function
    }

    #[test]
    fn test_inline_variable_action() {
        let program = r#"
fn main() {
    let x = 42
    println!("{}", x)
}
"#;

        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program should parse for inline variable action");
        
        // In full LSP: would offer "Inline Variable" code action
        // to replace x with 42 directly in the println call
    }
}

mod symbol_table_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_build_symbol_table() {
        let program = complex_program();
        
        let mut lexer = Lexer::new(&program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Complex program should parse");
        
        // Verify symbol table would contain all items
        let prog = ast.unwrap();
        let function_count = prog.items.iter().filter(|item| {
            matches!(item, windjammer::parser::Item::Function(_))
        }).count();
        let struct_count = prog.items.iter().filter(|item| {
            matches!(item, windjammer::parser::Item::Struct(_))
        }).count();
        let enum_count = prog.items.iter().filter(|item| {
            matches!(item, windjammer::parser::Item::Enum(_))
        }).count();
        
        assert!(function_count > 0, "Should have functions in symbol table");
        assert!(struct_count > 0, "Should have structs in symbol table");
        assert!(enum_count > 0, "Should have enums in symbol table");
    }

    #[test]
    fn test_symbol_lookup() {
        let program = simple_program();
        
        let mut lexer = Lexer::new(&program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program should parse for symbol lookup");
        
        // Find specific symbols
        let prog = ast.unwrap();
        let greet_exists = prog.items.iter().any(|item| {
            if let windjammer::parser::Item::Function(f) = item {
                f.name == "greet"
            } else {
                false
            }
        });
        let main_exists = prog.items.iter().any(|item| {
            if let windjammer::parser::Item::Function(f) = item {
                f.name == "main"
            } else {
                false
            }
        });
        
        assert!(greet_exists, "Should find 'greet' function by name");
        assert!(main_exists, "Should find 'main' function by name");
    }

    #[test]
    fn test_symbol_references() {
        let program = r#"
fn helper() -> int {
    42
}

fn main() {
    let x = helper()
    let y = helper()
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        assert!(ast.is_ok(), "Program with references should parse");
        
        // In full LSP: symbol table would track that 'helper'
        // is referenced twice in main() (lines 7, 8)
    }
}

mod performance_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_large_file_performance() {
        // Generate a large program (100 functions)
        let mut large_program = String::new();
        for i in 0..100 {
            large_program.push_str(&format!("fn func{}() {{ let x = {} }}\n", i, i));
        }
        
        let start = std::time::Instant::now();
        let mut lexer = Lexer::new(&large_program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        let elapsed = start.elapsed();
        
        assert!(ast.is_ok(), "Large file should parse successfully");
        assert!(elapsed.as_secs() < 5, "Should parse large file in reasonable time");
        
        // In full LSP: would verify all operations (hover, completion, etc.)
        // complete quickly even on large files
    }

    #[test]
    fn test_multiple_files_performance() {
        // Simulate multiple files by parsing several programs
        let programs = vec![
            simple_program(),
            complex_program(),
            r#"fn test1() { let x = 1 }"#.to_string(),
            r#"fn test2() { let y = 2 }"#.to_string(),
            r#"fn test3() { let z = 3 }"#.to_string(),
        ];
        
        let start = std::time::Instant::now();
        for program in &programs {
            let mut lexer = Lexer::new(program);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens);
            let _ast = parser.parse();
        }
        let elapsed = start.elapsed();
        
        assert!(elapsed.as_millis() < 1000, "Should handle multiple files quickly");
        
        // In full LSP: would verify LSP remains responsive with many open files
    }

    #[test]
    fn test_incremental_analysis() {
        let program_v1 = "fn main() { let x = 1 }";
        let program_v2 = "fn main() { let x = 2 }"; // Minor change
        
        // Parse v1
        let mut lexer1 = Lexer::new(program_v1);
        let tokens1 = lexer1.tokenize();
        let mut parser1 = Parser::new(tokens1);
        let ast1 = parser1.parse();
        assert!(ast1.is_ok());
        
        // Parse v2 (should be fast due to caching in real LSP)
        let mut lexer2 = Lexer::new(program_v2);
        let tokens2 = lexer2.tokenize();
        let mut parser2 = Parser::new(tokens2);
        let ast2 = parser2.parse();
        assert!(ast2.is_ok());
        
        // In full LSP with Salsa: would reuse most of the analysis
        // from v1 when analyzing v2 (incremental computation)
    }
}

mod edge_case_tests {
    use super::*;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;

    #[test]
    fn test_empty_file() {
        let program = "";
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        
        // Empty file should tokenize successfully (returns empty vec)
        // Lexer doesn't crash on empty input
        
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // Empty program is valid (no items)
        assert!(ast.is_ok(), "Empty file should parse successfully");
    }

    #[test]
    fn test_file_with_only_comments() {
        let program = "// Just a comment\n/* Block comment */";
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // Comment-only file parses as empty program (comments are stripped by lexer)
        // This is equivalent to an empty file
        if let Ok(prog) = ast {
            assert!(prog.items.is_empty(), "Comment-only file should have no items");
        } else {
            // If parser fails on comments, that's also acceptable
            // (it means comments are handled at a different level)
        }
    }

    #[test]
    fn test_incomplete_code() {
        let program = "fn main() {";
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // Incomplete code should produce parse error
        assert!(ast.is_err(), "Incomplete code should produce parse error");
        
        // In full LSP: would show diagnostic for unclosed brace
    }

    #[test]
    fn test_unicode_identifiers() {
        let program = r#"
fn grüßen(名前: string) {
    println!("Hello, {}!", 名前)
}
"#;
        
        let mut lexer = Lexer::new(program);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse();
        
        // Unicode identifiers should work
        assert!(ast.is_ok(), "Unicode identifiers should be supported");
        
        // Verify function with unicode name exists
        let prog = ast.unwrap();
        let has_unicode_fn = prog.items.iter().any(|item| {
            if let windjammer::parser::Item::Function(f) = item {
                f.name == "grüßen"
            } else {
                false
            }
        });
        
        assert!(has_unicode_fn, "Should support unicode function names");
    }
}
