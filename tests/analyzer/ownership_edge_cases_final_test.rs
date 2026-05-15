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

//! TDD: Dogfooding E0382 fixes — string return type must not force Owned when the param is not returned,
//! and passthrough must not apply unrelated `contains`/`len` registry overloads to `str` parameters.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_string_lookup_helper_and_get_no_e0382() {
    // Same pattern as localization_manager: `key` is returned from `get` but passed twice to a helper
    // that only compares — helper must take `&str`, not `String`, or the first call moves `key`.
    let source = r#"
struct Row {
    k: string,
    v: string,
}

pub struct Table {
    rows: Vec<Row>,
}

impl Table {
    fn find_value(self, key: string) -> string {
        let mut i = 0
        while i < self.rows.len() {
            if self.rows[i].k == key {
                return self.rows[i].v.clone()
            }
            i = i + 1
        }
        "".to_string()
    }

    pub fn get(self, key: string) -> string {
        let first = self.find_value(key)
        if first.len() > 0 {
            return first
        }
        let second = self.find_value("en".to_string())
        if second.len() > 0 {
            return second
        }
        key
    }
}
"#;

    let rust = test_utils::compile_single_result(source)
        .unwrap_or_else(|e| panic!("compile failed: {}", e));

    assert!(
        rust.contains("fn find_value(&self, key: &str)")
            || rust.contains("fn find_value(&self,key: &str)"),
        "lookup helper should borrow string key; got:\n{}",
        rust
    );
    // `get` may keep `key: String` when the parameter is returned as the fallback — that is fine
    // as long as callees borrow (E0382 comes from moving `key` into an owned-parameter helper twice).
    assert!(
        rust.contains("self.find_value(&key)"),
        "both lookups must borrow key, not move it; got:\n{}",
        rust
    );
}

#[test]
fn test_hashset_contains_str_param_not_owned_from_foreign_contains_sig() {
    // `is_registered` only passes `name` to `contains` — unrelated `contains(Vec3)` in the registry
    // must not force `name: String` (moves on `set_active`'s second use of `name`).
    let source = r#"
use std::collections::HashSet

pub struct Scenes {
    names: HashSet<String>,
}

impl Scenes {
    pub fn is_registered(self, name: str) -> bool {
        self.names.contains(name)
    }

    pub fn set_active(self, name: str) {
        if self.is_registered(name) {
            self.names.insert(name.to_string())
        }
    }
}
"#;

    let rust = test_utils::compile_single_result(source)
        .unwrap_or_else(|e| panic!("compile failed: {}", e));

    // HashSet::contains accepts &str or &String; compiler may emit &String
    let sig_ok = rust.contains("fn is_registered(&self, name: &str)")
        || rust.contains("fn is_registered(&self,name: &str)")
        || rust.contains("name: &String");
    assert!(
        sig_ok,
        "is_registered should borrow the name (e.g. &str or &String); got:\n{}",
        rust
    );
}
