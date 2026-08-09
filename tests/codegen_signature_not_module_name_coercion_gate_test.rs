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

//! Ownership/coercion follows resolved signatures — not module-name lists or
//! blanket `inline_module_qualified_call` skips.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile(source: &str, name: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join(name);
    fs::write(&path, source).unwrap();
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .args([
            "build",
            path.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        output.status.success(),
        "compile failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let rs = out.join(name.replace(".wj", ".rs"));
    fs::read_to_string(&rs).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .unwrap()
            .flatten()
            .find(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .map(|e| e.path())
            .map(|p| fs::read_to_string(p).unwrap())
            .expect("generated .rs")
    })
}

#[test]
fn user_module_named_strings_owned_formal_owns_literal() {
    let rust = compile(
        r#"
mod strings {
    pub fn take_owned(msg: string) {
        let mut v: Vec<string> = Vec::new()
        v.push(msg)
    }
}

fn main() {
    strings::take_owned("hello")
}
"#,
        "user_strings_owned.wj",
    );
    assert!(
        rust.contains("msg: String") || rust.contains("msg: string"),
        "stored string formal must stay owned String:\n{rust}"
    );
    assert!(
        rust.contains("take_owned(\"hello\".to_string())")
            || rust.contains("take_owned(String::from(\"hello\"))"),
        "owned string formal must coerce literal even if module is named strings:\n{rust}"
    );
}

#[test]
fn user_module_owned_formal_owns_literal() {
    let rust = compile(
        r#"
mod helpers {
    pub fn take_owned(msg: string) {
        let mut v: Vec<string> = Vec::new()
        v.push(msg)
    }
}

fn main() {
    helpers::take_owned("hello")
}
"#,
        "user_helpers_owned.wj",
    );
    assert!(
        rust.contains("take_owned(\"hello\".to_string())")
            || rust.contains("take_owned(String::from(\"hello\"))"),
        "owned string formal in user module must coerce literal:\n{rust}"
    );
}

#[test]
fn domain_borrowed_formal_keeps_bare_literal() {
    let rust = compile(
        r#"
struct Sink {
    n: int
}

impl Sink {
    fn accept(self, needle: string) -> int {
        if needle == "" {
            return 0
        }
        self.n
    }
}

fn main() {
    let s = Sink { n: 1 }
    let _ = s.accept("x")
}
"#,
        "domain_borrowed_literal.wj",
    );
    assert!(
        rust.contains("accept(\"x\")") && !rust.contains("accept(\"x\".to_string())"),
        "read-only/borrowed string formal must keep bare literal:\n{rust}"
    );
}

#[test]
fn vec_remove_index_not_map_key_borrow() {
    let rust = compile(
        r#"
fn drop_at(items: Vec<string>, i: int) -> Vec<string> {
    let mut items = items
    items.remove(i as usize)
    items
}

fn main() {
    let v = Vec::new()
    let _ = drop_at(v, 0)
}
"#,
        "vec_remove_not_map_key.wj",
    );
    assert!(
        rust.contains("remove(") && !rust.contains("remove(&"),
        "Vec::remove index must not get map-key borrow:\n{rust}"
    );
}
