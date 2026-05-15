//! E0599 / Vec<T>: Clone — generic `impl Foo<T>` that clones `self.dense` must emit `T: Clone`.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn wj_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_impl_clone_dense_adds_t_clone_bound() {
    let source = r#"
pub struct ComponentArray<T> {
    dense: Vec<T>,
}

impl ComponentArray<T> {
    pub fn get(self, i: i32) -> Option<T> {
        return Some(self.dense[i as usize].clone())
    }

    pub fn iter(self) -> Vec<T> {
        return self.dense.clone()
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("gen_clone_dense.wj");
    let output_dir = temp_dir.path().join("out");
    fs::write(&input_path, source).unwrap();

    let output = Command::new(wj_bin())
        .args([
            "build",
            input_path.to_str().unwrap(),
            "--no-cargo",
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("wj build");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rs = fs::read_to_string(output_dir.join("gen_clone_dense.rs")).expect("read rs");
    assert!(
        rs.contains("where") && rs.contains("T: Clone"),
        "expected `where T: Clone` in:\n{}",
        rs
    );
}
