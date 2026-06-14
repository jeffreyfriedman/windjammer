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

/// TDD Tests: Argument passed to method should not get &mut when method takes owned.
///
/// Bug: When calling `self.renderer.set_camera(camera)` where set_camera takes
/// `camera: CameraData` (owned), the compiler generates `&mut camera` instead of `camera`.
///
/// This causes E0308: expected `CameraData`, found `&mut CameraData`.
///
/// Root cause: The codegen adds `&mut` to arguments when it shouldn't.
/// The `&mut` prefix should only be added when the target parameter is `&mut T`.
#[path = "common/test_utils.rs"]
mod test_utils;
#[allow(unused_imports)]
use test_utils::compile_single;

/// When a method takes an owned parameter, passing a local variable should NOT
/// add &mut to it.
#[test]
fn test_method_owned_param_no_mut_ref() {
    let input = r#"
struct CameraData {
    x: f32,
    y: f32,
}

struct Renderer {
    active: bool,
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer { active: false }
    }

    pub fn set_camera(self, camera: CameraData) {
        self.active = true
    }
}

struct Editor {
    renderer: Renderer,
}

impl Editor {
    pub fn new() -> Editor {
        Editor { renderer: Renderer::new() }
    }

    pub fn render(self) {
        let camera = CameraData { x: 0.0, y: 0.0 }
        self.renderer.set_camera(camera)
    }
}
"#;
    let output = compile_single(input);
    // Should NOT contain &mut camera
    assert!(
        !output.contains("&mut camera"),
        "Should not add &mut when method takes owned parameter.\nGenerated:\n{}",
        output
    );
    // Should just pass camera directly
    assert!(
        output.contains("self.renderer.set_camera(camera)"),
        "Should pass camera directly as owned.\nGenerated:\n{}",
        output
    );
}

/// When a method's parameter is &mut T, THEN &mut should be added.
#[test]
fn test_method_mut_ref_param_gets_mut_ref() {
    let input = r#"
struct Data {
    value: i32,
}

struct Processor {
    count: i32,
}

impl Processor {
    pub fn new() -> Processor {
        Processor { count: 0 }
    }

    pub fn process(self, data: Data) {
        data.value = data.value + 1
        self.count = self.count + 1
    }
}
"#;
    let output = compile_single(input);
    // When data is mutated inside the function, it should take &mut Data
    assert!(
        output.contains("fn process(&mut self, data: &mut Data)"),
        "process should take &mut Data since data is mutated.\nGenerated:\n{}",
        output
    );
}
