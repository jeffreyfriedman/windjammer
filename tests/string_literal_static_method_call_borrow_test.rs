#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// Test: String literals passed to Type::method() static calls should NOT get .to_string()
/// when the method parameter is inferred as borrowed (&str).
///
/// Bug: VNode::element("hr") generates VNode::element("hr".to_string()) in Rust,
/// but the function signature is fn element(tag: String). This causes a type mismatch.
///
/// The codegen correctly suppresses .to_string() for module-qualified calls (draw::draw_text),
/// but not for type-qualified static method calls (VNode::element) in cross-file scenarios.
///
/// Scenario:
///   - Struct Builder has fn create(name: &str) where name is only read → inferred as &str
///   - Caller does Builder::create("hello") → should generate Builder::create("hello"), NOT "hello".to_string()
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Cross-file test: This is the actual failing case from windjammer-ui.
/// File A defines VNode::element(tag: String) [inferred as &str].
/// File B calls VNode::element("hr") from a free function.
/// The codegen must look up the cross-file signature to know tag is &str.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cross_file_string_literal_static_call_borrowed_param() {
    let temp_dir = TempDir::new().unwrap();

    // File A: defines the struct with a static method
    let vnode_source = r#"
pub struct VNode {
    pub handle: i64,
}

impl VNode {
    pub fn element(tag: string) -> VNode {
        VNode { handle: 1 }
    }

    pub fn text(content: string) -> VNode {
        VNode { handle: 2 }
    }

    pub fn div() -> VNode {
        VNode::element("div")
    }

    pub fn add_style(self, style: string) -> VNode {
        self
    }

    pub fn add_text(self, text: string) -> VNode {
        let text_node = VNode::text(text)
        self
    }
}
"#;

    // File B: calls VNode::element() and VNode::text() from free functions
    let helpers_source = r#"
use crate::vnode::VNode

pub fn divider() -> VNode {
    VNode::element("hr")
        .add_style("border: 0")
}

pub fn spacer() -> VNode {
    VNode::div().add_style("flex: 1")
}
"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("vnode.wj"), vnode_source).unwrap();
    fs::write(src_dir.join("helpers.wj"), helpers_source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    // Check the helpers.rs file (cross-file call site)
    let helpers_rs = temp_dir.path().join("build").join("helpers.rs");
    let generated = fs::read_to_string(&helpers_rs)
        .unwrap_or_else(|_| panic!("Failed to read generated helpers.rs"));

    // VNode::element("hr") should NOT get .to_string() because
    // element's `tag` parameter is inferred as &str (read-only)
    assert!(
        !generated.contains(r#""hr".to_string()"#),
        "Cross-file: String literal should NOT get .to_string() for borrowed param.\n\
         Expected: VNode::element(\"hr\")\n\
         Generated:\n{}",
        generated
    );

    // Also check vnode.rs for internal calls
    let vnode_rs = temp_dir.path().join("build").join("vnode.rs");
    let vnode_generated = fs::read_to_string(&vnode_rs)
        .unwrap_or_else(|_| panic!("Failed to read generated vnode.rs"));

    // VNode::element("div") inside VNode::div() should also not get .to_string()
    assert!(
        !vnode_generated.contains(r#""div".to_string()"#),
        "Same-file: String literal should NOT get .to_string() for borrowed param.\n\
         Expected: VNode::element(\"div\")\n\
         Generated:\n{}",
        vnode_generated
    );

    // add_text calls VNode::text(text) where text: &str → should not add .to_string()
    assert!(
        !vnode_generated.contains("text.to_string()"),
        "&str variable passed to &str param should NOT get .to_string().\nGenerated:\n{}",
        vnode_generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_in_type_static_call_borrowed_param() {
    let temp_dir = TempDir::new().unwrap();

    let source = r#"
extern fn external_create(name: &str) -> i64

pub struct Builder {
    pub handle: i64,
}

impl Builder {
    pub fn create(name: string) -> Builder {
        Builder { handle: external_create(name) }
    }
}

pub fn make_builder() -> Builder {
    Builder::create("hello")
}
"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("builder.wj"), source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    let builder_rs = temp_dir.path().join("build").join("builder.rs");
    let generated = fs::read_to_string(&builder_rs)
        .unwrap_or_else(|_| panic!("Failed to read generated builder.rs"));

    // The function `create` takes `name: String` but only reads it,
    // so the analyzer infers it as `&str`. The call site should pass
    // the string literal directly, not with .to_string().
    assert!(
        !generated.contains(r#""hello".to_string()"#),
        "String literal should NOT get .to_string() for borrowed string param in static call.\n\
         Expected: Builder::create(\"hello\")\n\
         Generated:\n{}",
        generated
    );

    // Verify the function signature uses &str
    assert!(
        generated.contains("name: &str"),
        "Parameter should be inferred as &str since it's only read.\nGenerated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_param_passed_to_borrowed_method_no_to_string() {
    let temp_dir = TempDir::new().unwrap();

    // This tests the case where a &str parameter is passed to another method
    // that also takes &str - no .to_string() should be added
    let source = r#"
pub struct Widget {
    pub handle: i64,
}

impl Widget {
    pub fn text(content: string) -> Widget {
        Widget { handle: 1 }
    }

    pub fn add_text(self, text: string) -> Widget {
        let text_node = Widget::text(text)
        self
    }
}
"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("widget.wj"), source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !wj_output.status.success() {
        panic!(
            "Compilation failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&wj_output.stdout),
            String::from_utf8_lossy(&wj_output.stderr)
        );
    }

    let widget_rs = temp_dir.path().join("build").join("widget.rs");
    let generated = fs::read_to_string(&widget_rs)
        .unwrap_or_else(|_| panic!("Failed to read generated widget.rs"));

    // Both `text` and `add_text` have `string` params inferred as `&str`.
    // When add_text calls Widget::text(text), no .to_string() should be added
    // since text is already &str and the target param is also &str.
    assert!(
        !generated.contains("text.to_string()"),
        "&str param passed to &str method should NOT get .to_string().\n\
         Expected: Widget::text(text)\n\
         Generated:\n{}",
        generated
    );
}
