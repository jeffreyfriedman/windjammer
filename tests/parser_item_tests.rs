//! Comprehensive Parser Item Tests
//!
//! These tests verify that the parser correctly parses all item types.
//! Items are top-level declarations: functions, structs, enums, traits, impls, etc.

use windjammer::lexer::Lexer;
use windjammer::parser::ast::*;
use windjammer::parser_impl::Parser;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn parse_program(input: &str) -> Program<'_> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().expect("Failed to parse program")
}

fn first_item(input: &str) -> Item<'_> {
    let program = parse_program(input);
    program.items.into_iter().next().expect("No items parsed")
}

// ============================================================================
// FUNCTION DECLARATIONS
// ============================================================================

#[test]
fn test_fn_simple() {
    let item = first_item("fn foo() { }");
    assert!(matches!(item, Item::Function { .. }));
}

#[test]
fn test_fn_with_params() {
    let item = first_item("fn foo(x: i32, y: i32) { }");
    if let Item::Function { decl, .. } = item {
        assert_eq!(decl.parameters.len(), 2);
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_fn_with_return_type() {
    let item = first_item("fn foo() -> i32 { 42 }");
    if let Item::Function { decl, .. } = item {
        assert!(decl.return_type.is_some());
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_fn_with_body() {
    let item = first_item("fn foo() { let x = 1; x + 1 }");
    if let Item::Function { decl, .. } = item {
        assert!(!decl.body.is_empty());
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_fn_pub() {
    let item = first_item("pub fn foo() { }");
    if let Item::Function { decl, .. } = item {
        assert!(decl.is_pub);
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_fn_async() {
    let item = first_item("async fn foo() { }");
    if let Item::Function { decl, .. } = item {
        assert!(decl.is_async);
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_fn_generic() {
    let item = first_item("fn foo<T>(x: T) -> T { x }");
    if let Item::Function { decl, .. } = item {
        assert!(!decl.type_params.is_empty());
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_fn_with_self() {
    let code = r#"
    impl Foo {
        fn bar(&self) { }
    }
    "#;
    let program = parse_program(code);
    assert!(!program.items.is_empty());
}

#[test]
fn test_fn_with_mut_self() {
    let code = r#"
    impl Foo {
        fn bar(&mut self) { }
    }
    "#;
    let program = parse_program(code);
    assert!(!program.items.is_empty());
}

// ============================================================================
// STRUCT DECLARATIONS
// ============================================================================

#[test]
fn test_struct_empty() {
    let item = first_item("struct Empty { }");
    assert!(matches!(item, Item::Struct { .. }));
}

#[test]
fn test_struct_with_fields() {
    let item = first_item("struct Point { x: i32, y: i32 }");
    if let Item::Struct { decl, .. } = item {
        assert_eq!(decl.fields.len(), 2);
    } else {
        panic!("Expected Struct");
    }
}

#[test]
fn test_struct_pub() {
    let item = first_item("pub struct Point { x: i32, y: i32 }");
    if let Item::Struct { decl, .. } = item {
        assert!(decl.is_pub);
    } else {
        panic!("Expected Struct");
    }
}

#[test]
fn test_struct_generic() {
    let item = first_item("struct Container<T> { value: T }");
    if let Item::Struct { decl, .. } = item {
        assert!(!decl.type_params.is_empty());
    } else {
        panic!("Expected Struct");
    }
}

#[test]
fn test_struct_with_decorator() {
    let item = first_item("@auto struct Point { x: i32, y: i32 }");
    if let Item::Struct { decl, .. } = item {
        assert!(!decl.decorators.is_empty());
    } else {
        panic!("Expected Struct");
    }
}

#[test]
fn test_struct_with_derive() {
    let item = first_item("@derive(Clone, Debug) struct Point { x: i32, y: i32 }");
    if let Item::Struct { decl, .. } = item {
        assert!(!decl.decorators.is_empty());
    } else {
        panic!("Expected Struct");
    }
}

#[test]
fn test_struct_nested_type() {
    let item = first_item("struct Container { items: Vec<String> }");
    assert!(matches!(item, Item::Struct { .. }));
}

// ============================================================================
// ENUM DECLARATIONS
// ============================================================================

#[test]
fn test_enum_simple() {
    let item = first_item("enum Color { Red, Green, Blue }");
    assert!(matches!(item, Item::Enum { .. }));
}

#[test]
fn test_enum_with_data() {
    let item = first_item("enum Option<T> { Some(T), None }");
    if let Item::Enum { decl, .. } = item {
        assert_eq!(decl.variants.len(), 2);
    } else {
        panic!("Expected Enum");
    }
}

#[test]
fn test_enum_struct_variants() {
    let item = first_item("enum Message { Text { content: String }, Image { url: String } }");
    assert!(matches!(item, Item::Enum { .. }));
}

#[test]
fn test_enum_pub() {
    let item = first_item("pub enum Color { Red, Green, Blue }");
    if let Item::Enum { decl, .. } = item {
        assert!(decl.is_pub);
    } else {
        panic!("Expected Enum");
    }
}

#[test]
fn test_enum_generic() {
    let item = first_item("enum Result<T, E> { Ok(T), Err(E) }");
    if let Item::Enum { decl, .. } = item {
        assert!(!decl.type_params.is_empty());
    } else {
        panic!("Expected Enum");
    }
}

// ============================================================================
// IMPL BLOCKS
// ============================================================================

#[test]
fn test_impl_simple() {
    let code = r#"
    impl Point {
        fn new() -> Point {
            Point { x: 0, y: 0 }
        }
    }
    "#;
    let item = first_item(code);
    assert!(matches!(item, Item::Impl { .. }));
}

#[test]
fn test_impl_with_methods() {
    let code = r#"
    impl Point {
        fn new() -> Point { Point { x: 0, y: 0 } }
        fn x(&self) -> i32 { self.x }
        fn set_x(&mut self, x: i32) { self.x = x }
    }
    "#;
    let item = first_item(code);
    if let Item::Impl { block, .. } = item {
        assert_eq!(block.functions.len(), 3);
    } else {
        panic!("Expected Impl");
    }
}

#[test]
fn test_impl_trait() {
    let code = r#"
    impl Display for Point {
        fn fmt(&self) -> String {
            "point"
        }
    }
    "#;
    let item = first_item(code);
    if let Item::Impl { block, .. } = item {
        assert!(block.trait_name.is_some());
    } else {
        panic!("Expected Impl");
    }
}

#[test]
fn test_impl_generic() {
    let code = r#"
    impl<T> Container<T> {
        fn new(value: T) -> Container<T> {
            Container { value: value }
        }
    }
    "#;
    let item = first_item(code);
    assert!(matches!(item, Item::Impl { .. }));
}

// ============================================================================
// TRAIT DECLARATIONS
// ============================================================================

#[test]
fn test_trait_simple() {
    // Trait methods with default implementations
    let code = r#"
    trait Drawable {
        fn draw(&self) { }
    }
    "#;
    let item = first_item(code);
    assert!(matches!(item, Item::Trait { .. }));
}

#[test]
fn test_trait_with_methods() {
    let code = r#"
    trait Animal {
        fn speak(&self) { }
        fn name(&self) -> String { "unknown" }
    }
    "#;
    let item = first_item(code);
    if let Item::Trait { decl, .. } = item {
        assert_eq!(decl.methods.len(), 2);
    } else {
        panic!("Expected Trait");
    }
}

#[test]
fn test_trait_with_default_impl() {
    let code = r#"
    trait Greet {
        fn greet(&self) {
            println!("Hello!")
        }
    }
    "#;
    let item = first_item(code);
    assert!(matches!(item, Item::Trait { .. }));
}

#[test]
fn test_trait_generic() {
    // Trait methods need default implementations in Windjammer
    let code = r#"
    trait Into<T> {
        fn into(self) -> T { self }
    }
    "#;
    let item = first_item(code);
    if let Item::Trait { decl, .. } = item {
        assert!(!decl.generics.is_empty());
    } else {
        panic!("Expected Trait");
    }
}

// ============================================================================
// USE STATEMENTS
// ============================================================================

#[test]
fn test_use_simple() {
    let item = first_item("use std::io");
    assert!(matches!(item, Item::Use { .. }));
}

#[test]
fn test_use_nested() {
    let item = first_item("use std::collections::HashMap");
    assert!(matches!(item, Item::Use { .. }));
}

#[test]
fn test_use_glob() {
    let item = first_item("use std::io::*");
    assert!(matches!(item, Item::Use { .. }));
}

#[test]
fn test_use_alias() {
    let item = first_item("use std::collections::HashMap as Map");
    assert!(matches!(item, Item::Use { .. }));
}

#[test]
fn test_use_multiple() {
    let item = first_item("use std::io::{Read, Write}");
    assert!(matches!(item, Item::Use { .. }));
}

// ============================================================================
// MOD DECLARATIONS
// ============================================================================

#[test]
fn test_mod_simple() {
    let item = first_item("mod utils");
    assert!(matches!(item, Item::Mod { .. }));
}

#[test]
fn test_mod_inline() {
    let code = r#"
    mod utils {
        fn helper() { }
    }
    "#;
    let item = first_item(code);
    assert!(matches!(item, Item::Mod { .. }));
}

#[test]
fn test_mod_pub() {
    let item = first_item("pub mod utils");
    if let Item::Mod { is_public, .. } = item {
        assert!(is_public);
    } else {
        panic!("Expected Mod");
    }
}

// ============================================================================
// CONST AND STATIC DECLARATIONS
// ============================================================================

#[test]
fn test_const_simple() {
    let item = first_item("const MAX: i32 = 100");
    assert!(matches!(item, Item::Const { .. }));
}

#[test]
fn test_const_with_name() {
    let item = first_item("const MAX: i32 = 100");
    if let Item::Const { name, .. } = item {
        assert_eq!(name, "MAX");
    } else {
        panic!("Expected Const");
    }
}

#[test]
fn test_static_simple() {
    let item = first_item("static COUNTER: i32 = 0");
    assert!(matches!(item, Item::Static { .. }));
}

#[test]
fn test_static_mut() {
    let item = first_item("static mut COUNTER: i32 = 0");
    if let Item::Static { mutable, .. } = item {
        assert!(mutable);
    } else {
        panic!("Expected Static");
    }
}

// ============================================================================
// MULTIPLE ITEMS
// ============================================================================

#[test]
fn test_multiple_functions() {
    let code = r#"
    fn foo() { }
    fn bar() { }
    fn baz() { }
    "#;
    let program = parse_program(code);
    assert_eq!(program.items.len(), 3);
}

#[test]
fn test_struct_and_impl() {
    let code = r#"
    struct Point { x: i32, y: i32 }
    
    impl Point {
        fn new(x: i32, y: i32) -> Point {
            Point { x: x, y: y }
        }
    }
    "#;
    let program = parse_program(code);
    assert_eq!(program.items.len(), 2);
}

#[test]
fn test_trait_and_impl() {
    let code = r#"
    trait Drawable {
        fn draw(&self) { }
    }
    
    struct Circle { radius: f32 }
    
    impl Drawable for Circle {
        fn draw(&self) { }
    }
    "#;
    let program = parse_program(code);
    assert_eq!(program.items.len(), 3);
}

// ============================================================================
// COMPLEX ITEMS
// ============================================================================

#[test]
fn test_generic_struct_with_bounds() {
    let code = "struct Container<T: Clone> { value: T }";
    let item = first_item(code);
    assert!(matches!(item, Item::Struct { .. }));
}

#[test]
fn test_impl_with_where_clause() {
    let code = r#"
    impl<T> Container<T> where T: Clone {
        fn clone_value(&self) -> T {
            self.value.clone()
        }
    }
    "#;
    let item = first_item(code);
    assert!(matches!(item, Item::Impl { .. }));
}

#[test]
fn test_async_trait_method() {
    // Async trait methods need default implementations
    let code = r#"
    trait AsyncReader {
        async fn read(&self) -> String { "data" }
    }
    "#;
    let item = first_item(code);
    assert!(matches!(item, Item::Trait { .. }));
}
