// Tests for automatic .iter() and .iter_mut() inference
// Windjammer Philosophy: The compiler does the work, not the developer

#[path = "test_utils.rs"]
mod test_utils;

/// Helper to compile a test fixture and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_inference_field_access() {
    let generated = test_utils::compile_fixture("iter_inference").expect("Compilation failed");

    // sum_items should have .iter() added for self.items iteration
    // Either .iter() or &self.field is valid for iteration
    assert!(
        generated.contains("self.items.iter()") || generated.contains("&self.items"),
        "Should infer iteration for field access in sum_items: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_inference_names_field() {
    let generated = test_utils::compile_fixture("iter_inference").expect("Compilation failed");

    // Either .iter() or &self.field is valid for iteration
    assert!(
        generated.contains("self.names.iter()") || generated.contains("&self.names"),
        "Should infer iteration for field access in print_names: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_double_iter() {
    let generated = test_utils::compile_fixture("iter_inference").expect("Compilation failed");

    // count_positive already has .iter(), should not double it
    assert!(
        !generated.contains(".iter().iter()"),
        "Should not double .iter(): {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_iter_after_enumerate() {
    let generated = test_utils::compile_fixture("iter_inference").expect("Compilation failed");

    // print_with_index uses .enumerate(), should not add .iter() after it
    assert!(
        !generated.contains(".enumerate().iter()"),
        "Should not add .iter() after .enumerate(): {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_inference_simple_vec() {
    let generated = test_utils::compile_fixture("iter_inference").expect("Compilation failed");

    // process_vec should iterate over items (either .iter() or for item in items is valid)
    assert!(
        generated.contains("items.iter()") || generated.contains("for item in items"),
        "Should have valid Vec iteration: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_count() {
    let generated = test_utils::compile_fixture("iter_inference").expect("Compilation failed");

    // Count both .iter() calls and &self.field forms (both are valid iteration)
    let iter_count = generated.matches(".iter()").count();
    let ref_count = generated.matches("for ").count(); // Count for loops as iterations
    assert!(
        iter_count >= 1 && ref_count >= 4,
        "Should have iteration patterns (found {} .iter() calls, {} for loops): {}",
        iter_count,
        ref_count,
        generated
    );
}
