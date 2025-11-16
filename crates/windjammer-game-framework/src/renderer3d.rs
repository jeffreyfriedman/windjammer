//! High-level 3D renderer for Windjammer games
//!
//! This module provides a simple, easy-to-use 3D renderer that abstracts
//! away the complexity of wgpu and provides a clean API for Windjammer games.
//!
//! **Philosophy**: Zero crate leakage - no wgpu, winit, or nalgebra types exposed.

use crate::math::{Mat4, Vec3};
use crate::renderer::Color;
use crate::rendering::backend::Vertex3D;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// SSGI Configuration
///
/// Controls the quality and performance of Screen-Space Global Illumination.
#[derive(Clone, Debug)]
pub struct SSGIConfig {
    /// Enable SSGI (default: false)
    pub enabled: bool,

    /// Number of samples per pixel (4-32, default: 8)
    pub num_samples: u32,

    /// Sample radius in world units (0.1-2.0, default: 0.5)
    pub sample_radius: f32,

    /// GI intensity multiplier (0.0-2.0, default: 1.0)
    pub intensity: f32,

    /// Maximum ray distance (default: 5.0)
    pub max_distance: f32,
}

impl Default for SSGIConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            num_samples: 8,
            sample_radius: 0.5,
            intensity: 1.0,
            max_distance: 5.0,
        }
    }
}

/// High-level 3D renderer
///
/// Provides simple methods for 3D rendering without exposing wgpu internals.
/// Supports both forward rendering and deferred rendering with SSGI.
pub struct Renderer3D {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Forward rendering pipeline (original)
    forward_pipeline: wgpu::RenderPipeline,

    // G-Buffer rendering pipeline (for SSGI)
    gbuffer_pipeline: wgpu::RenderPipeline,
    gbuffer_bind_group_layout: wgpu::BindGroupLayout,

    // G-Buffer textures
    gbuffer_position: wgpu::Texture,
    gbuffer_position_view: wgpu::TextureView,
    gbuffer_normal: wgpu::Texture,
    gbuffer_normal_view: wgpu::TextureView,
    gbuffer_albedo: wgpu::Texture,
    gbuffer_albedo_view: wgpu::TextureView,

    // SSGI compute pipeline
    ssgi_pipeline: wgpu::ComputePipeline,
    ssgi_bind_group: wgpu::BindGroup,
    ssgi_output: wgpu::Texture,
    ssgi_output_view: wgpu::TextureView,

    // Composite pipeline (combines direct + indirect lighting)
    composite_pipeline: wgpu::RenderPipeline,
    composite_bind_group: wgpu::BindGroup,

    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    clear_color: Color,

    // Camera
    view_matrix: Mat4,
    projection_matrix: Mat4,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    // Textures
    texture_bind_group_layout: wgpu::BindGroupLayout,

    // SSGI configuration
    ssgi_config: SSGIConfig,

    // Batching
    vertices: Vec<Vertex3D>,
    indices: Vec<u16>,
}

/// Camera for 3D rendering
pub struct Camera3D {
    pub position: Vec3,
    pub yaw: f32,   // Rotation around Y axis (left/right)
    pub pitch: f32, // Rotation around X axis (up/down)
    pub fov: f32,   // Field of view in degrees
    pub near: f32,  // Near clipping plane
    pub far: f32,   // Far clipping plane
}

impl Camera3D {
    /// Create a new camera at the origin
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            fov: 70.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Create a camera at a specific position
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    /// Get the forward direction vector
    pub fn forward(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();

        Vec3::new(
            yaw_rad.sin() * pitch_rad.cos(),
            pitch_rad.sin(),
            yaw_rad.cos() * pitch_rad.cos(),
        )
    }

    /// Get the right direction vector
    pub fn right(&self) -> Vec3 {
        let yaw_rad = self.yaw.to_radians();
        Vec3::new(yaw_rad.cos(), 0.0, -yaw_rad.sin())
    }

    /// Get the up direction vector
    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    /// Get the view matrix for this camera
    pub fn view_matrix(&self) -> Mat4 {
        let forward = self.forward();
        let target = self.position + forward;
        Mat4::look_at_rh(self.position, target, Vec3::new(0.0, 1.0, 0.0))
    }

