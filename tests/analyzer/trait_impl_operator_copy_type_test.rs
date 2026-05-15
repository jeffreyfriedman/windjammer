/// Test: Operator Trait Implementation for Copy Types
///
/// When implementing operator traits (Add, Sub, Mul, etc.) for Copy types,
/// the compiler should use `self` (owned), not `&self`, to match Rust's
/// standard library trait definitions.
///
/// This is critical for game engines where Vec2, Vec3, etc. are Copy types
/// and implement arithmetic operators.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_add_trait_copy_type_uses_owned_self() {
    let code = r#"
        use std::ops::Add
        
        struct Vec2 {
            x: f32,
            y: f32,
        }
        
        impl Add for Vec2 {
            type Output = Vec2
            
            fn add(self, other: Vec2) -> Vec2 {
                Vec2 { x: self.x + other.x, y: self.y + other.y }
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    // For operator traits on Copy types, should use `self` (owned), not `&self`
    assert!(
        generated.contains("fn add(self, other: Vec2) -> Vec2"),
        "Add trait should use owned self for Copy types, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_sub_trait_copy_type_uses_owned_self() {
    let code = r#"
        use std::ops::Sub
        
        struct Vec2 {
            x: f32,
            y: f32,
        }
        
        impl Sub for Vec2 {
            type Output = Vec2
            
            fn sub(self, other: Vec2) -> Vec2 {
                Vec2 { x: self.x - other.x, y: self.y - other.y }
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn sub(self, other: Vec2) -> Vec2"),
        "Sub trait should use owned self for Copy types, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mul_trait_copy_type_uses_owned_self() {
    let code = r#"
        use std::ops::Mul
        
        struct Vec2 {
            x: f32,
            y: f32,
        }
        
        impl Mul<f32> for Vec2 {
            type Output = Vec2
            
            fn mul(self, scalar: f32) -> Vec2 {
                Vec2 { x: self.x * scalar, y: self.y * scalar }
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn mul(self, scalar: f32) -> Vec2"),
        "Mul trait should use owned self for Copy types, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_neg_trait_copy_type_uses_owned_self() {
    let code = r#"
        use std::ops::Neg
        
        struct Vec2 {
            x: f32,
            y: f32,
        }
        
        impl Neg for Vec2 {
            type Output = Vec2
            
            fn neg(self) -> Vec2 {
                Vec2 { x: -self.x, y: -self.y }
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn neg(self) -> Vec2"),
        "Neg trait should use owned self for Copy types, got:\n{}",
        generated
    );
}
