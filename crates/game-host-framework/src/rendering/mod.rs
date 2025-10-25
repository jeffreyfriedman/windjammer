//! Graphics rendering using wgpu
//!
//! Supports both 2D and 3D rendering with multiple graphics APIs:
//! - Metal (macOS, iOS)
//! - Vulkan (cross-platform)
//! - DirectX 12 (Windows)
//! - WebGPU (web)

pub mod backend;
pub mod pipeline_2d;
pub mod pipeline_3d;
pub mod sprite;

pub use pipeline_2d::Pipeline2D;
pub use pipeline_3d::{CameraUniform, LightUniform, MaterialUniform, Pipeline3D};
pub use sprite::{Sprite, SpriteBatch};

use crate::math::{Mat4, Vec2, Vec3, Vec4};

/// Render context for drawing
pub struct RenderContext {
    // Will be implemented with wgpu
    _placeholder: (),
}

impl RenderContext {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }

    pub fn clear(&mut self, _color: Vec4) {
        // TODO: Implement with wgpu
    }

    pub fn draw_sprite(&mut self, _sprite: &Sprite, _position: Vec2) {
        // TODO: Implement with wgpu
    }

    pub fn draw_mesh(&mut self, _mesh: &Mesh, _transform: Mat4) {
        // TODO: Implement with wgpu
    }
}

impl Default for RenderContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 3D Mesh
pub struct Mesh {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
}

/// Vertex data for 3D rendering
pub struct Vertex3D {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

/// Material for rendering
pub struct Material {
    pub albedo: Vec4,
    pub metallic: f32,
    pub roughness: f32,
}

/// Camera for rendering
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 60.0,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Build view matrix (look-at)
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Build projection matrix (perspective)
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), self.aspect, self.near, self.far)
    }

    /// Build combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Build orthographic projection
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
        Mat4::orthographic_rh(left, right, bottom, top, near, far)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

/// Texture handle
pub struct Texture {
    _placeholder: (),
}

/// Generic handle for assets
pub struct Handle<T> {
    #[allow(dead_code)] // Used internally for resource tracking
    id: u64,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}
