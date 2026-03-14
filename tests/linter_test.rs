//! Linter Tests - TDD for Rust leakage detection
//!
//! Tests that the linter detects Rust-specific patterns and suggests
//! idiomatic Windjammer alternatives.

use windjammer::lexer::Lexer;
use windjammer::linter::rust_leakage::RustLeakageLinter;
use windjammer::parser::Parser;

fn parse_and_lint(source: &str) -> Vec<windjammer::linter::LintDiagnostic> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, "test.wj".to_string(), source.to_string());
    let program = parser.parse().expect("Parse should succeed");
    let mut linter = RustLeakageLinter::new("test.wj");
    linter.lint_program(&program);
    linter.into_diagnostics()
}

#[test]
fn test_detect_explicit_self_mut() {
    let source = r#"
struct Counter {
    value: i32,
}

impl Counter {
    pub fn update(&mut self, dt: f32) {
        self.value = self.value + 1
    }
}
"#;

    let warnings = parse_and_lint(source);

    assert!(
        !warnings.is_empty(),
        "Should detect &mut self - got {} warnings",
        warnings.len()
    );
    let w0001 = warnings.iter().find(|w| w.lint_name == "W0001");
    assert!(w0001.is_some(), "Should detect W0001 explicit ownership");
    assert!(
        w0001.unwrap().message.contains("explicit ownership"),
        "Message should mention explicit ownership"
    );
    assert!(
        w0001.unwrap().suggestion.as_ref().map(|s| s.contains("self")).unwrap_or(false),
        "Suggestion should mention self"
    );
}

#[test]
fn test_detect_unwrap() {
    let source = r#"
struct Game {
    entities: Vec<i32>,
}

impl Game {
    pub fn get_entity(self, id: u32) -> i32 {
        let entity = self.entities.get(id).unwrap()
        entity
    }
}
"#;

    let warnings = parse_and_lint(source);

    assert!(
        !warnings.is_empty(),
        "Should detect .unwrap() - got {} warnings: {:?}",
        warnings.len(),
        warnings.iter().map(|w| &w.lint_name).collect::<Vec<_>>()
    );
    let w0002 = warnings.iter().find(|w| w.lint_name == "W0002");
    assert!(w0002.is_some(), "Should detect W0002 unwrap");
    assert!(
        w0002.unwrap().message.contains("unwrap"),
        "Message should mention unwrap"
    );
}

#[test]
fn test_detect_iter() {
    let source = r#"
struct Processor {
    items: Vec<i32>,
}

impl Processor {
    pub fn process_all(self) {
        for entity in self.items.iter() {
            let _ = entity
        }
    }
}
"#;

    let warnings = parse_and_lint(source);

    assert!(
        !warnings.is_empty(),
        "Should detect .iter() - got {} warnings",
        warnings.len()
    );
    let w0003 = warnings.iter().find(|w| w.lint_name == "W0003");
    assert!(w0003.is_some(), "Should detect W0003 iter");
    assert!(
        w0003.unwrap().message.contains("iter"),
        "Message should mention iter"
    );
}

#[test]
fn test_detect_explicit_borrow() {
    let source = r#"
struct Mesh {}
struct Transform {}

fn draw_mesh(mesh: Mesh, transform: Transform) {}

pub fn render(mesh: Mesh, transform: Transform) {
    draw_mesh(&mesh, &transform)
}
"#;

    let warnings = parse_and_lint(source);

    assert!(
        warnings.len() >= 2,
        "Should detect &mesh and &transform - got {} warnings",
        warnings.len()
    );
    let w0004_count = warnings.iter().filter(|w| w.lint_name == "W0004").count();
    assert!(
        w0004_count >= 2,
        "Should detect at least 2 W0004 explicit borrows, got {}",
        w0004_count
    );
}

#[test]
fn test_no_false_positives_trait_impl() {
    let source = r#"
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

struct MyIter {
    count: i32,
}

impl Iterator for MyIter {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.count < 10 {
            self.count = self.count + 1;
            Some(self.count)
        } else {
            None
        }
    }
}
"#;

    let warnings = parse_and_lint(source);

    let w0001_count = warnings.iter().filter(|w| w.lint_name == "W0001").count();
    assert_eq!(
        w0001_count, 0,
        "Should NOT warn on &mut self in trait impl (trait requires it) - got {} W0001",
        w0001_count
    );
}

#[test]
fn test_no_false_positives_idiomatic() {
    let source = r#"
struct Counter {
    value: i32,
}

impl Counter {
    pub fn increment(self) {
        self.value = self.value + 1
    }

    pub fn get_value(self) -> i32 {
        self.value
    }
}

fn process(data: String) -> String {
    data
}
"#;

    let warnings = parse_and_lint(source);

    assert_eq!(
        warnings.len(),
        0,
        "Idiomatic Windjammer should have zero warnings - got: {:?}",
        warnings.iter().map(|w| (&w.lint_name, &w.message)).collect::<Vec<_>>()
    );
}

#[test]
fn test_linter_catches_all_patterns() {
    let source = r#"
struct Game {
    entities: Vec<i32>,
    items: Vec<i32>,
}

fn draw(entity: i32, camera: i32) {}

impl Game {
    pub fn bad_code(&mut self, camera: i32) {
        let entity = self.entities.get(0).unwrap()
        for item in self.items.iter() {
            draw(&entity, &camera)
        }
    }
}
"#;

    let warnings = parse_and_lint(source);

    // Should detect: &mut self, .unwrap(), .iter(), &entity, &camera
    assert!(
        warnings.len() >= 4,
        "Should detect at least 4 patterns - got {}: {:?}",
        warnings.len(),
        warnings.iter().map(|w| &w.lint_name).collect::<Vec<_>>()
    );

    let codes: std::collections::HashSet<_> = warnings.iter().map(|w| w.lint_name.as_str()).collect();
    assert!(codes.contains("W0001"), "Should detect W0001 (explicit ownership)");
    assert!(codes.contains("W0002"), "Should detect W0002 (unwrap)");
    assert!(codes.contains("W0003"), "Should detect W0003 (iter)");
    assert!(codes.contains("W0004"), "Should detect W0004 (explicit borrow)");
}

#[test]
fn test_suggestions_are_helpful() {
    let source = r#"
struct Counter {
    value: i32,
}

impl Counter {
    pub fn update(&mut self, dt: f32) {
        self.value = self.value + 1
    }
}
"#;

    let warnings = parse_and_lint(source);

    let w0001 = warnings.iter().find(|w| w.lint_name == "W0001").expect("Should have W0001");
    assert!(w0001.suggestion.is_some(), "Should have suggestion");
    assert!(
        w0001.suggestion.as_ref().unwrap().contains("self"),
        "Suggestion should mention self"
    );
}

#[test]
fn test_extern_fn_no_warning() {
    let source = r#"
extern fn rust_callback(data: &str) -> i32;

pub fn call_rust(s: String) -> i32 {
    rust_callback(s)
}
"#;

    let warnings = parse_and_lint(source);

    // Extern fn params may have explicit refs - we skip checking extern fn params
    // The rust_callback has &str - but that's in the extern declaration
    // Our linter checks func.parameters for non-extern. The extern fn's params
    // are in the extern declaration - we need to not warn on those.
    // Actually we're iterating func.parameters - and we skip when func.is_extern.
    // So we shouldn't warn on the extern fn's &str param. Good.
    let w0001_count = warnings.iter().filter(|w| w.lint_name == "W0001").count();
    assert_eq!(
        w0001_count, 0,
        "Should NOT warn on extern fn parameters - got {} W0001",
        w0001_count
    );
}
