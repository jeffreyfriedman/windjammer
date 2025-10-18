//! Math types and utilities for game development
//!
//! Re-exports from `glam` with some convenience additions

// Re-export glam types
pub use glam::{
    DVec2, DVec3, DVec4, IVec2, IVec3, IVec4, Mat2, Mat3, Mat3A, Mat4, Quat, UVec2, UVec3, UVec4,
    Vec2, Vec3, Vec3A, Vec4,
};

/// Common math constants
pub mod consts {
    pub const PI: f32 = std::f32::consts::PI;
    pub const TAU: f32 = std::f32::consts::TAU;
    pub const E: f32 = std::f32::consts::E;
    pub const SQRT_2: f32 = std::f32::consts::SQRT_2;
}

/// Lerp between two values
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Clamp a value between min and max
#[inline]
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

/// Convert degrees to radians
#[inline]
pub fn deg_to_rad(degrees: f32) -> f32 {
    degrees * (consts::PI / 180.0)
}

/// Convert radians to degrees
#[inline]
pub fn rad_to_deg(radians: f32) -> f32 {
    radians * (180.0 / consts::PI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-5.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_deg_rad_conversion() {
        let deg = 90.0;
        let rad = deg_to_rad(deg);
        assert!((rad - consts::PI / 2.0).abs() < 0.0001);
        assert!((rad_to_deg(rad) - deg).abs() < 0.0001);
    }

    #[test]
    fn test_vec2_creation() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_vec3_creation() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }
}
