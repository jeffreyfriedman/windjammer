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

//! `use rowlib::bytes::slice_bytes` imports a dependency function by its module path; the dependency's
//! `metadata.json` registers it under the crate key (`rowlib::slice_bytes`) with a read-only `data: &Vec<u8>`.
//! The call names that declaration, so the caller's explicit `data.clone()` becomes `&data` (E0308 otherwise).

use std::fs;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn wj(args: &[&str], cwd: &Path) {
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).args(args).current_dir(cwd).env("WJ_CARGO_TARGET_ROOT", cwd.join(".cache")).output().unwrap();
    assert!(out.status.success(), "wj {args:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn imported_dependency_function_is_called_by_its_declared_formals() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    write(t, "rowlib/wj.toml", "[package]\nname = \"rowlib\"\nversion = \"0.1.0\"\n\n[lib]\n");
    write(t, "rowlib/src/mod.wj", "pub mod bytes\n");
    write(
        t,
        "rowlib/src/bytes.wj",
        "pub fn slice_bytes(data: Vec<u8>, start: usize, end: usize) -> Vec<u8> {\n    let mut out: Vec<u8> = Vec::new()\n    let mut i: usize = start\n    \
         while i < end && i < data.len() {\n        out.push(data[i])\n        i = i + 1\n    }\n    out\n}\n",
    );
    write(t, "app/src/mod.wj", "pub mod m\n");
    write(
        t,
        "app/src/m.wj",
        "use rowlib::bytes::slice_bytes\n\npub fn head(data: Vec<u8>) -> Vec<u8> {\n    let body = slice_bytes(data.clone(), 0, data.len() - 1)\n    body\n}\n",
    );
    let lib = ["--library", "--module-file", "--no-cargo", "--no-generate-cargo-toml", "--no-lint"];
    let mut args = vec!["build", "src", "-o", "gen"];
    args.extend(lib);
    wj(&args, &t.join("rowlib"));
    let rowlib_rs = fs::read_to_string(t.join("rowlib/gen/bytes.rs")).unwrap();
    assert!(rowlib_rs.contains("data: &Vec<u8>"), "the dependency's read-only parameter is borrowed:\n{rowlib_rs}");
    let meta = format!("rowlib={}", t.join("rowlib/gen/metadata.json").display());
    let mut args = vec!["build", "src", "-o", "gen"];
    args.extend(lib);
    args.extend(["--metadata", meta.as_str()]);
    wj(&args, &t.join("app"));
    let m_rs = fs::read_to_string(t.join("app/gen/m.rs")).unwrap();
    assert!(m_rs.contains("slice_bytes(&data,"), "the call follows the declared &Vec<u8> formal:\n{m_rs}");
}
