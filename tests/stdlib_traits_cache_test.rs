//! Verify stdlib trait bootstrap is cached across Analyzer constructions.

use std::collections::HashSet;
use std::sync::Arc;
use windjammer::analyzer::Analyzer;

#[test]
fn test_for_library_pass_reuses_cached_drop_trait_analysis() {
    let copy = Arc::new(HashSet::new());
    let fields = Arc::new(std::collections::HashMap::new());
    let paths = Arc::new(std::collections::HashMap::new());

    let a1 = Analyzer::for_library_pass(copy.clone(), fields.clone(), paths.clone());
    let a2 = Analyzer::for_library_pass(copy, fields, paths);

    let drop1 = a1
        .analyzed_trait_methods
        .get("Drop")
        .and_then(|m| m.get("drop"));
    let drop2 = a2
        .analyzed_trait_methods
        .get("Drop")
        .and_then(|m| m.get("drop"));

    assert!(drop1.is_some(), "Drop::drop should be analyzed");
    assert!(drop2.is_some(), "Drop::drop should be cached on second pass");

    let self_mode1 = drop1.unwrap().inferred_ownership.get("self").copied();
    let self_mode2 = drop2.unwrap().inferred_ownership.get("self").copied();
    assert_eq!(
        self_mode1, self_mode2,
        "cached bootstrap should produce identical Drop::drop ownership"
    );
}
