// Tests for proper handling of & and &mut references

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Helper to compile a test fixture and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_ref_no_double_borrow() {
    let rust_code = test_utils::compile_fixture("mut_ref_test").expect("Compilation failed");

    // Should generate `modify(&mut vec)`, NOT `&mut &mut vec`
    assert!(
        rust_code.contains("modify(&mut vec)"),
        "Generated code should have single &mut, got: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("&mut &mut"),
        "Generated code should NOT have double &mut, got: {}",
        rust_code
    );

    // Should also have read(&vec) not &&vec
    assert!(
        rust_code.contains("read(&vec)"),
        "Generated code should have single &, got: {}",
        rust_code
    );
    assert!(
        !rust_code.contains("&&vec") && !rust_code.contains("& &vec"),
        "Generated code should NOT have double &, got: {}",
        rust_code
    );
}
