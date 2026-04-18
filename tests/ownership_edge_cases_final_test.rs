//! TDD: Dogfooding E0382 fixes — string return type must not force Owned when the param is not returned,
//! and passthrough must not apply unrelated `contains`/`len` registry overloads to `str` parameters.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_binary())
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let rust_file = out_dir.join("test.rs");
    fs::read_to_string(rust_file).map_err(|e| e.to_string())
}

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

    let rust = compile_to_rust(source).unwrap_or_else(|e| panic!("compile failed: {}", e));

    assert!(
        rust.contains("fn find_value(&self, key: &str)") || rust.contains("fn find_value(&self,key: &str)"),
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

    let rust = compile_to_rust(source).unwrap_or_else(|e| panic!("compile failed: {}", e));

    assert!(
        rust.contains("fn is_registered(&self, name: &str)")
            || rust.contains("fn is_registered(&self,name: &str)"),
        "is_registered should take &str; got:\n{}",
        rust
    );
}
