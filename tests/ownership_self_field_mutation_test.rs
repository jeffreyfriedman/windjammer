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

// TDD Test: self.field.method() Should Infer &mut self
//
// COMPILER BUG: When a method calls self.field.method() where field.method()
// requires &mut, the outer method should be inferred as &mut self.
//
// Current behavior: Infers &self (WRONG!)
// Expected behavior: Infers &mut self (CORRECT!)
//
// This pattern causes E0596 errors when self.field.method() requires &mut self.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_self_field_update_key_infers_mut_self() {
    // Reproduces bug: self.keyboard.update_key() should infer &mut self
    let source = r#"
struct KeyboardState {
    w_down: bool,
}

impl KeyboardState {
    fn new() -> KeyboardState {
        KeyboardState { w_down: false }
    }
    
    fn update_key(self, is_down: bool) {
        self.w_down = is_down
    }
}

struct Game {
    keyboard: KeyboardState,
}

impl Game {
    fn new() -> Game {
        Game { keyboard: KeyboardState::new() }
    }
    
    // BUG: This should be inferred as &mut self because:
    // 1. It calls self.keyboard.update_key()
    // 2. update_key() mutates self (self.w_down = ...)
    // 3. Therefore, self.keyboard needs &mut
    // 4. Therefore, outer self needs &mut
    fn poll_input(self) {
        self.keyboard.update_key(true)
    }
}
"#;

    let rust_code = match test_utils::compile_single_result(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    println!("Generated Rust:\n{}", rust_code);

    // Should infer &mut self for poll_input (fn or pub fn)
    let has_mut_self = rust_code.contains("fn poll_input(&mut self)")
        || rust_code.contains("pub fn poll_input(&mut self)");

    assert!(
        has_mut_self,
        "poll_input should be inferred as &mut self (calls mutating method on field)\n\nGenerated:\n{}", 
        rust_code
    );
}

#[test]
fn test_nested_field_mutation_inference() {
    // More complex: self.field1.field2.method()
    let source = r#"
struct Inner {
    value: i32,
}

impl Inner {
    fn set_value(self, v: i32) {
        self.value = v
    }
}

struct Middle {
    inner: Inner,
}

impl Middle {
    fn update_inner(self, v: i32) {
        self.inner.set_value(v)
    }
}

struct Outer {
    middle: Middle,
}

impl Outer {
    fn update_nested(self, v: i32) {
        self.middle.update_inner(v)
    }
}
"#;

    let rust_code = match test_utils::compile_single_result(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // All methods should be &mut self (transitive mutation)
    let has_update_nested = rust_code.contains("fn update_nested(&mut self")
        || rust_code.contains("pub fn update_nested(&mut self");
    let has_update_inner = rust_code.contains("fn update_inner(&mut self")
        || rust_code.contains("pub fn update_inner(&mut self");
    let has_set_value = rust_code.contains("fn set_value(&mut self")
        || rust_code.contains("pub fn set_value(&mut self");

    assert!(
        has_update_nested,
        "update_nested should infer &mut self\n\nGenerated:\n{}",
        rust_code
    );
    assert!(
        has_update_inner,
        "update_inner should infer &mut self\n\nGenerated:\n{}",
        rust_code
    );
    assert!(
        has_set_value,
        "set_value should infer &mut self\n\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_self_field_extern_method_infers_mut_self() {
    // self.camera.look_at() where Camera3D comes from an extern impl signature stub
    let source = r#"
// Simulate external type (from engine)
extern struct Camera3D {}

extern impl Camera3D {
    fn look_at(self, x: f32, y: f32, z: f32, tx: f32, ty: f32, tz: f32) {}
}

struct Game {
    camera: Camera3D,
}

impl Game {
    // BUG: Should infer &mut self because:
    // 1. Calls self.camera.look_at()
    // 2. look_at() is extern and marked as taking self (owned)
    // 3. Using self.camera requires &mut to avoid move
    fn update_camera(self, dt: f32) {
        self.camera.look_at(0.0, 5.0, 10.0, 0.0, 0.0, 0.0)
    }
}
"#;

    let rust_code = match test_utils::compile_single_result(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    println!("Generated Rust:\n{}", rust_code);

    let has_mut_self = rust_code.contains("fn update_camera(&mut self")
        || rust_code.contains("pub fn update_camera(&mut self");

    assert!(
        has_mut_self,
        "update_camera should be inferred as &mut self (calls method on field)\n\nGenerated:\n{}",
        rust_code
    );
}