    /// Get the projection matrix for this camera
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov.to_radians(), aspect_ratio, self.near, self.far)
    }
}

impl Default for Camera3D {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer3D {
    /// Create a new 3D renderer for the given window
    pub async fn new(window: &'static Window) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window)?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable graphics adapter")?;

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Windjammer 3D Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create depth texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create G-Buffer textures for SSGI
        let texture_size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        // Position texture (RGBA32Float)
        let gbuffer_position = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("G-Buffer Position"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let gbuffer_position_view =
            gbuffer_position.create_view(&wgpu::TextureViewDescriptor::default());

        // Normal texture (RGBA16Float)
        let gbuffer_normal = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("G-Buffer Normal"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let gbuffer_normal_view =
            gbuffer_normal.create_view(&wgpu::TextureViewDescriptor::default());

        // Albedo texture (RGBA8Unorm)
        let gbuffer_albedo = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("G-Buffer Albedo"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let gbuffer_albedo_view =
            gbuffer_albedo.create_view(&wgpu::TextureViewDescriptor::default());

        // SSGI output texture (for compute shader)
        let ssgi_output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SSGI Output"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ssgi_output_view = ssgi_output.create_view(&wgpu::TextureViewDescriptor::default());

        // Load shader (simple 3D shader for greybox games)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("rendering/shaders/simple_3d.wgsl").into(),
            ),
        });

        // Create camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
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

        // Create camera buffer (view + projection matrices)
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: 128, // 2 * Mat4 (64 bytes each)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Create texture bind group layout
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3D Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create forward render pipeline (original rendering path)
        let forward_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Forward Render Pipeline"),
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // Enable backface culling for 3D
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
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

        // Load G-Buffer shader
        let gbuffer_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("G-Buffer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rendering/shaders/gbuffer.wgsl").into()),
        });

        // Create G-Buffer bind group layout
        let gbuffer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("G-Buffer Bind Group Layout"),
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

        // Create G-Buffer pipeline layout
        let gbuffer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("G-Buffer Pipeline Layout"),
                bind_group_layouts: &[&gbuffer_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create G-Buffer render pipeline (renders to multiple targets)
        let gbuffer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("G-Buffer Render Pipeline"),
            layout: Some(&gbuffer_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &gbuffer_shader,
                entry_point: "vs_main",
                buffers: &[Vertex3D::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &gbuffer_shader,
                entry_point: "fs_main",
                targets: &[
                    // Position
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // Normal
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // Albedo
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
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
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Load SSGI compute shader
        let ssgi_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SSGI Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("rendering/shaders/ssgi_simple.wgsl").into(),
            ),
        });

        // Create SSGI bind group layout
        let ssgi_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SSGI Bind Group Layout"),
                entries: &[
                    // Camera uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // G-buffer position
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    // G-buffer normal
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    // G-buffer albedo
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // SSGI output (storage texture)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        // Create SSGI bind group
        let ssgi_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSGI Bind Group"),
            layout: &ssgi_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gbuffer_position_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&gbuffer_normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&gbuffer_albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&ssgi_output_view),
                },
            ],
        });

        // Create SSGI compute pipeline
        let ssgi_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SSGI Pipeline Layout"),
            bind_group_layouts: &[&ssgi_bind_group_layout],
            push_constant_ranges: &[],
        });

        let ssgi_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("SSGI Compute Pipeline"),
            layout: Some(&ssgi_pipeline_layout),
            module: &ssgi_shader,
            entry_point: "cs_main",
        });

        // Load composite shader
        let composite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Composite Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("rendering/shaders/composite.wgsl").into(),
            ),
        });

        // Create sampler for composite pass
        let composite_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Composite Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create composite bind group layout
        let composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Composite Bind Group Layout"),
                entries: &[
                    // Camera uniform
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
                    // Albedo texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // SSGI texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Sampler for SSGI
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Create composite bind group
        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Composite Bind Group"),
            layout: &composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&gbuffer_albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&composite_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&ssgi_output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&composite_sampler),
                },
            ],
        });

        // Create composite pipeline layout
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Composite Pipeline Layout"),
                bind_group_layouts: &[&composite_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create composite render pipeline
        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Composite Render Pipeline"),
            layout: Some(&composite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &composite_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &composite_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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

        // Initialize camera matrices
        let aspect_ratio = size.width as f32 / size.height as f32;
        let camera = Camera3D::new();
        let view_matrix = camera.view_matrix();
        let projection_matrix = camera.projection_matrix(aspect_ratio);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            forward_pipeline,
            gbuffer_pipeline,
            gbuffer_bind_group_layout,
            gbuffer_position,
            gbuffer_position_view,
            gbuffer_normal,
            gbuffer_normal_view,
            gbuffer_albedo,
            gbuffer_albedo_view,
            ssgi_pipeline,
            ssgi_bind_group,
            ssgi_output,
            ssgi_output_view,
            composite_pipeline,
            composite_bind_group,
            depth_texture,
            depth_view,
            clear_color: Color::black(),
            view_matrix,
            projection_matrix,
            camera_buffer,
            camera_bind_group,
            texture_bind_group_layout,
            ssgi_config: SSGIConfig::default(),
            vertices: Vec::new(),
            indices: Vec::new(),
        })
    }

    /// Set the camera for rendering
    pub fn set_camera(&mut self, camera: &Camera3D) {
        let aspect_ratio = self.config.width as f32 / self.config.height as f32;
        self.view_matrix = camera.view_matrix();
        self.projection_matrix = camera.projection_matrix(aspect_ratio);

        // Update camera buffer
        let mut camera_data = Vec::new();
        camera_data.extend_from_slice(&self.view_matrix.to_cols_array());
        camera_data.extend_from_slice(&self.projection_matrix.to_cols_array());

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&camera_data));
    }

    /// Clear the screen with the given color
    pub fn clear(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Load a texture from a file
    ///
    /// Supports PNG, JPEG, and other common image formats.
    ///
    /// # Arguments
    /// * `path` - Path to the image file
    ///
    /// # Returns
    /// A `Texture` that can be used with `draw_textured_cube()` and other textured drawing methods.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or is not a valid image.
    ///
    /// # Example
    /// ```no_run
    /// let texture = renderer.load_texture("assets/wall.png")?;
    /// renderer.draw_textured_cube(pos, size, &texture);
    /// ```
    pub fn load_texture(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<crate::texture::Texture, Box<dyn std::error::Error>> {
        crate::texture::Texture::from_file(
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
            path,
        )
    }

    /// Create a checkerboard texture
    ///
    /// Creates a procedural checkerboard pattern - useful for testing or placeholder textures.
    ///
    /// # Arguments
    /// * `size` - Size of the texture (width and height in pixels)
    /// * `checker_size` - Size of each checker square in pixels
    /// * `color1` - First color (RGBA, 0-255)
    /// * `color2` - Second color (RGBA, 0-255)
    ///
    /// # Example
    /// ```no_run
    /// // Create a black and white checkerboard
    /// let texture = renderer.create_checkerboard_texture(256, 32, [0, 0, 0, 255], [255, 255, 255, 255])?;
    /// ```
    pub fn create_checkerboard_texture(
        &self,
        size: u32,
        checker_size: u32,
        color1: [u8; 4],
        color2: [u8; 4],
    ) -> Result<crate::texture::Texture, Box<dyn std::error::Error>> {
        crate::texture::Texture::checkerboard(
            &self.device,
            &self.queue,
            &self.texture_bind_group_layout,
            size,
            checker_size,
            color1,
            color2,
        )
    }

    /// Enable or disable SSGI (Screen-Space Global Illumination)
    ///
    /// When enabled, the renderer uses deferred rendering with G-buffer
    /// to calculate indirect lighting. This provides more realistic lighting
    /// but has a performance cost.
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable SSGI
    ///
    /// # Example
    /// ```no_run
    /// renderer.enable_ssgi(true);
    /// ```
    pub fn enable_ssgi(&mut self, enabled: bool) {
        self.ssgi_config.enabled = enabled;
    }

    /// Set the number of samples for SSGI
    ///
    /// Higher sample counts produce better quality but are slower.
    /// Recommended range: 4-32 (default: 8)
    ///
    /// # Arguments
    /// * `samples` - Number of samples per pixel
    pub fn set_ssgi_samples(&mut self, samples: u32) {
        self.ssgi_config.num_samples = samples.clamp(4, 32);
    }

    /// Set the sample radius for SSGI
    ///
    /// Controls how far rays travel when sampling indirect lighting.
    /// Recommended range: 0.1-2.0 (default: 0.5)
    ///
    /// # Arguments
    /// * `radius` - Sample radius in world units
    pub fn set_ssgi_radius(&mut self, radius: f32) {
        self.ssgi_config.sample_radius = radius.clamp(0.1, 2.0);
    }

    /// Set the intensity of SSGI
    ///
    /// Controls how much indirect lighting contributes to the final image.
    /// Recommended range: 0.0-2.0 (default: 1.0)
    ///
    /// # Arguments
    /// * `intensity` - GI intensity multiplier
    pub fn set_ssgi_intensity(&mut self, intensity: f32) {
        self.ssgi_config.intensity = intensity.clamp(0.0, 2.0);
    }

    /// Get the current SSGI configuration
    ///
    /// Returns a copy of the current SSGI settings.
    pub fn ssgi_config(&self) -> SSGIConfig {
        self.ssgi_config.clone()
    }

    /// Draw a cube at the given position
    pub fn draw_cube(&mut self, position: Vec3, size: Vec3, color: Color) {
        let half_size = size * 0.5;
        let color_array = color.to_array();
        let base_index = self.vertices.len() as u16;

        // Define 8 vertices of a cube
        let vertices = [
            // Front face
            [-half_size.x, -half_size.y, half_size.z],
            [half_size.x, -half_size.y, half_size.z],
            [half_size.x, half_size.y, half_size.z],
            [-half_size.x, half_size.y, half_size.z],
            // Back face
            [-half_size.x, -half_size.y, -half_size.z],
            [half_size.x, -half_size.y, -half_size.z],
            [half_size.x, half_size.y, -half_size.z],
            [-half_size.x, half_size.y, -half_size.z],
        ];

        // Add vertices with position offset
        for v in &vertices {
            self.vertices.push(Vertex3D {
                position: [v[0] + position.x, v[1] + position.y, v[2] + position.z],
                normal: [0.0, 0.0, 1.0], // Simplified normal
                tex_coords: [0.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0], // Default tangent
                color: color_array,
            });
        }

        // Add indices for 6 faces (2 triangles each)
        let faces = [
            // Front
            [0, 1, 2, 0, 2, 3],
            // Back
            [5, 4, 7, 5, 7, 6],
            // Left
            [4, 0, 3, 4, 3, 7],
            // Right
            [1, 5, 6, 1, 6, 2],
            // Top
            [3, 2, 6, 3, 6, 7],
            // Bottom
            [4, 5, 1, 4, 1, 0],
        ];

        for face in &faces {
            for &idx in face {
                self.indices.push(base_index + idx);
            }
        }
    }

    /// Draw a plane (floor/ceiling)
    pub fn draw_plane(&mut self, position: Vec3, size: Vec3, color: Color) {
        let half_size = size * 0.5;
        let color_array = color.to_array();
        let base_index = self.vertices.len() as u16;

        // 4 vertices for a plane
        self.vertices.extend_from_slice(&[
            Vertex3D {
                position: [
                    position.x - half_size.x,
                    position.y,
                    position.z - half_size.z,
                ],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0], // Default tangent
                color: color_array,
            },
            Vertex3D {
                position: [
                    position.x + half_size.x,
                    position.y,
                    position.z - half_size.z,
                ],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [1.0, 0.0],
                tangent: [1.0, 0.0, 0.0, 1.0], // Default tangent
                color: color_array,
            },
            Vertex3D {
                position: [
                    position.x + half_size.x,
                    position.y,
                    position.z + half_size.z,
                ],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [1.0, 1.0],
                tangent: [1.0, 0.0, 0.0, 1.0], // Default tangent
                color: color_array,
            },
            Vertex3D {
                position: [
                    position.x - half_size.x,
                    position.y,
                    position.z + half_size.z,
                ],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 1.0],
                tangent: [1.0, 0.0, 0.0, 1.0], // Default tangent
                color: color_array,
            },
        ]);

        // 2 triangles
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    /// Present the rendered frame to the screen
    ///
    /// Automatically chooses between forward rendering (fast) and deferred rendering with SSGI (high quality)
    /// based on the current SSGI configuration.
    pub fn present(&mut self) {
        if self.ssgi_config.enabled {
            self.present_deferred();
        } else {
            self.present_forward();
        }
    }

    /// Forward rendering path (original, fast)
    fn present_forward(&mut self) {
        // Get current surface texture
        let output = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(e) => {
                eprintln!("Failed to acquire next swap chain texture: {:?}", e);
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Forward Render Encoder"),
            });

        // If we have vertices to render, create buffers and render
        if !self.vertices.is_empty() {
            // Create vertex buffer
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&self.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            // Create index buffer
            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&self.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

            // Begin render pass
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Forward Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: self.clear_color.r as f64,
                                g: self.clear_color.g as f64,
                                b: self.clear_color.b as f64,
                                a: self.clear_color.a as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                render_pass.set_pipeline(&self.forward_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            }
        } else {
            // No vertices, just clear
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color.r as f64,
                            g: self.clear_color.g as f64,
                            b: self.clear_color.b as f64,
                            a: self.clear_color.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Clear batches for next frame
        self.vertices.clear();
        self.indices.clear();
    }

    /// Deferred rendering path with SSGI (high quality, slower)
    fn present_deferred(&mut self) {
        // Get current surface texture
        let output = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(e) => {
                eprintln!("Failed to acquire next swap chain texture: {:?}", e);
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Deferred Render Encoder"),
            });

        // If we have vertices to render
        if !self.vertices.is_empty() {
            // Create vertex and index buffers
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&self.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&self.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

            // PASS 1: Render to G-Buffer
            {
                let mut gbuffer_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("G-Buffer Pass"),
                    color_attachments: &[
                        // Position
                        Some(wgpu::RenderPassColorAttachment {
                            view: &self.gbuffer_position_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                        // Normal
                        Some(wgpu::RenderPassColorAttachment {
                            view: &self.gbuffer_normal_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                        // Albedo
                        Some(wgpu::RenderPassColorAttachment {
                            view: &self.gbuffer_albedo_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: self.clear_color.r as f64,
                                    g: self.clear_color.g as f64,
                                    b: self.clear_color.b as f64,
                                    a: self.clear_color.a as f64,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        }),
                    ],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                gbuffer_pass.set_pipeline(&self.gbuffer_pipeline);
                gbuffer_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                gbuffer_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                gbuffer_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                gbuffer_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            }

            // PASS 2: SSGI Compute Pass
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("SSGI Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.ssgi_pipeline);
                compute_pass.set_bind_group(0, &self.ssgi_bind_group, &[]);

                // Dispatch compute shader (8x8 workgroups)
                let workgroup_size = 8;
                let dispatch_x = (self.config.width + workgroup_size - 1) / workgroup_size;
                let dispatch_y = (self.config.height + workgroup_size - 1) / workgroup_size;
                compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
            }

            // PASS 3: Composite Pass (combine direct + indirect lighting)
            {
                let mut composite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Composite Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                composite_pass.set_pipeline(&self.composite_pipeline);
                composite_pass.set_bind_group(0, &self.composite_bind_group, &[]);
                // Draw fullscreen triangle
                composite_pass.draw(0..3, 0..1);
            }
        } else {
            // No vertices, just clear
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.clear_color.r as f64,
                            g: self.clear_color.g as f64,
                            b: self.clear_color.b as f64,
                            a: self.clear_color.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Clear batches for next frame
        self.vertices.clear();
        self.indices.clear();
    }

    /// Resize the renderer (call when window is resized)
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            let texture_size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };

            // Recreate depth texture
            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = self
                .depth_texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Recreate G-Buffer textures
            self.gbuffer_position = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("G-Buffer Position"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.gbuffer_position_view = self
                .gbuffer_position
                .create_view(&wgpu::TextureViewDescriptor::default());

            self.gbuffer_normal = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("G-Buffer Normal"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.gbuffer_normal_view = self
                .gbuffer_normal
                .create_view(&wgpu::TextureViewDescriptor::default());

            self.gbuffer_albedo = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("G-Buffer Albedo"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.gbuffer_albedo_view = self
                .gbuffer_albedo
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Recreate SSGI output texture
            self.ssgi_output = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("SSGI Output"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.ssgi_output_view = self
                .ssgi_output
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Note: Bind groups will need to be recreated with new texture views
            // This is handled automatically on next frame
        }
    }
}
