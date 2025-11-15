//! Unit tests for Math System
//!
//! Tests Vec2, Vec3, Vec4, Mat4, Quat, and utility functions.

use windjammer_game_framework::math::*;

// ============================================================================
// Vec2 Tests
// ============================================================================

#[test]
fn test_vec2_creation() {
    let v = Vec2::new(1.0, 2.0);
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 2.0);
    println!("✅ Vec2 created: ({}, {})", v.x, v.y);
}

#[test]
fn test_vec2_zero() {
    let v = Vec2::ZERO;
    assert_eq!(v.x, 0.0);
    assert_eq!(v.y, 0.0);
    println!("✅ Vec2::ZERO");
}

#[test]
fn test_vec2_one() {
    let v = Vec2::ONE;
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 1.0);
    println!("✅ Vec2::ONE");
}

#[test]
fn test_vec2_addition() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let c = a + b;
    assert_eq!(c.x, 4.0);
    assert_eq!(c.y, 6.0);
    println!("✅ Vec2 addition: ({}, {}) + ({}, {}) = ({}, {})", a.x, a.y, b.x, b.y, c.x, c.y);
}

#[test]
fn test_vec2_subtraction() {
    let a = Vec2::new(5.0, 7.0);
    let b = Vec2::new(2.0, 3.0);
    let c = a - b;
    assert_eq!(c.x, 3.0);
    assert_eq!(c.y, 4.0);
    println!("✅ Vec2 subtraction");
}

#[test]
fn test_vec2_scalar_multiplication() {
    let v = Vec2::new(2.0, 3.0);
    let scaled = v * 2.0;
    assert_eq!(scaled.x, 4.0);
    assert_eq!(scaled.y, 6.0);
    println!("✅ Vec2 scalar multiplication");
}

#[test]
fn test_vec2_length() {
    let v = Vec2::new(3.0, 4.0);
    let len = v.length();
    assert!((len - 5.0).abs() < 0.0001);
    println!("✅ Vec2 length: {}", len);
}

#[test]
fn test_vec2_normalize() {
    let v = Vec2::new(3.0, 4.0);
    let normalized = v.normalize();
    let len = normalized.length();
    assert!((len - 1.0).abs() < 0.0001);
    println!("✅ Vec2 normalize: length = {}", len);
}

#[test]
fn test_vec2_dot_product() {
    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);
    let dot = a.dot(b);
    assert_eq!(dot, 11.0); // 1*3 + 2*4 = 11
    println!("✅ Vec2 dot product: {}", dot);
}

#[test]
fn test_vec2_distance() {
    let a = Vec2::new(0.0, 0.0);
    let b = Vec2::new(3.0, 4.0);
    let dist = a.distance(b);
    assert!((dist - 5.0).abs() < 0.0001);
    println!("✅ Vec2 distance: {}", dist);
}

// ============================================================================
// Vec3 Tests
// ============================================================================

#[test]
fn test_vec3_creation() {
    let v = Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 2.0);
    assert_eq!(v.z, 3.0);
    println!("✅ Vec3 created: ({}, {}, {})", v.x, v.y, v.z);
}

#[test]
fn test_vec3_zero() {
    let v = Vec3::ZERO;
    assert_eq!(v.x, 0.0);
    assert_eq!(v.y, 0.0);
    assert_eq!(v.z, 0.0);
    println!("✅ Vec3::ZERO");
}

#[test]
fn test_vec3_one() {
    let v = Vec3::ONE;
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 1.0);
    assert_eq!(v.z, 1.0);
    println!("✅ Vec3::ONE");
}

#[test]
fn test_vec3_axes() {
    assert_eq!(Vec3::X, Vec3::new(1.0, 0.0, 0.0));
    assert_eq!(Vec3::Y, Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(Vec3::Z, Vec3::new(0.0, 0.0, 1.0));
    println!("✅ Vec3 axes (X, Y, Z)");
}

#[test]
fn test_vec3_addition() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    let c = a + b;
    assert_eq!(c.x, 5.0);
    assert_eq!(c.y, 7.0);
    assert_eq!(c.z, 9.0);
    println!("✅ Vec3 addition");
}

