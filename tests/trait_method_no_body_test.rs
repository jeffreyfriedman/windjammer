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

//! TDD Test: Trait method declarations without bodies
//! WINDJAMMER PHILOSOPHY: Omit `self` on all instance methods; compiler infers &self vs &mut self
//! from body analysis and impl merging — never from method names or void-return defaults.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_no_body_single() {
    let code = r#"
    pub trait Drawable {
        fn draw();
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("fn draw(&self);"),
        "Abstract trait method without body defaults to &self. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_no_body_multiple() {
    // Trait-only, no impl: abstract methods default to &self (safe object-safe default).
    let code = r#"
    pub trait GameLoop {
        fn init();
        fn update(delta: f32);
        fn render();
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("fn init(&self);"),
        "Expected init(&self) for abstract method. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn update(&self, delta: f32);"),
        "Expected update(&self, ...) for abstract method. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn render(&self);"),
        "Expected render(&self) for abstract method. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_mixed_bodies() {
    let code = r#"
    pub trait Updatable {
        fn update();
        
        fn tick() {
            self.update()
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("fn update(&self);"),
        "Abstract method without body defaults to &self. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn tick(&self) {"),
        "Calling &self method on self does not require &mut self. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_with_no_body_trait() {
    // Impl body mutates self — infer_trait_signatures_from_impls upgrades trait contract.
    let code = r#"
    pub trait Drawable {
        fn draw();
        fn update(delta: f32);
    }
    
    pub struct Sprite {
        pub x: f32,
        pub y: f32,
    }
    
    impl Drawable for Sprite {
        fn draw() {
            let _pos = self.x + self.y
        }
        
        fn update(delta: f32) {
            self.x = self.x + delta;
            self.y = self.y + delta
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("fn draw(&self);"),
        "Read-only impl keeps trait at &self. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn update(&mut self, delta: f32);"),
        "Mutating impl upgrades trait to &mut self. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn draw(&self) {") || generated.contains("pub fn draw(&self) {"),
        "Impl method should have body. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_with_return_type() {
    let code = r#"
    pub trait Calculator {
        fn add(a: int, b: int) -> int;
        fn multiply(a: int, b: int) -> int;
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("fn add(&self, a: i64, b: i64) -> i64;"),
        "Method with return type should end with semicolon. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn multiply(&self, a: i64, b: i64) -> i64;"),
        "Method with return type should end with semicolon. Generated:\n{}",
        generated
    );
}
