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
))]

//! P3.346: `self.search_query = query` must `.to_string()` when `query` is demoted `&str`
//! and the field is owned `String` — not `.clone()` (asset_browser loop-reuse demotion).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const FIXTURE: &str = r#"
pub struct AssetEntry {
    pub name: string,
    pub visible: bool,
}

pub struct AssetBrowser {
    pub search_query: string,
    pub assets: Vec<AssetEntry>,
}

impl AssetBrowser {
    pub fn search(self, query: string) {
        self.search_query = query
        for i in 0..self.assets.len() {
            self.assets[i].visible = self.assets[i].name.contains(query)
        }
    }
}
"#;

#[test]
fn module_file_demoted_str_field_assign_must_to_string() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), FIXTURE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    assert!(
        generated.contains("query: &str") || generated.contains("query: &String"),
        "P3.346: expected demoted query formal:\n{generated}"
    );
    assert!(
        !generated.contains("search_query = query.clone()"),
        "P3.346: owned String field assign from demoted str must not use .clone():\n{generated}"
    );
    assert!(
        generated.contains("search_query = query.to_string()"),
        "P3.346: expected query.to_string() for owned field:\n{generated}"
    );
}