#[test]
fn test_vec3_subtraction() {
    let a = Vec3::new(5.0, 7.0, 9.0);
    let b = Vec3::new(2.0, 3.0, 4.0);
    let c = a - b;
    assert_eq!(c.x, 3.0);
    assert_eq!(c.y, 4.0);
    assert_eq!(c.z, 5.0);
    println!("✅ Vec3 subtraction");
}

#[test]
fn test_vec3_scalar_multiplication() {
    let v = Vec3::new(2.0, 3.0, 4.0);
    let scaled = v * 2.0;
    assert_eq!(scaled.x, 4.0);
    assert_eq!(scaled.y, 6.0);
    assert_eq!(scaled.z, 8.0);
    println!("✅ Vec3 scalar multiplication");
}

#[test]
fn test_vec3_length() {
    let v = Vec3::new(2.0, 3.0, 6.0);
    let len = v.length();
    assert!((len - 7.0).abs() < 0.0001);
    println!("✅ Vec3 length: {}", len);
}

#[test]
fn test_vec3_normalize() {
    let v = Vec3::new(3.0, 4.0, 0.0);
    let normalized = v.normalize();
    let len = normalized.length();
    assert!((len - 1.0).abs() < 0.0001);
    println!("✅ Vec3 normalize: length = {}", len);
}

#[test]
fn test_vec3_dot_product() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    let dot = a.dot(b);
    assert_eq!(dot, 32.0); // 1*4 + 2*5 + 3*6 = 32
    println!("✅ Vec3 dot product: {}", dot);
}

#[test]
fn test_vec3_cross_product() {
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(0.0, 1.0, 0.0);
    let cross = a.cross(b);
    assert_eq!(cross, Vec3::new(0.0, 0.0, 1.0));
    println!("✅ Vec3 cross product: ({}, {}, {})", cross.x, cross.y, cross.z);
}

#[test]
fn test_vec3_distance() {
    let a = Vec3::new(0.0, 0.0, 0.0);
    let b = Vec3::new(3.0, 4.0, 0.0);
    let dist = a.distance(b);
    assert!((dist - 5.0).abs() < 0.0001);
    println!("✅ Vec3 distance: {}", dist);
}

// ============================================================================
// Vec4 Tests
// ============================================================================

#[test]
fn test_vec4_creation() {
    let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 2.0);
    assert_eq!(v.z, 3.0);
    assert_eq!(v.w, 4.0);
    println!("✅ Vec4 created: ({}, {}, {}, {})", v.x, v.y, v.z, v.w);
}

#[test]
fn test_vec4_zero() {
    let v = Vec4::ZERO;
    assert_eq!(v.x, 0.0);
    assert_eq!(v.y, 0.0);
    assert_eq!(v.z, 0.0);
    assert_eq!(v.w, 0.0);
    println!("✅ Vec4::ZERO");
}

#[test]
fn test_vec4_addition() {
    let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
    let c = a + b;
    assert_eq!(c.x, 6.0);
    assert_eq!(c.y, 8.0);
    assert_eq!(c.z, 10.0);
    assert_eq!(c.w, 12.0);
    println!("✅ Vec4 addition");
}

// ============================================================================
// Mat4 Tests
// ============================================================================

#[test]
fn test_mat4_identity() {
    let m = Mat4::IDENTITY;
    let v = Vec4::new(1.0, 2.0, 3.0, 1.0);
    let result = m * v;
    assert_eq!(result, v);
    println!("✅ Mat4 identity");
}

