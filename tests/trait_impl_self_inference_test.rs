// TDD: Trait impl methods must inherit receiver (&self / &mut self) from trait analysis,
// not from impl body alone (empty bodies and missing self in AST must still match trait).

use std::fs;
use std::process::Command;
use std::sync::Mutex;

use tempfile::TempDir;

// `wj build` is not safe to invoke concurrently from multiple tests (shared cwd / temp paths).
static WJ_BUILD_LOCK: Mutex<()> = Mutex::new(());

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let _lock = WJ_BUILD_LOCK.lock().expect("wj build lock poisoned");
    let temp = TempDir::new().expect("temp dir");
    let test_dir = temp.path();
    let input_file = test_dir.join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let generated_file = test_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_file)
        .unwrap_or_else(|_| String::from_utf8_lossy(&output.stdout).to_string());

    if output.status.success() {
        Ok(generated)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_inherits_borrowed_self_from_trait() {
    let code = r#"
pub trait Reader {
    fn read() -> string
}

pub struct FileReader {
    path: string
}

impl Reader for FileReader {
    fn read() -> string {
        self.path
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    assert!(
        g.contains("fn read(&self) -> String"),
        "impl/trait should use &self; got:\n{}",
        g
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_inherits_mut_self_from_trait() {
    let code = r#"
pub trait Counter {
    fn increment()
}

pub struct SimpleCounter {
    count: int
}

impl Counter for SimpleCounter {
    fn increment() {
        self.count = self.count + 1
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    assert!(
        g.contains("fn increment(&mut self)"),
        "impl/trait should use &mut self; got:\n{}",
        g
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_associated_fn_no_receiver_create_self() {
    let code = r#"
pub trait Factory {
    fn create() -> Self
}

pub struct Thing {
    field: int
}

impl Factory for Thing {
    fn create() -> Thing {
        Thing { field: 0 }
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    assert!(
        g.contains("fn create() -> Thing"),
        "associated function should not add self; got:\n{}",
        g
    );
    assert!(
        !g.contains("fn create(&self) -> Thing"),
        "should not infer &self for create() -> Self; got:\n{}",
        g
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_empty_body_still_gets_trait_receiver() {
    let code = r#"
pub trait Port {
    fn shutdown()
}

pub struct Renderer {
    width: int
}

impl Port for Renderer {
    fn shutdown() {
    }
}
"#;

    let result = compile_windjammer_code(code);
    assert!(result.is_ok(), "compile failed: {:?}", result.err());
    let g = result.unwrap();
    assert!(
        g.contains("fn shutdown(&mut self)"),
        "void abstract trait methods default to &mut self; empty impl must match; got:\n{}",
        g
    );
}
