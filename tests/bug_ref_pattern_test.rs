// BUG: Parser doesn't support `ref` keyword in match patterns
//
// DISCOVERED DURING: Phase 3 - SVO Octree implementation
//
// PROBLEM:
// Windjammer source: `match self.children { Some(ref c) => c.len(), None => 0 }`
// Parser fails: "Unexpected token: ref"
//
// ROOT CAUSE:
// Parser's match arm pattern handler doesn't recognize `ref` keyword
//
// FIX:
// Add support for `ref` and `ref mut` in pattern matching
// Pattern syntax: `ref <identifier>` and `ref mut <identifier>`

use windjammer::compile_to_rust;

#[test]
fn test_ref_pattern_in_match_some() {
    let source = r#"
struct Container {
    data: Option<Vec<i32>>,
}

impl Container {
    fn len(self) -> i32 {
        match self.data {
            Some(ref vec) => vec.len() as i32,
            None => 0,
        }
    }
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    // Should preserve ref keyword
    assert!(
        rust_code.contains("Some(ref vec)") || rust_code.contains("Some(ref vec )"),
        "Match pattern should preserve 'ref' keyword"
    );
}

#[test]
fn test_ref_mut_pattern_in_match() {
    let source = r#"
struct Container {
    data: Option<Vec<i32>>,
}

impl Container {
    fn push(self, value: i32) {
        match self.data {
            Some(ref mut vec) => vec.push(value),
            None => {},
        }
    }
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    // Should preserve ref mut keyword
    assert!(
        rust_code.contains("Some(ref mut vec)"),
        "Match pattern should preserve 'ref mut' keyword"
    );
}

#[test]
fn test_ref_pattern_with_tuple() {
    let source = r#"
fn process(data: Option<(i32, i32)>) -> i32 {
    match data {
        Some(ref tuple) => tuple.0 + tuple.1,
        None => 0,
    }
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    assert!(
        rust_code.contains("Some(ref tuple)"),
        "Tuple pattern should preserve 'ref'"
    );
}

#[test]
fn test_ref_pattern_octree_use_case() {
    // The exact pattern from SVO Octree that failed
    let source = r#"
struct OctreeNode {
    children: Option<Vec<OctreeNode>>,
}

impl OctreeNode {
    fn child_count(self) -> i32 {
        match self.children {
            Some(ref c) => c.len() as i32,
            None => 0,
        }
    }
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    assert!(
        rust_code.contains("Some(ref c)"),
        "Octree pattern should work with 'ref'"
    );
}

#[test]
fn test_ref_pattern_nested_struct() {
    let source = r#"
struct Inner {
    value: i32,
}

struct Outer {
    inner: Option<Inner>,
}

impl Outer {
    fn get_value(self) -> i32 {
        match self.inner {
            Some(ref i) => i.value,
            None => 0,
        }
    }
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    assert!(
        rust_code.contains("Some(ref i)"),
        "Nested struct pattern should work with 'ref'"
    );
}

#[test]
fn test_regular_pattern_without_ref_still_works() {
    // Ensure we don't break existing patterns
    let source = r#"
fn test(data: Option<i32>) -> i32 {
    match data {
        Some(x) => x,
        None => 0,
    }
}
"#;

    let result = compile_to_rust(source, "test.wj");
    assert!(result.is_ok(), "Compilation should succeed");
    
    let rust_code = result.unwrap().generated_code;
    println!("Generated:\n{}", rust_code);
    
    // Should work without ref
    assert!(
        rust_code.contains("Some(x)"),
        "Regular pattern without ref should still work"
    );
}