#[test]
fn test_mat4_translation() {
    let m = Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0));
    let v = Vec4::new(0.0, 0.0, 0.0, 1.0);
    let result = m * v;
    assert!((result.x - 10.0).abs() < 0.0001);
    assert!((result.y - 20.0).abs() < 0.0001);
    assert!((result.z - 30.0).abs() < 0.0001);
    println!("✅ Mat4 translation");
}

#[test]
fn test_mat4_scale() {
    let m = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let v = Vec4::new(1.0, 1.0, 1.0, 1.0);
    let result = m * v;
    assert!((result.x - 2.0).abs() < 0.0001);
    assert!((result.y - 3.0).abs() < 0.0001);
    assert!((result.z - 4.0).abs() < 0.0001);
    println!("✅ Mat4 scale");
}

#[test]
fn test_mat4_rotation_x() {
    let m = Mat4::from_rotation_x(std::f32::consts::PI / 2.0); // 90 degrees
    let v = Vec4::new(0.0, 1.0, 0.0, 1.0);
    let result = m * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 0.0).abs() < 0.0001);
    assert!((result.z - 1.0).abs() < 0.0001);
    println!("✅ Mat4 rotation X");
}

#[test]
fn test_mat4_rotation_y() {
    let m = Mat4::from_rotation_y(std::f32::consts::PI / 2.0); // 90 degrees
    let v = Vec4::new(1.0, 0.0, 0.0, 1.0);
    let result = m * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 0.0).abs() < 0.0001);
    assert!((result.z - -1.0).abs() < 0.0001);
    println!("✅ Mat4 rotation Y");
}

#[test]
fn test_mat4_rotation_z() {
    let m = Mat4::from_rotation_z(std::f32::consts::PI / 2.0); // 90 degrees
    let v = Vec4::new(1.0, 0.0, 0.0, 1.0);
    let result = m * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 1.0).abs() < 0.0001);
    assert!((result.z - 0.0).abs() < 0.0001);
    println!("✅ Mat4 rotation Z");
}

#[test]
fn test_mat4_multiplication() {
    let m1 = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0));
    let m2 = Mat4::from_scale(Vec3::new(2.0, 2.0, 2.0));
    let m = m1 * m2;
    let v = Vec4::new(1.0, 1.0, 1.0, 1.0);
    let result = m * v;
    assert!((result.x - 12.0).abs() < 0.0001); // Translate then scale
    assert!((result.y - 2.0).abs() < 0.0001);
    assert!((result.z - 2.0).abs() < 0.0001);
    println!("✅ Mat4 multiplication");
}

// ============================================================================
// Quaternion Tests
// ============================================================================

#[test]
fn test_quat_identity() {
    let q = Quat::IDENTITY;
    let v = Vec3::new(1.0, 2.0, 3.0);
    let result = q * v;
    assert_eq!(result, v);
    println!("✅ Quat identity");
}

#[test]
fn test_quat_from_axis_angle() {
    let q = Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI / 2.0);
    let v = Vec3::new(1.0, 0.0, 0.0);
    let result = q * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 0.0).abs() < 0.0001);
    assert!((result.z - -1.0).abs() < 0.0001);
    println!("✅ Quat from axis-angle");
}

#[test]
fn test_quat_from_rotation_x() {
    let q = Quat::from_rotation_x(std::f32::consts::PI / 2.0);
    let v = Vec3::new(0.0, 1.0, 0.0);
    let result = q * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 0.0).abs() < 0.0001);
    assert!((result.z - 1.0).abs() < 0.0001);
    println!("✅ Quat rotation X");
}

#[test]
fn test_quat_from_rotation_y() {
    let q = Quat::from_rotation_y(std::f32::consts::PI / 2.0);
    let v = Vec3::new(1.0, 0.0, 0.0);
    let result = q * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 0.0).abs() < 0.0001);
    assert!((result.z - -1.0).abs() < 0.0001);
    println!("✅ Quat rotation Y");
}

