// PONG - Complete playable game with rendering
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

struct Paddle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    dy: f32,
}

impl Paddle {
    fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            width: 20.0,
            height: 100.0,
            dy: 0.0,
        }
    }

    fn update(&mut self) {
        self.y += self.dy;
        if self.y < 0.0 {
            self.y = 0.0;
        }
        if self.y + self.height > WINDOW_HEIGHT as f32 {
            self.y = WINDOW_HEIGHT as f32 - self.height;
        }
    }

    fn to_vertices(&self) -> Vec<Vertex> {
        let x1 = (self.x / WINDOW_WIDTH as f32) * 2.0 - 1.0;
        let y1 = 1.0 - (self.y / WINDOW_HEIGHT as f32) * 2.0;
        let x2 = ((self.x + self.width) / WINDOW_WIDTH as f32) * 2.0 - 1.0;
        let y2 = 1.0 - ((self.y + self.height) / WINDOW_HEIGHT as f32) * 2.0;

        vec![
            Vertex {
                position: [x1, y1],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                position: [x2, y1],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                position: [x1, y2],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                position: [x2, y1],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                position: [x2, y2],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex {
                position: [x1, y2],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ]
    }
}

struct Ball {
    x: f32,
    y: f32,
    size: f32,
    dx: f32,
    dy: f32,
}

impl Ball {
    fn new() -> Self {
        Self {
            x: WINDOW_WIDTH as f32 / 2.0,
            y: WINDOW_HEIGHT as f32 / 2.0,
            size: 15.0,
            dx: 4.0,
            dy: 3.0,
        }
    }

    fn update(&mut self, left_paddle: &Paddle, right_paddle: &Paddle) -> Option<bool> {
        self.x += self.dx;
        self.y += self.dy;

        if self.y <= 0.0 || self.y + self.size >= WINDOW_HEIGHT as f32 {
            self.dy = -self.dy;
        }

        if self.x < left_paddle.x + left_paddle.width
            && self.x + self.size > left_paddle.x
            && self.y < left_paddle.y + left_paddle.height
            && self.y + self.size > left_paddle.y
        {
            self.dx = self.dx.abs();
        }

        if self.x < right_paddle.x + right_paddle.width
            && self.x + self.size > right_paddle.x
            && self.y < right_paddle.y + right_paddle.height
            && self.y + self.size > right_paddle.y
        {
            self.dx = -self.dx.abs();
        }

        if self.x < 0.0 {
            self.reset();
            return Some(false);
        } else if self.x > WINDOW_WIDTH as f32 {
            self.reset();
            return Some(true);
        }

        None
    }

    fn reset(&mut self) {
        self.x = WINDOW_WIDTH as f32 / 2.0;
        self.y = WINDOW_HEIGHT as f32 / 2.0;
        self.dx = -self.dx;
    }

    fn to_vertices(&self) -> Vec<Vertex> {
        let x1 = (self.x / WINDOW_WIDTH as f32) * 2.0 - 1.0;
        let y1 = 1.0 - (self.y / WINDOW_HEIGHT as f32) * 2.0;
        let x2 = ((self.x + self.size) / WINDOW_WIDTH as f32) * 2.0 - 1.0;
        let y2 = 1.0 - ((self.y + self.size) / WINDOW_HEIGHT as f32) * 2.0;

        vec![
            Vertex {
                position: [x1, y1],
                color: [1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [x2, y1],
                color: [1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [x1, y2],
                color: [1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [x2, y1],
                color: [1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [x2, y2],
                color: [1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [x1, y2],
                color: [1.0, 1.0, 0.0, 1.0],
            },
        ]
    }
}

fn main() {
    println!("🎮 PONG - Windjammer Game Framework");
    println!("====================================");
    println!("Controls:");
    println!("  Left Paddle:  W (up) / S (down)");
    println!("  Right Paddle: ↑ (up) / ↓ (down)");
    println!("  Quit: ESC");
    println!("");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("PONG - Windjammer Game")
        .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance.create_surface(&window).unwrap();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: None,
        },
        None,
    ))
    .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: WINDOW_WIDTH,
        height: WINDOW_HEIGHT,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // Create shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
    });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
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

    let mut left_paddle = Paddle::new(30.0, WINDOW_HEIGHT as f32 / 2.0 - 50.0);
    let mut right_paddle = Paddle::new(
        WINDOW_WIDTH as f32 - 50.0,
        WINDOW_HEIGHT as f32 / 2.0 - 50.0,
    );
    let mut ball = Ball::new();
    let mut left_score = 0;
    let mut right_score = 0;
    let mut last_update = Instant::now();

    println!("Game started! Score: 0 - 0");
    println!("Window should show white paddles and yellow ball!");

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("Final Score: {} - {}", left_score, right_score);
                    elwt.exit();
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    physical_key: PhysicalKey::Code(key_code),
                                    state,
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    let pressed = state.is_pressed();
                    match key_code {
                        KeyCode::KeyW => left_paddle.dy = if pressed { -5.0 } else { 0.0 },
                        KeyCode::KeyS => left_paddle.dy = if pressed { 5.0 } else { 0.0 },
                        KeyCode::ArrowUp => right_paddle.dy = if pressed { -5.0 } else { 0.0 },
                        KeyCode::ArrowDown => right_paddle.dy = if pressed { 5.0 } else { 0.0 },
                        KeyCode::Escape => {
                            println!("Final Score: {} - {}", left_score, right_score);
                            elwt.exit();
                        }
                        _ => {}
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(new_size),
                    ..
                } => {
                    if new_size.width > 0 && new_size.height > 0 {
                        config.width = new_size.width;
                        config.height = new_size.height;
                        surface.configure(&device, &config);
                    }
                }
                Event::AboutToWait => {
                    let now = Instant::now();
                    if now.duration_since(last_update) >= Duration::from_millis(16) {
                        left_paddle.update();
                        right_paddle.update();

                        if let Some(left_scored) = ball.update(&left_paddle, &right_paddle) {
                            if left_scored {
                                left_score += 1;
                                println!("Left scores! Score: {} - {}", left_score, right_score);
                            } else {
                                right_score += 1;
                                println!("Right scores! Score: {} - {}", left_score, right_score);
                            }
                        }

                        last_update = now;
                    }

                    // Render
                    let output = surface.get_current_texture().unwrap();
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                    // Collect all vertices
                    let mut vertices = Vec::new();
                    vertices.extend(left_paddle.to_vertices());
                    vertices.extend(right_paddle.to_vertices());
                    vertices.extend(ball.to_vertices());

                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                    {
                        let mut render_pass =
                            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                            r: 0.0,
                                            g: 0.0,
                                            b: 0.0,
                                            a: 1.0,
                                        }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });

                        render_pass.set_pipeline(&render_pipeline);
                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_pass.draw(0..vertices.len() as u32, 0..1);
                    }

                    queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }
                _ => {}
            }
        })
        .unwrap();
}
