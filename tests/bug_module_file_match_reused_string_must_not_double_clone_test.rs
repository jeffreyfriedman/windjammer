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

//! a `string` passed into a call and also moved in a later `match`
//! arm must not `.clone().clone()`. One clone into the call is enough.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Asset {
    pub name: string,
}

pub struct Loader {
    pub error_count: int,
}

impl Loader {
    pub fn load(self, name: string, path: string, size: int) -> Result<Asset, string> {
        if size == 0 {
            return Err("empty")
        }
        Ok(Asset { name: name })
    }

    pub fn load_batch(self, assets: Vec<(string, string, int)>) -> int {
        let mut failures = Vec::new()
        for entry in assets {
            let (name, path, size) = entry
            match self.load(name, path, size) {
                Ok(_asset) => {}
                Err(error) => {
                    self.error_count = self.error_count + 1
                    failures.push(name)
                    let _kept = error
                }
            }
        }
        failures.len()
    }
}
"#;

#[test]
fn module_file_match_reused_string_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED match-reused string double-cloned into load:\n{rs}"
    );
    let _ = test.cargo_check();
}
