//! 2D Rendering Pipeline
//!
//! Design Philosophy:
//! - Simple, efficient sprite rendering
//! - Batching for performance
//! - Orthographic projection for 2D games
//! - Easy to understand and extend

use super::backend::{GraphicsBackend, Vertex2D};
use super::sprite::{Sprite, SpriteBatch};
use crate::math::Vec2;

/// 2D rendering pipeline for sprites
pub struct Pipeline2D {
    backend: GraphicsBackend,
    render_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
}

impl Pipeline2D {
    /// Create a new 2D rendering pipeline
    pub async fn new() -> Result<Self, String> {
        let backend = GraphicsBackend::new().await?;

        Ok(Self {
            backend,
            render_pipeline: None,
            vertex_buffer: None,
            index_buffer: None,
            bind_group: None,
        })
    }

    /// Initialize the rendering pipeline with shaders
    pub fn init(&mut self, width: u32, height: u32) -> Result<(), String> {
        // Create shader module
        let shader = self
            .backend
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("2D Sprite Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite_2d.wgsl").into()),
            });

        // Create pipeline layout
        let pipeline_layout =
            self.backend
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("2D Pipeline Layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        // Create render pipeline
        let render_pipeline =
            self.backend
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("2D Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex2D::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                });

        self.render_pipeline = Some(render_pipeline);

        Ok(())
    }

    /// Render a sprite batch
    pub fn render_batch(&mut self, batch: &SpriteBatch) -> Result<(), String> {
        // Get vertices and indices from batch
        let vertices = batch.vertices();
        let indices = batch.indices();

        if vertices.is_empty() {
            return Ok(()); // Nothing to render
        }

        // Create vertex buffer
        use wgpu::util::DeviceExt;
        let vertex_buffer =
            self.backend
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Sprite Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        // Create index buffer
        let index_buffer =
            self.backend
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Sprite Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);

        // Actual rendering would happen here with a render pass
        // For now, we've set up the buffers

        Ok(())
    }

    /// Clear the screen with a color
    pub fn clear(&self, _color: [f32; 4]) {
        // Would create a render pass and clear
        // Placeholder for now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_layout() {
        let desc = Vertex2D::desc();
        assert_eq!(desc.attributes.len(), 3);
        assert_eq!(desc.array_stride, std::mem::size_of::<Vertex2D>() as u64);
    }
}
