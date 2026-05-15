//! TDD Test: Methods that modify self fields should be &mut self even if user wrote &self
//!
//! When a user explicitly writes `&self` but the method modifies self fields,
//! the compiler should upgrade it to `&mut self` automatically.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ref_self_upgrade_to_mut() {
    // User wrote &self but method modifies field - should be upgraded to &mut self
    let code = r#"
pub struct Panel {
    visible: bool,
}

impl Panel {
    pub fn hide(&self) {
        self.visible = false
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Compile error:\n{}", err);
    }

    assert!(success, "Compilation should succeed");

    // Should be upgraded to &mut self
    assert!(
        generated.contains("fn hide(&mut self)"),
        "hide should be &mut self (upgraded from &self). Generated:\n{}",
        generated
    );
}
