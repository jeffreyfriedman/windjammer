#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile Windjammer code and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_chain_string_conversion() {
    let code = r#"
    struct MenuItem {
        name: string,
        shortcut: string,
    }
    impl MenuItem {
        pub fn new(name: string) -> MenuItem {
            MenuItem {
                name,
                shortcut: "",
            }
        }
        
        pub fn shortcut(self, shortcut: string) -> MenuItem {
            self.shortcut = shortcut
            self
        }
    }
    pub fn create_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::new("New Project").shortcut("Ctrl+N"),
            MenuItem::new("Save").shortcut("Ctrl+S"),
        ]
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // shortcut() expects String, so "Ctrl+N" should be converted
    assert!(
        generated.contains(r#"shortcut("Ctrl+N".to_string())"#),
        "Method chain should convert string literals: {}",
        generated
    );
}
