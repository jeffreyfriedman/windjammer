//! Cross-file trait method signatures: multipass build must match impl receivers to inferred trait
//! receivers (E0053 / E0186 regressions).

#[path = "integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Trait in one file, impl in another — impl mutates `self`, so Rust must use `&mut self` on both.
#[test]
fn test_trait_method_signature_match_multi_file() {
    let mut test = MultiFileTest::new();

    test.add_file(
        "trait_defs.wj",
        r#"
pub trait MyTrait {
    fn method(self, x: i32)
}
"#,
    );

    test.add_file(
        "holder.wj",
        r#"
use trait_defs::MyTrait

pub struct MyStruct {
    value: i32,
}

impl MyTrait for MyStruct {
    fn method(self, x: i32) {
        self.value = x
    }
}
"#,
    );

    let generated = test.compile().expect("multipass compile");
    let holder = generated.get("holder.rs").expect("holder.rs");
    let defs = generated.get("trait_defs.rs").expect("trait_defs.rs");
    let sig = "fn method(&mut self, x: i32)";
    assert!(
        holder.contains(sig),
        "impl must match inferred trait receiver (E0053 if &self); holder.rs:\n{holder}"
    );
    assert!(
        defs.contains(sig),
        "trait module should agree after cross-file inference; trait_defs.rs:\n{defs}"
    );
}
