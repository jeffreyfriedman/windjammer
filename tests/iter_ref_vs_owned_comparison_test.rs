// BUG: Comparing iteration variable (&String) with struct field (String)
//
// When iterating over a collection destructured from an enum variant match,
// the iteration variable is a &String, but the struct field being compared
// is an owned String. The comparison `o == self.value` generates
// `&String == String` which Rust doesn't implement directly.
//
// Root cause: The codegen doesn't add * dereference when comparing
// a borrowed iteration variable with an owned value.

use std::fs;
use std::process::Command;

fn compile_wj_test(source: &str) -> (bool, String, String) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let unique_id = format!("test_{}_{}", std::process::id(), test_id);

    let _tmp = tempfile::tempdir().unwrap();

    let temp_dir = _tmp.path();

    let test_file = temp_dir.join(format!("{}.wj", unique_id));
    fs::write(&test_file, source).expect("Failed to write temp file");

    let output_dir = temp_dir.join(format!("output_{}", unique_id));
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "--output",
            output_dir.to_str().unwrap(),
            test_file.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let rs_file = output_dir.join(format!("{}.rs", unique_id));
    let rust_code = fs::read_to_string(&rs_file).unwrap_or_default();

    let _ = fs::remove_file(&test_file);

    (success, rust_code, stderr)
}

fn compile_wj_and_verify(source: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static VERIFY_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let (success, rust_code, stderr) = compile_wj_test(source);

    if !success {
        panic!(
            "WJ compilation failed:\n{}\n\nGenerated:\n{}",
            stderr, rust_code
        );
    }

    let verify_id = VERIFY_COUNTER.fetch_add(1, Ordering::SeqCst);
    let _tmp2 = tempfile::tempdir().unwrap();
    let temp_dir = _tmp2.path();

    let rs_file = temp_dir.join(format!(
        "verify_iter_ref_{}_{}.rs",
        std::process::id(),
        verify_id
    ));
    fs::write(&rs_file, &rust_code).expect("Failed to write rs file");

    let verify = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            rs_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let _ = fs::remove_file(&rs_file);

    if !verify.status.success() {
        let verify_stderr = String::from_utf8_lossy(&verify.stderr);
        panic!(
            "Generated Rust doesn't compile:\n{}\n\nGenerated code:\n{}",
            verify_stderr, rust_code
        );
    }

    rust_code
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_compared_to_struct_field() {
    // This reproduces the propertyeditor.wj bug:
    // for o in opts { if o == self.value { ... } }
    // where opts comes from enum destructuring (borrowed)
    let source = r#"
pub enum WidgetType {
    Text,
    Dropdown { options: Vec<string> },
}

pub struct Widget {
    pub value: string,
    pub widget_type: WidgetType,
}

impl Widget {
    pub fn render(self) -> string {
        match self.widget_type {
            WidgetType::Dropdown { options: opts } => {
                let mut result = String::new()
                for o in opts {
                    if o == self.value {
                        result.push_str("selected")
                    }
                }
                result
            },
            _ => "text",
        }
    }
}

fn main() {
    let w = Widget {
        value: "a",
        widget_type: WidgetType::Dropdown { options: vec!["a", "b", "c"] },
    }
    let r = w.render()
}
"#;

    let rust_code = compile_wj_and_verify(source);

    // The generated Rust must compile. The comparison between
    // an iteration variable (possibly &String) and an owned field (String)
    // must be handled correctly by the codegen.
    assert!(
        !rust_code.contains("can't compare"),
        "Should not have comparison errors in generated code"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_str_comparison_with_field() {
    let source = r#"
pub struct Filter {
    pub items: Vec<string>,
    pub current: string,
}

impl Filter {
    pub fn find_current(self) -> bool {
        for item in self.items {
            if item == self.current {
                return true
            }
        }
        false
    }
}

fn main() {
    let f = Filter { items: vec!["a", "b"], current: "a" }
    let found = f.find_current()
}
"#;

    compile_wj_and_verify(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_destructured_enum_iter_comparison() {
    // Reproduces the propertyeditor.wj bug:
    // match self.widget_type {
    //     Dropdown { options: opts } => { for o in opts { if o == self.value { ... } } }
    // }
    // opts comes from match destructuring a borrowed enum variant,
    // so it's in borrowed_iterator_vars. When used as for-loop iterable,
    // is_iterating_over_borrowed must recognize it.
    let source = r#"
pub enum WidgetType {
    Text,
    Dropdown { options: Vec<string> },
}

pub struct Widget {
    pub value: string,
    pub widget_type: WidgetType,
}

impl Widget {
    pub fn find_selected(self) -> string {
        match self.widget_type {
            WidgetType::Dropdown { options: opts } => {
                let mut result = String::new()
                for o in opts {
                    if o == self.value {
                        result = "found"
                    }
                }
                result
            },
            _ => "",
        }
    }
}

fn main() {
    let w = Widget {
        value: "b",
        widget_type: WidgetType::Dropdown { options: vec!["a", "b", "c"] },
    }
    let r = w.find_selected()
}
"#;

    let rust_code = compile_wj_and_verify(source);

    // Verify the generated code doesn't have &String == String comparison
    assert!(
        !rust_code.contains("can't compare"),
        "Generated Rust should compile without comparison errors:\n{}",
        rust_code
    );
}
