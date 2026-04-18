/// TDD: Codegen bugs discovered from windjammer-ui dogfooding.
///
/// Bug 1: clamp() arguments get wrong float suffix (f32 instead of f64)
///   Source: (self.value / self.max * 100.0).clamp(0.0, 100.0)
///   Generated: (...).clamp(0.0_f32, 100.0_f32) — but receiver is f64!
///   Fix: Method call arguments inherit receiver's float type for literal suffixes.
///
/// Bug 2: Iterator variable &String compared with owned String without deref
///   Source: for o in opts { if o == self.value { ... } }
///   Generated: o == self.value — but o is &String (borrowed iter), self.value is String
///   Fix: Auto-deref borrowed iterator variable in comparisons.
///
/// Bug 3: self.field moves String out of &self context
///   Source: let star_color = if filled { self.color } else { "#e2e8f0" }
///   Generated: self.color — but self is &Self, can't move String out
///   Fix: Auto-clone non-Copy types accessed from &self.

use std::process::Command;
use tempfile::TempDir;
use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

fn run_rustc(rs_code: &str) -> (bool, String) {
    let test_dir = TempDir::new().expect("Failed to create temp dir");
    let rs_file = test_dir.path().join("test.rs");
    std::fs::write(&rs_file, rs_code).unwrap();

    let output = Command::new("rustc")
        .arg(&rs_file)
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg("--out-dir")
        .arg(test_dir.path())
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
}

/// Bug 1: clamp() arguments should get f64 suffix when receiver is f64
#[test]
fn test_clamp_args_match_receiver_float_type() {
    let source = r#"
pub struct Progress {
    value: float,
    max: float,
}

impl Progress {
    pub fn render(self) -> string {
        let percentage = (self.value / self.max * 100.0).clamp(0.0, 100.0)
        format!("{}", percentage)
    }
}
"#;

    let result = compile_and_get_rust(source);

    // The clamp args should NOT have _f32 suffix when the receiver is f64
    assert!(
        !result.contains("clamp(0.0_f32"),
        "clamp args should not use f32 suffix when receiver is f64. Got:\n{}",
        result
    );

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "Should compile without type mismatch in clamp args. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Bug 2: Borrowed iterator variable should auto-deref in comparisons
#[test]
fn test_borrowed_iter_var_comparison_deref() {
    let source = r#"
pub struct Editor {
    value: string,
}

impl Editor {
    pub fn render(self) -> string {
        let opts = vec!["a".to_string(), "b".to_string()]
        let mut result = String::new()
        for o in opts {
            let selected = if o == self.value { "selected" } else { "" }
            result = result + selected
        }
        result
    }
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "Borrowed iterator var comparison should compile. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Bug 2b: push_str(format!(...)) should auto-borrow the format result
#[test]
fn test_push_str_format_auto_borrow() {
    let source = r#"
pub struct Serializer {
    name: string,
}

impl Serializer {
    pub fn serialize(self) -> string {
        let mut json = String::new()
        json.push_str(format!("name: {}", self.name))
        json
    }
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "push_str(format!(...)) should compile (auto-borrow String to &str). stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Bug 4: Comparison in destructured enum context should not over-clone
/// When iterating a Vec<String> from a destructured enum field (which is borrowed),
/// the loop var is &String. Comparing &String with self.field (also &String in &self)
/// should work without cloning — both sides are references.
///   Source: match self.kind { Kind::Items { data: items } => { for item in items { if item == self.value { ... } } } }
///   Bug: self.value gets auto-cloned to String, making &String == String (fails)
///   Fix: Don't auto-clone self.field in comparison contexts — refs compare fine.
#[test]
fn test_enum_destructure_iter_comparison_no_clone() {
    let source = r#"
pub enum Kind {
    Items { data: Vec<string> },
    Empty,
}

pub struct Foo {
    value: string,
    kind: Kind,
}

impl Foo {
    pub fn render(self) -> string {
        let mut result = String::new()
        match self.kind {
            Kind::Items { data: items } => {
                for item in items {
                    let sel = if item == self.value { "yes" } else { "no" }
                    result = result + sel
                }
            },
            Kind::Empty => {},
        }
        result
    }
}
"#;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "Comparison between &String (iter var from destructured enum) and self.field \
         should compile. Both should be references. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Bug 5: String concatenation with format!() should not cast to f32
/// Source: options_html = options_html + format!("...", item)
/// Bug: Generated as options_html as f32 + &format!(...) — incorrect float cast
/// Fix: String + format!() should produce string concatenation, not numeric addition.
#[test]
fn test_string_concat_with_format_no_float_cast() {
    let source = r#"
pub struct Builder {
    items: Vec<string>,
}

impl Builder {
    pub fn build(self) -> string {
        let mut result = "".to_string()
        for item in self.items {
            result = result + format!("<li>{}</li>", item)
        }
        result
    }
}
"#;

    let result = compile_and_get_rust(source);

    assert!(
        !result.contains("as f32"),
        "String concatenation should NOT produce 'as f32' cast. Got:\n{}",
        result
    );

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "String + format!() should compile as string concatenation. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Bug 5b: String concat inside enum match arm with multiple format args
/// Reproduces the exact propertyeditor.wj pattern with match + iter + concat + multi-arg format
#[test]
fn test_string_concat_in_enum_match_arm() {
    let source = r#"
pub enum PropType {
    Dropdown { options: Vec<string> },
    Text,
}

pub struct Property {
    value: string,
    prop_type: PropType,
}

impl Property {
    pub fn render(self) -> string {
        match self.prop_type {
            PropType::Dropdown { options: opts } => {
                let mut options_html = "".to_string()
                for o in opts {
                    let selected = if o == self.value { "selected" } else { "" }
                    options_html = options_html + format!("<option value='{}' {}>{}</option>", o, selected, o)
                }
                options_html
            },
            PropType::Text => {
                "text".to_string()
            },
        }
    }
}
"#;

    let result = compile_and_get_rust(source);

    assert!(
        !result.contains("as f32"),
        "String concat in enum match arm should NOT produce 'as f32' cast. Got:\n{}",
        result
    );

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "String concat in enum match arm should compile. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}

/// Bug 3: self.field should auto-clone non-Copy types in &self context
#[test]
fn test_self_field_auto_clone_in_ref_context() {
    let source = r##"
pub struct Rating {
    color: string,
    readonly: bool,
}

impl Rating {
    pub fn render(self) -> string {
        let star_color = if self.readonly {
            self.color
        } else {
            "#e2e8f0"
        }
        format!("color: {}", star_color)
    }
}
"##;

    let result = compile_and_get_rust(source);

    let (rustc_ok, stderr) = run_rustc(&result);
    assert!(
        rustc_ok,
        "self.field should auto-clone non-Copy types in &self. stderr: {}\n\nGenerated:\n{}",
        stderr,
        result
    );
}
