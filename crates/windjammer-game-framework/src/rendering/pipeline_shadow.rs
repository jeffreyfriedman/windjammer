// Shadow Map Pipeline
// Renders depth from light's perspective for shadow mapping

/// Shadow map pipeline for rendering depth from light's perspective
pub struct ShadowMapPipeline {
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

/// Camera uniform for light-space transformation
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowCameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl ShadowMapPipeline {
    /// Create a new shadow map pipeline
    pub fn new(device: &wgpu::Device) -> Self {
        // Create bind group layout for light-space camera
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shadow Map Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Load shadow map shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shadow Map Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/shadow_map.wgsl").into()
            ),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shadow Map Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shadow Map Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    // Vertex buffer layout (position only for shadow mapping)
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[], // No color output, only depth
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2, // Constant bias to prevent shadow acne
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            render_pipeline,
            bind_group_layout,
        }
    }

    /// Create a bind group for a specific light-space camera
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Map Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        })
    }

    /// Render shadow map
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        shadow_texture_view: &wgpu::TextureView,
        bind_group: &wgpu::BindGroup,
        vertex_buffer: &wgpu::Buffer,
        index_buffer: &wgpu::Buffer,
        index_count: u32,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shadow Map Render Pass"),
            color_attachments: &[], // No color output
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: shadow_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..index_count, 0, 0..1);
    }

    /// Get bind group layout (for creating bind groups externally)
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}

/// Shadow map texture and view
pub struct ShadowMap {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub resolution: u32,
}

impl ShadowMap {
    /// Create a new shadow map texture
    pub fn new(device: &wgpu::Device, resolution: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map Texture"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create comparison sampler for shadow mapping
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Map Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // For PCF
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            resolution,
        }
    }
}

/// Cube shadow map for point lights (6 faces)
pub struct CubeShadowMap {
    pub texture: wgpu::Texture,
    pub views: [wgpu::TextureView; 6],
    pub sampler: wgpu::Sampler,
    pub resolution: u32,
}

impl CubeShadowMap {
    /// Create a new cube shadow map
    pub fn new(device: &wgpu::Device, resolution: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Cube Shadow Map Texture"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 6, // 6 faces
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Create views for each face
        let views = [
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cube Shadow Map Face +X"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: 0,
                array_layer_count: Some(1),
                ..Default::default()
            }),
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cube Shadow Map Face -X"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: 1,
                array_layer_count: Some(1),
                ..Default::default()
            }),
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cube Shadow Map Face +Y"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: 2,
                array_layer_count: Some(1),
                ..Default::default()
            }),
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cube Shadow Map Face -Y"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: 3,
                array_layer_count: Some(1),
                ..Default::default()
            }),
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cube Shadow Map Face +Z"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: 4,
                array_layer_count: Some(1),
                ..Default::default()
            }),
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Cube Shadow Map Face -Z"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: 5,
                array_layer_count: Some(1),
                ..Default::default()
            }),
        ];

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Cube Shadow Map Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        Self {
            texture,
            views,
            sampler,
            resolution,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_camera_uniform_size() {
        // Ensure uniform is properly aligned
        assert_eq!(
            std::mem::size_of::<ShadowCameraUniform>(),
            64 // 4x4 matrix = 16 floats = 64 bytes
        );
    }
}

