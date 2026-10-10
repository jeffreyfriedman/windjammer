#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! Copy u32 from a helper/FFI return, assigned to a field and reused
//! in `w * h`, must not emit `w.clone()` / `h.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn get_screen_width() -> u32 {
    1280
}

pub fn get_screen_height() -> u32 {
    720
}

pub struct Renderer {
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Renderer {
    pub fn init_gpu(self) {
        let w = get_screen_width()
        let h = get_screen_height()
        self.screen_width = w
        self.screen_height = h
        let pixel_count = w * h
        let _ = pixel_count
    }
}
"#;

#[test]
fn module_file_ffi_u32_return_must_not_clone_on_assign() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("w.clone()") || rs.contains("h.clone()");
    assert!(
        !bad,
        "RED Copy u32 helper return cloned on assign/reuse:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
