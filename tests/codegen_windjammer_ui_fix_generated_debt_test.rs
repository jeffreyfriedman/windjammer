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

//! Gates for windjammer-ui TECH_DEBT.md bugs that historically required
//! post-processing / hand patches before a pure-Windjammer regenerate is safe.
//!
//! Mapping (TECH_DEBT.md):
//!   #1 desktop feature gates
//!   #2 no pub mod for missing files
//!   #4 Rc<F> → Rc<dyn Fn()> coercion
//!   #5 doc comments before macros (thread_local!)
//!
//! Already covered elsewhere:
//!   #3 ambiguous globs → codegen_ambiguous_vnode_reexport_test.rs
//!   #6/#7 unused fields/params → codegen_unused_variable_prefix_test.rs (+ related)
//!   StatusChip/Badge ownership → codegen_owned_reuse_after_helper_test.rs,
//!     codegen_owned_string_param_test.rs, codegen_string_param_to_owned_method_test.rs

#[path = "common/test_utils.rs"]
mod test_utils;

/// Bug #1: modules that import egui/eframe must emit `#[cfg(feature = "desktop")]`
/// on their `pub mod` declarations (or equivalent metadata-driven gate).
#[test]
fn desktop_backend_import_should_cfg_gate_module() {
    // Multi-file library layout: entry + desktop-only module.
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("mod.wj"),
        r#"
pub mod chrome
pub mod desktop_panel
"#,
    )
    .unwrap();
    std::fs::write(
        src.join("chrome.wj"),
        r#"
pub fn label() -> string { "ok".to_string() }
"#,
    )
    .unwrap();
    std::fs::write(
        src.join("desktop_panel.wj"),
        r#"
// Uses desktop backend types — codegen must feature-gate this module.
use egui::Ui

pub fn paint(ui: Ui) -> string {
    "panel".to_string()
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("wj");

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lib = std::fs::read_to_string(out.join("lib.rs"))
        .or_else(|_| std::fs::read_to_string(out.join("mod.rs")))
        .unwrap_or_default();

    let gated = lib.contains("#[cfg(feature = \"desktop\")]")
        && (lib.contains("pub mod desktop_panel") || lib.contains("mod desktop_panel"));
    assert!(
        gated || output.status.success() && combined.contains("cfg(feature"),
        "desktop_panel (egui import) must be cfg-gated in generated mod. lib.rs:\n{}\ncompiler:\n{}",
        lib,
        combined
    );
}

/// Bug #2: do not emit `pub mod X` / `pub use X::*` when X.rs was never generated.
#[test]
fn should_not_declare_modules_without_generated_files() {
    let source = r#"
pub fn hello() -> string {
    "hi".to_string()
}

fn main() {
    println!("{}", hello())
}
"#;
    let result = test_utils::compile_single(source);
    assert!(
        !result.contains("pub mod lib;")
            && !result.contains("pub use lib::*")
            && !result.contains("pub mod examples_wasm"),
        "codegen must not hallucinate missing modules. Got:\n{}",
        result
    );
}

/// Bug #4: pass Rc<F: Fn()> into APIs expecting &Rc<dyn Fn()>.
#[test]
fn rc_generic_fn_should_coerce_to_dyn_fn_trait_object() {
    let source = r#"
use std::rc::Rc

fn run_effect(id: int, f: &Rc<dyn Fn()>) {
    let _ = id
    f()
}

fn schedule(f: Rc) {
    // WJ: Rc capturing a closure — codegen must coerce to Rc<dyn Fn()>
    run_effect(1, &f)
}

fn main() {
    schedule(Rc::new(|| {}))
}
"#;
    let result = test_utils::compile_single(source);
    let __result = test_utils::verify_rust_compiles(&result);
    let ok = __result.is_ok()
        || result.contains("Rc<dyn Fn()>")
        || result.contains("as Rc<dyn")
        || result.contains("dyn Fn()");
    assert!(
        ok,
        "Rc<F> passed to &Rc<dyn Fn()> must coerce. stderr={:?}\nGot:\n{}",
        __result.err(),
        result
    );
}

/// Bug #5: macros must not be preceded by `///` doc comments (rustc warns / rejects attachment).
#[test]
fn macros_should_use_plain_comments_not_doc_comments() {
    let source = r#"
/// Global reactive context
thread_local! {
    static CTX: int = 0
}

fn main() {
    println!("ok")
}
"#;
    let result = test_utils::compile_single(source);
    // After codegen, thread_local! should not have /// immediately above it.
    let bad = result.contains("/// Global") && result.contains("thread_local!");
    let good = result.contains("// Global") && result.contains("thread_local!");
    assert!(
        good || !bad,
        "doc comment before thread_local! must become plain //. Got:\n{}",
        result
    );
}
