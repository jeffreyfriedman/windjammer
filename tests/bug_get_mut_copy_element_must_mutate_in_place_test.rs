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

//! Silent mutation loss: `Option<&mut T>` for a `Copy` element type must not be
//! turned into an owned copy (`.copied()`), or every mutation through the binding is
//! applied to a temporary and silently dropped.
//!
//! The program is compiled and *run*; the observable result is the contract.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;

const SOURCE: &str = r#"
pub struct Counter {
    value: u32,
    limit: u32,
}

impl Counter {
    pub fn bump(self, amount: u32) {
        self.value = (self.value + amount).min(self.limit)
    }
}

pub struct Bank {
    items: Vec<Counter>,
}

impl Bank {
    pub fn bump_at(self, idx: int, amount: u32) {
        if let Some(c) = self.items.get_mut(idx) {
            c.bump(amount)
        }
    }

    pub fn value_at(self, idx: int) -> u32 {
        self.items[idx].value
    }
}

pub fn main() {
    let mut bank = Bank { items: vec![Counter { value: 1, limit: 100 }] }
    bank.bump_at(0, 9)
    println("${bank.value_at(0)}")
}
"#;

fn compile_and_run(rust: &str) -> String {
    let tmp = tempfile::tempdir().expect("tempdir");
    let rs = tmp.path().join("prog.rs");
    fs::write(&rs, rust).expect("write rs");
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
    let run = Command::new(tmp.path().join("prog")).output().expect("run prog");
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

#[test]
fn get_mut_on_copy_element_mutates_the_vec_element() {
    let (rust, ok) = test_utils::compile_single_check(SOURCE);
    assert!(ok, "transpile must succeed");
    assert!(
        !rust.contains("get_mut(idx as usize).copied()"),
        "Option<&mut T> must not be copied before mutation:\n{rust}"
    );
    assert_eq!(compile_and_run(&rust), "10", "mutation through get_mut was lost:\n{rust}");
}
