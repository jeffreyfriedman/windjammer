#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Dogfooding ownership — trait methods and default bodies.
#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// TEST 5: Trait Method Self Inference Gap
//
// Real game code: systems.wj defines a System trait with abstract methods:
//   fn name(self) -> string
//   fn update(self, dt: f32)
//   fn is_enabled(self) -> bool
//   fn priority(self) -> i32 { 0 }  // has default body
//
// The compiler generates bare `self` for abstract methods (name, update, is_enabled)
// but correctly generates `&self` for priority (which has a default body).
//
// This is a compiler bug: abstract trait methods should NEVER use bare `self`
// (which moves ownership) unless explicitly requested. The compiler should:
//   - Default abstract methods to `&self` (borrowed)
//   - Upgrade to `&mut self` if ANY impl body mutates self for that method
//   - Impls must match the inferred trait signature
//
// This tests the Windjammer philosophy: the compiler infers ownership, not the user.
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_abstract_method_self_inference() {
    let source = r#"
pub trait System {
    fn name(self) -> string
    fn update(self, dt: f32)
    fn is_enabled(self) -> bool
    fn priority(self) -> i32 {
        0
    }
}

pub struct PhysicsSystem {
    enabled: bool,
    gravity: f32,
}

impl PhysicsSystem {
    pub fn new(gravity: f32) -> PhysicsSystem {
        PhysicsSystem { enabled: true, gravity: gravity }
    }
}

impl System for PhysicsSystem {
    fn name(self) -> string {
        "PhysicsSystem".to_string()
    }
    fn update(self, dt: f32) {
        // read-only: just reads self.gravity
        let force = self.gravity * dt
    }
    fn is_enabled(self) -> bool {
        self.enabled
    }
    fn priority(self) -> i32 {
        100
    }
}

pub struct RenderSystem {
    enabled: bool,
    draw_calls: i32,
}

impl RenderSystem {
    pub fn new() -> RenderSystem {
        RenderSystem { enabled: true, draw_calls: 0 }
    }
}

impl System for RenderSystem {
    fn name(self) -> string {
        "RenderSystem".to_string()
    }
    fn update(self, dt: f32) {
        // MUTATES self: resets draw_calls
        self.draw_calls = 0
    }
    fn is_enabled(self) -> bool {
        self.enabled
    }
    fn priority(self) -> i32 {
        -100
    }
}
"#;

    let (generated, stderr) = test_utils::compile_via_cli_with_stderr(source);
    eprintln!("=== Generated Code ===\n{}", generated);
    eprintln!("=== Compiler Stderr ===\n{}", stderr);

    // CRITICAL: Abstract trait methods must NOT use bare `self`
    // bare `self` moves ownership, making the trait non-object-safe
    // and preventing calling the method more than once on the same object

    // The trait definition should use &self for read-only methods
    assert!(
        generated.contains("fn name(&self) -> String"),
        "COMPILER BUG: Trait method 'name' should be '&self' (read-only in all impls).\n\
         Bare 'self' moves ownership, which is almost never intended for trait methods.\n\
         Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn is_enabled(&self) -> bool"),
        "COMPILER BUG: Trait method 'is_enabled' should be '&self' (read-only in all impls).\n\
         Generated:\n{}",
        generated
    );

    // update() should be &mut self because RenderSystem::update mutates self.draw_calls
    assert!(
        generated.contains("fn update(&mut self, dt: f32)"),
        "COMPILER BUG: Trait method 'update' should be '&mut self' because RenderSystem::update \
         mutates self.draw_calls.\n\
         Generated:\n{}",
        generated
    );

    // priority() has a default body and should be &self (read-only default)
    assert!(
        generated.contains("fn priority(&self) -> i32"),
        "COMPILER BUG: Trait method 'priority' should be '&self' (default impl is read-only).\n\
         Generated:\n{}",
        generated
    );

    // Impl methods must match the trait signature
    // PhysicsSystem::update should be &mut self (matching trait, even though body is read-only)
    assert!(
        generated.contains("fn update(&mut self, dt: f32)"),
        "COMPILER BUG: Impl method 'update' must match trait signature '&mut self'.\n\
         Generated:\n{}",
        generated
    );

    // name and is_enabled impls should be &self (matching trait)
    // Count occurrences to verify both trait AND impl use &self
    let name_ref_count = generated.matches("fn name(&self) -> String").count();
    assert!(
        name_ref_count >= 2,
        "COMPILER BUG: Expected at least 2 occurrences of 'fn name(&self) -> String' \
         (trait + impls), found {}.\nGenerated:\n{}",
        name_ref_count,
        generated
    );
}

// ============================================================================
// TEST 6: Trailing Semicolon on Return Expressions in Default Trait Methods
//
// In Rust, the last expression in a block must NOT have a trailing semicolon
// if it's the return value. `fn priority(&self) -> i32 { 0; }` is a type error
// because `0;` evaluates to `()` (unit), not `i32`.
//
// The correct code is `fn priority(&self) -> i32 { 0 }` (no semicolon).
//
// This affects default trait methods, match arm bodies, if/else return values,
// and any block where the last expression is the implicit return.
//
// THE WINDJAMMER WAY: The compiler generates correct Rust code.
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_default_trait_method_return_no_trailing_semicolon() {
    let source = r#"
pub trait Configurable {
    fn default_value(self) -> i32 {
        42
    }

    fn default_name(self) -> string {
        "unnamed".to_string()
    }

    fn is_valid(self) -> bool {
        true
    }

    fn compute(self, x: f32) -> f32 {
        x * 2.0
    }
}
"#;

    let (generated, stderr) = test_utils::compile_via_cli_with_stderr(source);
    eprintln!("=== Generated Code ===\n{}", generated);
    eprintln!("=== Compiler Stderr ===\n{}", stderr);

    // The return expression must NOT have a trailing semicolon
    // Good: `fn default_value(&self) -> i32 { 42 }`
    // Bad:  `fn default_value(&self) -> i32 { 42; }`

    // Check that 42 is returned without semicolon
    assert!(
        generated.contains("42\n") || generated.contains("42 }") || generated.contains("42\r\n"),
        "COMPILER BUG: Return expression '42' in default trait method should NOT have trailing semicolon.\n\
         `42;` evaluates to `()`, not `i32`, causing E0308.\n\
         Generated:\n{}",
        generated
    );

    // Specifically check it doesn't have the bad pattern
    // Find the default_value method body and check the return expression
    let has_semicolon_42 = generated.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "42;" && !trimmed.starts_with("//")
    });
    assert!(
        !has_semicolon_42,
        "COMPILER BUG: Found '42;' with trailing semicolon in default trait method.\n\
         This causes E0308: expected `i32`, found `()`.\n\
         Generated:\n{}",
        generated
    );

    // Check that true is returned without semicolon
    let has_semicolon_true = generated.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "true;" && !trimmed.starts_with("//")
    });
    assert!(
        !has_semicolon_true,
        "COMPILER BUG: Found 'true;' with trailing semicolon in default trait method.\n\
         Generated:\n{}",
        generated
    );
}
