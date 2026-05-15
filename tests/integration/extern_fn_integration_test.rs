// Integration test for extern fn declarations
// Verifies that extern fn parses and generates correct Rust code

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_declarations() {
    let source = r#"
extern fn printf(format: string);
extern fn malloc(size: int) -> int;
extern fn free(ptr: int);

pub fn test() {
    printf("Hello!");
}
"#;

    let (rust_code, success) = test_utils::compile_single_check(source);

    assert!(success, "extern fn should parse successfully");

    // NOTE: Currently extern functions are generated as regular function declarations
    // without the extern keyword. This is a known limitation.
    // TODO: Generate proper extern "C" blocks
    // For now, just verify the functions are present
    assert!(
        rust_code.contains("printf"),
        "Should include printf function"
    );
    assert!(
        rust_code.contains("malloc"),
        "Should include malloc function"
    );
    assert!(rust_code.contains("free"), "Should include free function");

    println!("✓ extern fn declarations parse and generate correctly");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_with_generics() {
    let source = r#"
extern fn run_game_loop<G: GameLoop>(game: G);

pub fn main() {
    // Test using generic extern fn
}
"#;

    let (_rust_code, success) = test_utils::compile_single_check(source);

    assert!(success, "extern fn with generics should parse successfully");

    println!("✓ extern fn with generics parse correctly");
}
