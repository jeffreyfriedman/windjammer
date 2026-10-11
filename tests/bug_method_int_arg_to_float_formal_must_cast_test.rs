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

//! A method whose formals are `f32` must receive integer arguments cast to `f32`, for plain
//! identifiers as well as arithmetic. Every method used to be flagged as a "type collision"
//! because its receiver slot was registered as both `&Self` and the concrete type, which
//! disabled the int-to-float call-site cast.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;

const SOURCE: &str = r#"
pub struct Grid {
    width: i32,
}

impl Grid {
    pub fn check(self, x: f32, y: f32) -> f32 {
        x + y
    }

    pub fn go(self, x: i32, y: i32) -> f32 {
        self.check(x, y)
    }

    pub fn go_shifted(self, x: i32, y: i32) -> f32 {
        self.check(x + 1, y)
    }
}

pub fn main() {
    let g = Grid { width: 3 }
    println("${g.go(1, 2)} ${g.go_shifted(1, 2)}")
}
"#;

#[test]
fn int_arguments_are_cast_for_float_method_formals() {
    let (rust, ok) = test_utils::compile_single_check(SOURCE);
    assert!(ok, "transpile must succeed");
    assert!(
        !rust.contains("as f32) as f32"),
        "casts must not stack:\n{rust}"
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
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "3 4");
}
