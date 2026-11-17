//! A generic parameter is not a type to import: a file with a wildcard import and `fn show<F: Named>(fe: F)` once
//! came out with `use super::F;` (E0432 unresolved import). The library build of the reduced project must emit no
//! import named after the parameter, and the generated crate must pass rustc.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn write(dir: &Path, rel: &str, text: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn build_library(src: &Path, out: &Path) {
    let o = Command::new(WJ)
        .arg("build")
        .arg(src)
        .arg("-o")
        .arg(out)
        .args(["--library", "--module-file", "--no-cargo", "--no-generate-cargo-toml", "--no-lint"])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
}

fn rustc_accepts_lib(out: &Path) -> Result<(), String> {
    let o = Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "lib", "--emit=metadata", "-A", "warnings", "--crate-name", "generic_param"])
        .arg("-o")
        .arg(out.join("out.rmeta"))
        .arg(out.join("lib.rs"))
        .output()
        .unwrap();
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).to_string())
    }
}

fn project(d: &Path, user: &str) {
    write(d, "src/mod.wj", "pub mod tr\npub mod user\n");
    write(
        d,
        "src/tr.wj",
        "pub trait Named {\n    fn name(&self) -> string\n}\n\npub struct A {\n    pub n: u32,\n}\n\nimpl Named for A {\n    fn name(&self) -> string {\n        \"a\"\n    }\n}\n",
    );
    write(d, "src/user.wj", user);
}

const GLOB_USER: &str = "use crate::tr::*\n\npub struct Report {\n    pub n: u32,\n}\n\npub fn show<F: Named>(fe: F) -> Report {\n    let a = A { n: 1 }\n    Report { n: a.n + (fe.name().len() as u32) }\n}\n";

#[test]
fn a_wildcard_import_and_a_generic_function_emit_no_import_of_the_parameter() {
    let d = Scratch::new("generic-param-import");
    project(d.path(), GLOB_USER);
    let out = d.join("out");
    build_library(&d.join("src"), &out);
    let user = std::fs::read_to_string(out.join("user.rs")).unwrap();
    assert!(!user.contains("use super::F;"), "the generic parameter is not an import:\n{user}");
    assert!(user.contains("use crate::tr::*;") && user.contains("pub fn show<F: Named>"), "{user}");
    rustc_accepts_lib(&out).unwrap_or_else(|e| panic!("rustc rejects the generated crate:\n{e}\n{user}"));
}

#[test]
fn explicit_imports_still_build_the_same_crate() {
    let d = Scratch::new("generic-param-explicit");
    project(d.path(), &GLOB_USER.replace("use crate::tr::*", "use crate::tr::Named\nuse crate::tr::A"));
    let out = d.join("out");
    build_library(&d.join("src"), &out);
    let user = std::fs::read_to_string(out.join("user.rs")).unwrap();
    assert!(!user.contains("use super::F;"), "{user}");
    rustc_accepts_lib(&out).unwrap_or_else(|e| panic!("rustc rejects the generated crate:\n{e}\n{user}"));
}
