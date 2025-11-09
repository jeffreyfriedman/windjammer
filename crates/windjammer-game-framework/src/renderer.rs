//! High-level renderer for 2D games
//!
//! This module provides a simple, easy-to-use renderer that abstracts
//! away the complexity of wgpu and provides a clean API for Windjammer games.

use crate::math::Vec4;
use crate::rendering::backend::Vertex2D;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// Color representation (RGBA, 0.0-1.0)
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn black() -> Self {
        Self::rgb(0.0, 0.0, 0.0)
    }

    pub const fn white() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }

    pub const fn red() -> Self {
        Self::rgb(1.0, 0.0, 0.0)
    }

    pub const fn green() -> Self {
        Self::rgb(0.0, 1.0, 0.0)
    }

    pub const fn blue() -> Self {
        Self::rgb(0.0, 0.0, 1.0)
    }

    pub const fn yellow() -> Self {
        Self::rgb(1.0, 1.0, 0.0)
    }

    pub const fn cyan() -> Self {
        Self::rgb(0.0, 1.0, 1.0)
    }

    pub const fn magenta() -> Self {
        Self::rgb(1.0, 0.0, 1.0)
    }

    pub fn to_vec4(&self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// High-level 2D renderer
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    clear_color: Color,
    
    // Batching
    vertices: Vec<Vertex2D>,
    indices: Vec<u16>,
}

impl Renderer {
    /// Create a new renderer for the given window
    pub async fn new(window: &'static Window) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        // SAFETY: Window must live for 'static (guaranteed by game loop)
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
                    label: Some("Windjammer Device"),
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
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("rendering/shaders/sprite_2d.wgsl").into()),
        });

        // Create render pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Disable backface culling for 2D rendering
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

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            clear_color: Color::black(),
            vertices: Vec::new(),
            indices: Vec::new(),
        })
    }

    /// Clear the screen with the given color
    pub fn clear(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Draw a rectangle
    pub fn draw_rect(&mut self, x: f64, y: f64, width: f64, height: f64, color: Color) {
        // Convert f64 to f32 for GPU
        let x = x as f32;
        let y = y as f32;
        let width = width as f32;
        let height = height as f32;
        
        // Convert screen coordinates to NDC (-1 to 1)
        let screen_width = self.config.width as f32;
        let screen_height = self.config.height as f32;

        let x1 = (x / screen_width) * 2.0 - 1.0;
        let y1 = 1.0 - (y / screen_height) * 2.0;
        let x2 = ((x + width) / screen_width) * 2.0 - 1.0;
        let y2 = 1.0 - ((y + height) / screen_height) * 2.0;

        let color_array = color.to_array();
        let base_index = self.vertices.len() as u16;

        // Add vertices (two triangles for a rectangle)
        self.vertices.extend_from_slice(&[
            Vertex2D {
                position: [x1, y1],
                tex_coords: [0.0, 0.0],
                color: color_array,
            },
            Vertex2D {
                position: [x2, y1],
                tex_coords: [1.0, 0.0],
                color: color_array,
            },
            Vertex2D {
                position: [x2, y2],
                tex_coords: [1.0, 1.0],
                color: color_array,
            },
            Vertex2D {
                position: [x1, y2],
                tex_coords: [0.0, 1.0],
                color: color_array,
            },
        ]);

        // Add indices (two triangles)
        self.indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    /// Draw a circle (approximated with triangles)
    pub fn draw_circle(&mut self, x: f64, y: f64, radius: f64, color: Color) {
        // Convert f64 to f32 for GPU
        let x = x as f32;
        let y = y as f32;
        let radius = radius as f32;
        
        let segments = 32;
        let screen_width = self.config.width as f32;
        let screen_height = self.config.height as f32;

        // Convert center to NDC
        let cx = (x / screen_width) * 2.0 - 1.0;
        let cy = 1.0 - (y / screen_height) * 2.0;

        // Convert radius to NDC (use average of width/height for aspect ratio)
        let rx = (radius / screen_width) * 2.0;
        let ry = (radius / screen_height) * 2.0;

        let color_array = color.to_array();
        let base_index = self.vertices.len() as u16;

        // Center vertex
        self.vertices.push(Vertex2D {
            position: [cx, cy],
            tex_coords: [0.5, 0.5],
            color: color_array,
        });

        // Perimeter vertices
        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let px = cx + angle.cos() * rx;
            let py = cy + angle.sin() * ry;

            self.vertices.push(Vertex2D {
                position: [px, py],
                tex_coords: [0.5 + angle.cos() * 0.5, 0.5 + angle.sin() * 0.5],
                color: color_array,
            });

            if i > 0 {
                self.indices
                    .extend_from_slice(&[base_index, base_index + i, base_index + i + 1]);
            }
        }
    }

    /// Draw a progress bar (useful for health, ammo, etc.)
    ///
    /// # Arguments
    /// * `x`, `y` - Top-left corner position
    /// * `width`, `height` - Bar dimensions
    /// * `fill_ratio` - How full the bar is (0.0 to 1.0)
    /// * `fill_color` - Color of the filled portion
    /// * `bg_color` - Color of the background/empty portion
    pub fn draw_bar(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill_ratio: f64,
        fill_color: Color,
        bg_color: Color,
    ) {
        // Clamp fill_ratio to 0.0-1.0
        let fill_ratio = fill_ratio.max(0.0).min(1.0);
        
        // Draw background
        self.draw_rect(x, y, width, height, bg_color);
        
        // Draw filled portion
        if fill_ratio > 0.0 {
            let fill_width = width * fill_ratio;
            self.draw_rect(x, y, fill_width, height, fill_color);
        }
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
                label: Some("Render Encoder"),
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
                    label: Some("Render Pass"),
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

                render_pass.set_pipeline(&self.pipeline);
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
    /// 
    /// This is a Windjammer-friendly API that doesn't expose winit types.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    
    /// Internal: Resize from winit event (used by generated code)
    #[doc(hidden)]
    pub fn resize_from_winit(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.resize(new_size.width, new_size.height);
    }
}

