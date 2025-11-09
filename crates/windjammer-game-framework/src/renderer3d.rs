//! High-level 3D renderer for Windjammer games
//!
//! This module provides a simple, easy-to-use 3D renderer that abstracts
//! away the complexity of wgpu and provides a clean API for Windjammer games.
//!
//! **Philosophy**: Zero crate leakage - no wgpu, winit, or nalgebra types exposed.

use crate::math::{Mat4, Vec3, Vec4};
use crate::renderer::Color;
use crate::rendering::backend::Vertex3D;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// High-level 3D renderer
///
/// Provides simple methods for 3D rendering without exposing wgpu internals.
pub struct Renderer3D {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    clear_color: Color,
    
    // Camera
    view_matrix: Mat4,
    projection_matrix: Mat4,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    
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

        // Load shader (simple 3D shader for greybox games)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("3D Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rendering/shaders/simple_3d.wgsl").into()),
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

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3D Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D Render Pipeline"),
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
            pipeline,
            depth_texture,
            depth_view,
            clear_color: Color::black(),
            view_matrix,
            projection_matrix,
            camera_buffer,
            camera_bind_group,
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
        
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&camera_data));
    }

    /// Clear the screen with the given color
    pub fn clear(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Draw a cube at the given position
    pub fn draw_cube(&mut self, position: Vec3, size: Vec3, color: Color) {
        let half_size = size * 0.5;
        let color_array = color.to_array();
        let base_index = self.vertices.len() as u16;

        // Define 8 vertices of a cube
        let vertices = [
            // Front face
            [-half_size.x, -half_size.y,  half_size.z],
            [ half_size.x, -half_size.y,  half_size.z],
            [ half_size.x,  half_size.y,  half_size.z],
            [-half_size.x,  half_size.y,  half_size.z],
            // Back face
            [-half_size.x, -half_size.y, -half_size.z],
            [ half_size.x, -half_size.y, -half_size.z],
            [ half_size.x,  half_size.y, -half_size.z],
            [-half_size.x,  half_size.y, -half_size.z],
        ];

        // Add vertices with position offset
        for v in &vertices {
            self.vertices.push(Vertex3D {
                position: [
                    v[0] + position.x,
                    v[1] + position.y,
                    v[2] + position.z,
                ],
                normal: [0.0, 0.0, 1.0], // Simplified normal
                tex_coords: [0.0, 0.0],
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
                position: [position.x - half_size.x, position.y, position.z - half_size.z],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 0.0],
                color: color_array,
            },
            Vertex3D {
                position: [position.x + half_size.x, position.y, position.z - half_size.z],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [1.0, 0.0],
                color: color_array,
            },
            Vertex3D {
                position: [position.x + half_size.x, position.y, position.z + half_size.z],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [1.0, 1.0],
                color: color_array,
            },
            Vertex3D {
                position: [position.x - half_size.x, position.y, position.z + half_size.z],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0, 1.0],
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
    pub fn present(&mut self) {
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
                label: Some("3D Render Encoder"),
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
                    label: Some("3D Render Pass"),
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

                render_pass.set_pipeline(&self.pipeline);
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

    /// Resize the renderer (call when window is resized)
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            
            // Recreate depth texture
            self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        }
    }
}

