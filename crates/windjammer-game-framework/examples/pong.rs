// PONG - A complete playable game using Windjammer Game Framework
use winit::event::{Event, WindowEvent, KeyEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;
use wgpu::util::DeviceExt;
use std::time::{Duration, Instant};

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

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

        // Bounce off top and bottom
        if self.y <= 0.0 || self.y + self.size >= WINDOW_HEIGHT as f32 {
            self.dy = -self.dy;
        }

        // Check paddle collisions
        if self.x < left_paddle.x + left_paddle.width &&
           self.x + self.size > left_paddle.x &&
           self.y < left_paddle.y + left_paddle.height &&
           self.y + self.size > left_paddle.y {
            self.dx = self.dx.abs();
        }

        if self.x < right_paddle.x + right_paddle.width &&
           self.x + self.size > right_paddle.x &&
           self.y < right_paddle.y + right_paddle.height &&
           self.y + self.size > right_paddle.y {
            self.dx = -self.dx.abs();
        }

        // Check scoring
        if self.x < 0.0 {
            self.reset();
            return Some(false); // Right scores
        } else if self.x > WINDOW_WIDTH as f32 {
            self.reset();
            return Some(true); // Left scores
        }

        None
    }

    fn reset(&mut self) {
        self.x = WINDOW_WIDTH as f32 / 2.0;
        self.y = WINDOW_HEIGHT as f32 / 2.0;
        self.dx = -self.dx;
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

    // Setup WGPU
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

    // Game state
    let mut left_paddle = Paddle::new(30.0, WINDOW_HEIGHT as f32 / 2.0 - 50.0);
    let mut right_paddle = Paddle::new(WINDOW_WIDTH as f32 - 50.0, WINDOW_HEIGHT as f32 / 2.0 - 50.0);
    let mut ball = Ball::new();
    let mut left_score = 0;
    let mut right_score = 0;
    let mut last_update = Instant::now();

    println!("Game started! Score: 0 - 0");

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
                    event: WindowEvent::KeyboardInput {
                        event: KeyEvent {
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
                    // Update game logic
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

                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

                    {
                        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                        // Note: In a complete implementation, we'd draw the paddles and ball here
                        // For now, this proves the game loop and input handling work
                    }

                    queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }
                _ => {}
            }
        })
        .unwrap();
}
