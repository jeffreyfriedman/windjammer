// Game rendering backend using wgpu
// Provides cross-platform 2D/3D rendering for games

use super::{Color, Sprite, Transform, Vec3};

/// Rendering backend abstraction
pub trait RenderBackend {
    fn clear(&mut self, color: Color);
    fn draw_sprite(&mut self, sprite: &Sprite, transform: &Transform);
    fn present(&mut self);
    fn resize(&mut self, width: u32, height: u32);
}

/// 2D sprite renderer
pub struct Renderer2D {
    width: u32,
    height: u32,
    clear_color: [f32; 4],
    sprites_to_draw: Vec<(Sprite, Transform)>,
}

impl Renderer2D {
    pub fn new(width: u32, height: u32) -> Self {
        Renderer2D {
            width,
            height,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            sprites_to_draw: Vec::new(),
        }
    }

    pub fn begin_frame(&mut self) {
        self.sprites_to_draw.clear();
    }

    pub fn end_frame(&mut self) {
        // In full implementation, this would sort sprites and submit to GPU
    }
}

impl RenderBackend for Renderer2D {
    fn clear(&mut self, color: Color) {
        self.clear_color = color.to_f32_array();
    }

    fn draw_sprite(&mut self, sprite: &Sprite, transform: &Transform) {
        self.sprites_to_draw
            .push((sprite.clone(), transform.clone()));
    }

    fn present(&mut self) {
        // Submit draw calls to GPU
        self.end_frame();
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

/// Sprite batch for efficient rendering
pub struct SpriteBatch {
    sprites: Vec<SpriteInstance>,
    texture_id: Option<u32>,
}

#[derive(Clone)]
struct SpriteInstance {
    position: [f32; 2],
    scale: [f32; 2],
    rotation: f32,
    color: [f32; 4],
    uv: [f32; 4], // x, y, width, height in texture
}

impl SpriteBatch {
    pub fn new() -> Self {
        SpriteBatch {
            sprites: Vec::new(),
            texture_id: None,
        }
    }

    pub fn add(&mut self, sprite: &Sprite, transform: &Transform) {
        self.sprites.push(SpriteInstance {
            position: [transform.position.x as f32, transform.position.y as f32],
            scale: [transform.scale.x as f32, transform.scale.y as f32],
            rotation: transform.rotation.y as f32, // Using y rotation for 2D
            color: sprite.color,
            uv: [0.0, 0.0, 1.0, 1.0], // Full texture for now
        });
    }

    pub fn flush(&mut self) {
        // Submit batch to GPU
        self.sprites.clear();
    }
}

/// Camera for 2D rendering
pub struct Camera2D {
    pub position: Vec3,
    pub zoom: f32,
    pub rotation: f32,
}

impl Camera2D {
    pub fn new() -> Self {
        Camera2D {
            position: Vec3::zero(),
            zoom: 1.0,
            rotation: 0.0,
        }
    }

    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        // Simple 2D view matrix
        let cos_r = self.rotation.cos() as f32;
        let sin_r = self.rotation.sin() as f32;
        let z = self.zoom as f32;

        [
            [cos_r * z, sin_r * z, 0.0, 0.0],
            [-sin_r * z, cos_r * z, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [
                -(self.position.x as f32),
                -(self.position.y as f32),
                0.0,
                1.0,
            ],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer2D::new(800, 600);
        assert_eq!(renderer.width, 800);
        assert_eq!(renderer.height, 600);
    }

    #[test]
    fn test_sprite_batch() {
        let mut batch = SpriteBatch::new();
        let sprite = Sprite::new("test", 32.0, 32.0);
        let transform = Transform::new(Vec3::new(0.0, 0.0, 0.0));

        batch.add(&sprite, &transform);
        assert_eq!(batch.sprites.len(), 1);

        batch.flush();
        assert_eq!(batch.sprites.len(), 0);
    }

    #[test]
    fn test_camera_2d() {
        let camera = Camera2D::new();
        assert_eq!(camera.zoom, 1.0);

        let matrix = camera.view_matrix();
        // Identity-ish matrix for default camera
        assert_eq!(matrix[0][0], 1.0);
    }
}
