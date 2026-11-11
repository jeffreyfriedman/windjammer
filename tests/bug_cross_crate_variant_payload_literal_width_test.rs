#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! NUM-FIELD-DECLARED across crates: a literal passed to a tuple variant declared in a dependency crate
//! (`Value::Int64(99)` where the variant declares `i64`) has the declared payload width. The dependency's
//! `metadata.json` carries its variant payload types; without them the literal was typed `99_i32` (E0308).
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cross_crate_variant_payload_literal_takes_the_declared_width() {
    let tmp = TempDir::new().expect("tempdir");
    let pkg_src = tmp.path().join("types_src");
    fs::create_dir_all(&pkg_src).unwrap();
    fs::write(pkg_src.join("value.wj"), "pub enum Value {\n    Int64(i64),\n    Small(u8),\n    Text(string),\n}\n").unwrap();
    let pkg_gen = tmp.path().join("types_gen");
    let pkg_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", pkg_src.to_str().unwrap(), "--output", pkg_gen.to_str().unwrap(), "--library", "--no-cargo", "--module-file"])
        .output()
        .expect("library build");
    assert!(pkg_build.status.success(), "library build failed:\n{}", String::from_utf8_lossy(&pkg_build.stderr));
    let metadata_path = pkg_gen.join("metadata.json");
    let meta = fs::read_to_string(&metadata_path).expect("metadata.json");
    assert!(meta.contains("Value::Int64"), "metadata.json lists the variant payloads:\n{meta}");

    let app_src = tmp.path().join("app_src");
    fs::create_dir_all(&app_src).unwrap();
    fs::write(
        app_src.join("part.wj"),
        r#"use types_pkg::Value

pub struct Version {
    pub n: u64,
}

impl Version {
    pub fn new(n: u64) -> Version {
        Version { n: n }
    }
}

pub struct Part {
    pub key: u64,
    pub value: Value,
    pub v: Version,
}

impl Part {
    pub fn new(key: u64, value: Value, v: Version) -> Part {
        Part { key: key, value: value, v: v }
    }
}

pub fn make() -> Part {
    let key = 42
    let small = Value::Small(7)
    Part::new(key, Value::Int64(99), Version::new(10))
}
"#,
    )
    .unwrap();
    let app_gen = tmp.path().join("app_gen");
    let app_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", app_src.join("part.wj").to_str().unwrap(), "--output", app_gen.to_str().unwrap(), "--no-cargo"])
        .args(["--metadata", &format!("types_pkg={}", metadata_path.display())])
        .output()
        .expect("app build");
    assert!(app_build.status.success(), "app build failed:\n{}", String::from_utf8_lossy(&app_build.stderr));
    let generated = fs::read_to_string(app_gen.join("part.rs")).expect("part.rs");
    assert!(!generated.contains("99_i32") && !generated.contains("7_i32"), "payload literals take the declared widths:\n{generated}");
    assert!(generated.contains("Value::Int64(99") && generated.contains("Value::Small(7"), "{generated}");
}
