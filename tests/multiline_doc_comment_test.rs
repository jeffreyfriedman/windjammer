/// Bug #19: Multi-line doc comments break parser
///
/// When multiple consecutive `///` doc comment lines appear before
/// a function, struct, or other item, the parser fails with:
/// "Expected Fn, got DocComment(...)"
///
/// This is essential for writing clear, comprehensive documentation.
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[allow(dead_code)]
fn compile_wj_code(code: &str) -> Result<String, String> {
    let temp_dir = tempdir().map_err(|e| e.to_string())?;
    let test_file = temp_dir.path().join("test.wj");

    fs::write(&test_file, code).map_err(|e| e.to_string())?;

    let output = Command::new("cargo")
        .args(["run", "--release", "--", "build", "--no-cargo"])
        .arg(&test_file)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| e.to_string())?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstderr: {}\nstdout: {}",
            stderr, stdout
        ));
    }

    Ok(stdout.to_string())
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_two_line_doc_comment() {
    let code = r#"
        /// This is the first line
        /// This is the second line
        fn test_function() -> int {
            42
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Two-line doc comment should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_three_line_doc_comment() {
    let code = r#"
        /// First line of documentation
        /// Second line of documentation
        /// Third line of documentation
        fn test_function() -> int {
            42
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Three-line doc comment should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_doc_comment_on_struct() {
    let code = r#"
        /// A test structure
        /// With multiple lines of documentation
        struct TestStruct {
            value: int,
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Multi-line doc comment on struct should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_doc_comment_on_enum() {
    let code = r#"
        /// An enumeration type
        /// With multiple lines of docs
        enum TestEnum {
            Variant1,
            Variant2,
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Multi-line doc comment on enum should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_doc_comment_on_impl_method() {
    let code = r#"
        struct TestStruct {
            value: int,
        }
        
        impl TestStruct {
            /// Gets the value
            /// Returns an integer
            pub fn get_value(&self) -> int {
                self.value
            }
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Multi-line doc comment on impl method should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_voxel_mesh_pattern() {
    let code = r#"
        struct Mesh {
            vertices: Vec<int>,
        }
        
        impl Mesh {
            /// Add a quad (2 triangles) to the mesh
            /// Vertices should be in counter-clockwise order
            pub fn add_quad(&mut self, v0: int, v1: int, v2: int, v3: int) {
                self.vertices.push(v0);
                self.vertices.push(v1);
                self.vertices.push(v2);
                self.vertices.push(v3);
            }
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Voxel mesh pattern (actual bug case) should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_doc_comment_with_empty_line() {
    let code = r#"
        /// First paragraph of documentation
        ///
        /// Second paragraph after empty line
        fn test_function() -> int {
            42
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Doc comment with empty line should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_many_line_doc_comment() {
    let code = r#"
        /// Line 1
        /// Line 2
        /// Line 3
        /// Line 4
        /// Line 5
        /// Line 6
        /// Line 7
        /// Line 8
        /// Line 9
        /// Line 10
        fn test_function() -> int {
            42
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Many-line doc comment should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_doc_comment_with_special_chars() {
    let code = r#"
        /// This function returns `true` if valid
        /// It uses special chars: *, #, @, !, etc.
        fn test_function() -> bool {
            true
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Doc comment with special chars should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_multiple_items_with_multiline_docs() {
    let code = r#"
        /// First function
        /// With docs
        fn first() -> int {
            1
        }
        
        /// Second function
        /// Also with docs
        fn second() -> int {
            2
        }
        
        /// Third function
        /// More docs
        fn third() -> int {
            3
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Multiple items with multi-line docs should compile. Error: {:?}",
        result.err()
    );
}