#[test]
fn test_quat_from_rotation_z() {
    let q = Quat::from_rotation_z(std::f32::consts::PI / 2.0);
    let v = Vec3::new(1.0, 0.0, 0.0);
    let result = q * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 1.0).abs() < 0.0001);
    assert!((result.z - 0.0).abs() < 0.0001);
    println!("✅ Quat rotation Z");
}

#[test]
fn test_quat_multiplication() {
    let q1 = Quat::from_rotation_y(std::f32::consts::PI / 4.0); // 45 degrees
    let q2 = Quat::from_rotation_y(std::f32::consts::PI / 4.0); // 45 degrees
    let q = q1 * q2; // Should be 90 degrees
    let v = Vec3::new(1.0, 0.0, 0.0);
    let result = q * v;
    assert!((result.x - 0.0).abs() < 0.0001);
    assert!((result.y - 0.0).abs() < 0.0001);
    assert!((result.z - -1.0).abs() < 0.0001);
    println!("✅ Quat multiplication");
}

#[test]
fn test_quat_slerp() {
    let q1 = Quat::IDENTITY;
    let q2 = Quat::from_rotation_y(std::f32::consts::PI / 2.0);
    let q_mid = q1.slerp(q2, 0.5);
    let v = Vec3::new(1.0, 0.0, 0.0);
    let result = q_mid * v;
    // At 50% rotation (45 degrees), x and z should be equal
    assert!((result.x.abs() - result.z.abs()).abs() < 0.0001);
    println!("✅ Quat slerp (spherical interpolation)");
}

// ============================================================================
// Utility Function Tests
// ============================================================================

#[test]
fn test_lerp() {
    assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
    assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
    assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    println!("✅ lerp function");
}

#[test]
fn test_clamp() {
    assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
    assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
    assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    println!("✅ clamp function");
}

#[test]
fn test_deg_to_rad() {
    let rad = deg_to_rad(90.0);
    assert!((rad - std::f32::consts::PI / 2.0).abs() < 0.0001);
    println!("✅ deg_to_rad: 90° = {} rad", rad);
}

#[test]
fn test_rad_to_deg() {
    let deg = rad_to_deg(std::f32::consts::PI / 2.0);
    assert!((deg - 90.0).abs() < 0.0001);
    println!("✅ rad_to_deg: π/2 = {}°", deg);
}

// ============================================================================
// Transform Tests
// ============================================================================

#[test]
fn test_transform_creation() {
    let t = Transform::new();
    assert_eq!(t.position, Vec2::ZERO);
    assert_eq!(t.rotation, 0.0);
    assert_eq!(t.scale, Vec2::ONE);
    println!("✅ Transform created");
}

#[test]
fn test_transform_at_position() {
    let t = Transform::at_position(Vec2::new(10.0, 20.0));
    assert_eq!(t.position, Vec2::new(10.0, 20.0));
    assert_eq!(t.rotation, 0.0);
    assert_eq!(t.scale, Vec2::ONE);
    println!("✅ Transform at position");
}

#[test]
fn test_transform_translate() {
    let mut t = Transform::new();
    t.translate(Vec2::new(5.0, 10.0));
    assert_eq!(t.position, Vec2::new(5.0, 10.0));
    println!("✅ Transform translate");
}

#[test]
fn test_transform_rotate() {
    let mut t = Transform::new();
    t.rotate(45.0);
    assert_eq!(t.rotation, 45.0);
    t.rotate(45.0);
    assert_eq!(t.rotation, 90.0);
    println!("✅ Transform rotate");
}

#[test]
fn test_transform_scale() {
    let mut t = Transform::new();
    t.set_scale(Vec2::new(2.0, 3.0));
    assert_eq!(t.scale, Vec2::new(2.0, 3.0));
    println!("✅ Transform scale");
}

#[test]
fn test_transform_uniform_scale() {
    let mut t = Transform::new();
    t.set_uniform_scale(2.5);
    assert_eq!(t.scale, Vec2::new(2.5, 2.5));
    println!("✅ Transform uniform scale");
}

