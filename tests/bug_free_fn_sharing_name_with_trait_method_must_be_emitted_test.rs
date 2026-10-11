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
))]

//! A free function that shares its name with a trait or impl method must still be emitted.
//! Program generation used to decide "this is an impl method, skip it as a free function" by
//! bare name, so `fn register(deps, ..)` silently vanished (or took the method's `&Self`
//! signature in multi-module builds) whenever any method was called `register`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;

const SOURCE: &str = r#"
pub struct Draft {
    pub name: string,
}

trait Port {
    fn register(self, tenant: string, draft: Draft) -> Result<int, string>
}

pub struct Adapter {
    pub n: int,
}

impl Port for Adapter {
    fn register(self, tenant: string, draft: Draft) -> Result<int, string> {
        Ok(self.n)
    }
}

pub struct Deps {
    pub adapter: Adapter,
    pub label: string,
}

pub fn register(deps: Deps, tenant_slug: string, draft: Draft) -> Result<int, string> {
    deps.adapter.register(tenant_slug, draft)
}

pub fn main() {
    let deps = Deps { adapter: Adapter { n: 7 }, label: "x" }
    match register(deps, "t", Draft { name: "d" }) {
        Ok(n) => println("${n}"),
        Err(e) => println("${e}"),
    }
}
"#;

#[test]
fn free_function_named_like_a_trait_method_is_emitted_and_runs() {
    let (rust, ok) = test_utils::compile_single_check(SOURCE);
    assert!(ok, "transpile must succeed");
    assert!(
        rust.contains("pub fn register(deps:"),
        "free function `register` must be emitted:\n{rust}"
    );
    let tmp = tempfile::tempdir().expect("tempdir");
    let rs = tmp.path().join("prog.rs");
    fs::write(&rs, &rust).expect("write rs");
    let build = Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "bin", "--out-dir"])
        .arg(tmp.path())
        .arg(&rs)
        .output()
        .expect("run rustc");
    assert!(
        build.status.success(),
        "generated Rust must compile:\n{}\n\n{rust}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(tmp.path().join("prog")).output().expect("run");
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "7");
}
