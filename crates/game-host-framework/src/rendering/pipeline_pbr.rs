//! PBR rendering pipeline with full material support

use super::backend::{GraphicsBackend, Vertex3D};
use crate::math::{Mat4, Vec3, Vec4};
use crate::pbr::{AlphaMode, Light, PBRMaterial, TextureHandle};

/// PBR rendering pipeline
pub struct PipelinePBR {
    render_pipeline: wgpu::RenderPipeline,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    light_bind_group_layout: wgpu::BindGroupLayout,
    material_bind_group_layout: wgpu::BindGroupLayout,
}

impl PipelinePBR {
    /// Create a new PBR pipeline
    pub fn new(backend: &GraphicsBackend, surface_format: wgpu::TextureFormat) -> Self {
        let device = &backend.device;

        // Camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("PBR Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Light bind group layout
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("PBR Light Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Material bind group layout (with 5 texture slots)
        let material_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("PBR Material Bind Group Layout"),
                entries: &[
                    // Material uniform (binding 0)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Texture sampler (binding 1)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Base color texture (binding 2)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Metallic-roughness texture (binding 3)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Normal texture (binding 4)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Occlusion texture (binding 5)
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Emissive texture (binding 6)
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        // Load PBR shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PBR Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/pbr.wgsl").into()),
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("PBR Pipeline Layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &light_bind_group_layout,
                &material_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PBR Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex3D::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
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
            camera_bind_group_layout,
            light_bind_group_layout,
            material_bind_group_layout,
        }
    }

    /// Get the render pipeline
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.render_pipeline
    }

    /// Get the camera bind group layout
    pub fn camera_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.camera_bind_group_layout
    }

    /// Get the light bind group layout
    pub fn light_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.light_bind_group_layout
    }

    /// Get the material bind group layout
    pub fn material_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.material_bind_group_layout
    }
}

/// Camera uniform data for PBR shader
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 3],
    pub _padding: f32,
}

unsafe impl bytemuck::Pod for CameraUniform {}
unsafe impl bytemuck::Zeroable for CameraUniform {}

impl CameraUniform {
    pub fn new(view_proj: Mat4, view_pos: Vec3) -> Self {
        Self {
            view_proj: view_proj.to_cols_array_2d(),
            view_pos: view_pos.to_array(),
            _padding: 0.0,
        }
    }
}

/// Light uniform data for PBR shader
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct LightUniform {
    pub position: [f32; 3],
    pub _padding1: f32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub light_type: u32, // 0 = directional, 1 = point, 2 = spot
    pub _padding2: [u32; 3],
    pub direction: [f32; 3],
    pub range: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub _padding3: [f32; 2],
}

unsafe impl bytemuck::Pod for LightUniform {}
unsafe impl bytemuck::Zeroable for LightUniform {}

impl LightUniform {
    pub fn from_light(light: &Light) -> Self {
        match light {
            Light::Directional(dir_light) => Self {
                position: [0.0, 0.0, 0.0],
                _padding1: 0.0,
                color: dir_light.color.to_array(),
                intensity: dir_light.intensity,
                light_type: 0,
                _padding2: [0, 0, 0],
                direction: dir_light.direction.to_array(),
                range: 0.0,
                inner_angle: 0.0,
                outer_angle: 0.0,
                _padding3: [0.0, 0.0],
            },
            Light::Point(point_light) => Self {
                position: point_light.position.to_array(),
                _padding1: 0.0,
                color: point_light.color.to_array(),
                intensity: point_light.intensity,
                light_type: 1,
                _padding2: [0, 0, 0],
                direction: [0.0, 0.0, 0.0],
                range: point_light.range,
                inner_angle: 0.0,
                outer_angle: 0.0,
                _padding3: [0.0, 0.0],
            },
            Light::Spot(spot_light) => Self {
                position: spot_light.position.to_array(),
                _padding1: 0.0,
                color: spot_light.color.to_array(),
                intensity: spot_light.intensity,
                light_type: 2,
                _padding2: [0, 0, 0],
                direction: spot_light.direction.to_array(),
                range: spot_light.range,
                inner_angle: spot_light.inner_angle,
                outer_angle: spot_light.outer_angle,
                _padding3: [0.0, 0.0],
            },
        }
    }
}

/// Material uniform data for PBR shader
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct MaterialUniform {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
    pub normal_strength: f32,
    pub occlusion_strength: f32,
    pub alpha_cutoff: f32,
    pub alpha_mode: u32,
    pub has_base_color_texture: u32,
    pub has_metallic_roughness_texture: u32,
    pub has_normal_texture: u32,
    pub has_occlusion_texture: u32,
    pub has_emissive_texture: u32,
    pub _padding: [u32; 2],
}

unsafe impl bytemuck::Pod for MaterialUniform {}
unsafe impl bytemuck::Zeroable for MaterialUniform {}

impl MaterialUniform {
    pub fn from_material(material: &PBRMaterial) -> Self {
        let alpha_mode = match material.alpha_mode {
            AlphaMode::Opaque => 0,
            AlphaMode::Mask => 1,
            AlphaMode::Blend => 2,
        };

        Self {
            base_color: material.base_color.to_array(),
            metallic: material.metallic,
            roughness: material.roughness,
            emissive: material.emissive.to_array(),
            emissive_strength: material.emissive_strength,
            normal_strength: material.normal_strength,
            occlusion_strength: material.occlusion_strength,
            alpha_cutoff: material.alpha_cutoff,
            alpha_mode,
            has_base_color_texture: material.base_color_texture.is_some() as u32,
            has_metallic_roughness_texture: material.metallic_roughness_texture.is_some() as u32,
            has_normal_texture: material.normal_texture.is_some() as u32,
            has_occlusion_texture: material.occlusion_texture.is_some() as u32,
            has_emissive_texture: material.emissive_texture.is_some() as u32,
            _padding: [0, 0],
        }
    }
}

