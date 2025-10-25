//! Transform component for 2D and 3D positioning

use crate::math::{Mat4, Quat, Vec2, Vec3};

/// 2D Transform component
#[derive(Debug, Clone, Copy)]
pub struct Transform2D {
    /// Position in 2D space
    pub position: Vec2,
    /// Rotation in radians
    pub rotation: f32,
    /// Scale (uniform or non-uniform)
    pub scale: Vec2,
}

impl Transform2D {
    /// Create a new transform at the origin
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }

    /// Create a transform at a specific position
    pub fn at(position: Vec2) -> Self {
        Self {
            position,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }

    /// Set position
    pub fn with_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    /// Set rotation (in radians)
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set rotation (in degrees)
    pub fn with_rotation_degrees(mut self, degrees: f32) -> Self {
        self.rotation = degrees.to_radians();
        self
    }

    /// Set scale
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Set uniform scale
    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = Vec2::new(scale, scale);
        self
    }

    /// Translate by a vector
    pub fn translate(&mut self, delta: Vec2) {
        self.position += delta;
    }

    /// Rotate by an angle (in radians)
    pub fn rotate(&mut self, delta: f32) {
        self.rotation += delta;
    }

    /// Rotate by an angle (in degrees)
    pub fn rotate_degrees(&mut self, degrees: f32) {
        self.rotation += degrees.to_radians();
    }

    /// Scale by a factor
    pub fn scale_by(&mut self, factor: Vec2) {
        self.scale.x *= factor.x;
        self.scale.y *= factor.y;
    }

    /// Get the forward direction (based on rotation)
    pub fn forward(&self) -> Vec2 {
        Vec2::new(self.rotation.cos(), self.rotation.sin())
    }

    /// Get the right direction (based on rotation)
    pub fn right(&self) -> Vec2 {
        Vec2::new(-self.rotation.sin(), self.rotation.cos())
    }

    /// Convert to a 3x3 transformation matrix (for 2D rendering)
    pub fn to_matrix(&self) -> [[f32; 3]; 3] {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();

        [
            [cos * self.scale.x, -sin * self.scale.x, self.position.x],
            [sin * self.scale.y, cos * self.scale.y, self.position.y],
            [0.0, 0.0, 1.0],
        ]
    }

    /// Transform a point from local space to world space
    pub fn transform_point(&self, point: Vec2) -> Vec2 {
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();

        Vec2::new(
            point.x * cos * self.scale.x - point.y * sin * self.scale.x + self.position.x,
            point.x * sin * self.scale.y + point.y * cos * self.scale.y + self.position.y,
        )
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::new()
    }
}

/// 3D Transform component
#[derive(Debug, Clone, Copy)]
pub struct Transform3D {
    /// Position in 3D space
    pub position: Vec3,
    /// Rotation as a quaternion
    pub rotation: Quat,
    /// Scale (uniform or non-uniform)
    pub scale: Vec3,
}

impl Transform3D {
    /// Create a new transform at the origin
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Create a transform at a specific position
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    /// Set position
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Set rotation
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set scale
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Set uniform scale
    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = Vec3::new(scale, scale, scale);
        self
    }

    /// Translate by a vector
    pub fn translate(&mut self, delta: Vec3) {
        self.position += delta;
    }

    /// Rotate by a quaternion
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    /// Scale by a factor
    pub fn scale_by(&mut self, factor: Vec3) {
        self.scale.x *= factor.x;
        self.scale.y *= factor.y;
        self.scale.z *= factor.z;
    }

    /// Get the forward direction
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::new(0.0, 0.0, -1.0)
    }

    /// Get the right direction
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::new(1.0, 0.0, 0.0)
    }

    /// Get the up direction
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::new(0.0, 1.0, 0.0)
    }

    /// Convert to a 4x4 transformation matrix
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Look at a target position
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let forward = (target - self.position).normalize();
        let right = forward.cross(up).normalize();
        let up = right.cross(forward);

        // Create rotation matrix from basis vectors
        let mat = glam::Mat3::from_cols(right, up, -forward);
        self.rotation = Quat::from_mat3(&mat);
    }
}

impl Default for Transform3D {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform2d_creation() {
        let transform = Transform2D::new();
        assert_eq!(transform.position, Vec2::ZERO);
        assert_eq!(transform.rotation, 0.0);
        assert_eq!(transform.scale, Vec2::ONE);
    }

    #[test]
    fn test_transform2d_builder() {
        let transform = Transform2D::new()
            .with_position(Vec2::new(10.0, 20.0))
            .with_rotation(std::f32::consts::PI / 2.0)
            .with_scale(Vec2::new(2.0, 2.0));

        assert_eq!(transform.position, Vec2::new(10.0, 20.0));
        assert!((transform.rotation - std::f32::consts::PI / 2.0).abs() < 0.001);
        assert_eq!(transform.scale, Vec2::new(2.0, 2.0));
    }

    #[test]
    fn test_transform2d_translate() {
        let mut transform = Transform2D::at(Vec2::new(5.0, 5.0));
        transform.translate(Vec2::new(3.0, 2.0));
        assert_eq!(transform.position, Vec2::new(8.0, 7.0));
    }

    #[test]
    fn test_transform2d_rotate() {
        let mut transform = Transform2D::new();
        transform.rotate(std::f32::consts::PI / 4.0);
        assert!((transform.rotation - std::f32::consts::PI / 4.0).abs() < 0.001);
    }

    #[test]
    fn test_transform2d_forward() {
        let transform = Transform2D::new().with_rotation(0.0);
        let forward = transform.forward();
        assert!((forward.x - 1.0).abs() < 0.001);
        assert!(forward.y.abs() < 0.001);
    }

    #[test]
    fn test_transform2d_transform_point() {
        let transform = Transform2D::at(Vec2::new(10.0, 10.0));
        let point = transform.transform_point(Vec2::new(5.0, 5.0));
        assert_eq!(point, Vec2::new(15.0, 15.0));
    }

    #[test]
    fn test_transform3d_creation() {
        let transform = Transform3D::new();
        assert_eq!(transform.position, Vec3::ZERO);
        assert_eq!(transform.rotation, Quat::IDENTITY);
        assert_eq!(transform.scale, Vec3::ONE);
    }

    #[test]
    fn test_transform3d_builder() {
        let transform = Transform3D::new()
            .with_position(Vec3::new(1.0, 2.0, 3.0))
            .with_uniform_scale(2.0);

        assert_eq!(transform.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(transform.scale, Vec3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_transform3d_translate() {
        let mut transform = Transform3D::at(Vec3::new(1.0, 2.0, 3.0));
        transform.translate(Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(transform.position, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn test_transform3d_directions() {
        let transform = Transform3D::new();
        let forward = transform.forward();
        let right = transform.right();
        let up = transform.up();

        // Default forward is -Z
        assert!((forward.z + 1.0).abs() < 0.001);
        // Default right is +X
        assert!((right.x - 1.0).abs() < 0.001);
        // Default up is +Y
        assert!((up.y - 1.0).abs() < 0.001);
    }
}
