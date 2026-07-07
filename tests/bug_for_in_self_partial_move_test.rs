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

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// TDD test: When a method takes owned `self` and iterates over `self.collection`
/// with the loop body also accessing other self fields/methods,
/// the compiler must iterate with `&self.collection` to avoid partial move.
///
/// Bug: `for x in self.passes { ... self.has_dependents(...) ... }` moves self.passes,
/// then self.has_dependents() can't borrow self (E0382: partially moved).
///
/// When all methods are in the same file, the compiler correctly detects mutations
/// and infers `&mut self`, which makes `&self.passes` work. But we verify the
/// loop body self-access detection ensures `&` on the iterable.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_in_self_field_borrows_when_self_used_in_body() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_for_in_self_partial_move");

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_source = r#"
struct PassDef {
    id: i32,
    shader: string
}

struct CompiledPass {
    id: i32,
    has_deps: bool
}

struct GraphBuilder {
    passes: Vec<PassDef>,
    dep_count: i32
}

impl GraphBuilder {
    pub fn has_dependents(self, id: i32) -> bool {
        self.dep_count > id
    }

    pub fn infer_deps(self) {
        self.dep_count = self.passes.len() as i32
    }

    pub fn build(self) -> Vec<CompiledPass> {
        self.infer_deps()
        let mut result = Vec::new()
        for pass in self.passes {
            let compiled = CompiledPass {
                id: pass.id,
                has_deps: self.has_dependents(pass.id)
            }
            result.push(compiled)
        }
        result
    }
}
"#;

    let wj_file = test_dir.join("for_self_partial_move.wj");
    fs::write(&wj_file, wj_source).unwrap();

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(test_dir.join("out"))
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(test_dir.join("out").join("for_self_partial_move.rs")).unwrap();

    // The for loop over self.passes must avoid partial move since
    // self.has_dependents() is called in the loop body.
    // Valid strategies: borrow (&self.passes), .iter(), or .clone()
    let iterates_by_ref = generated.contains("for pass in &self.passes")
        || generated.contains("for pass in self.passes.iter()")
        || generated.contains("self.passes.clone()");

    assert!(
        iterates_by_ref,
        "Expected `for pass in &self.passes`, `.iter()`, or `.clone()` to avoid partial move.\n\
         Generated code:\n{}",
        generated
    );

    // Verify the generated code compiles with rustc
    let rs_file = test_dir.join("out").join("for_self_partial_move.rs");
    let rustc_out = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg(&rs_file)
        .arg("-o")
        .arg(test_dir.join("out").join("test.rlib"))
        .output()
        .expect("Failed to run rustc");
    assert!(
        rustc_out.status.success(),
        "Generated code doesn't compile with rustc:\n{}",
        String::from_utf8_lossy(&rustc_out.stderr)
    );
}
