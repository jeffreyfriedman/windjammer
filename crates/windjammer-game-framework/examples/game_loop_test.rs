//! Test complete game loop integration
//! Combines rendering, physics, input, and game loop timing

use rapier2d::prelude::*;
use std::time::Instant;
use windjammer_game_framework::input::{Input, KeyCode};
use windjammer_game_framework::math::Vec2;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::window::WindowBuilder;

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

struct GameState {
    // Physics
    gravity: Vector<f32>,
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,

    // Game objects
    ball_handle: RigidBodyHandle,

    // Input
    input: Input,

    // Stats
    update_count: u64,
    render_count: u64,
}

impl GameState {
    fn new() -> Self {
        // Create physics world
        let gravity = vector![0.0, -9.81];
        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        // Create ground
        let ground_body = RigidBodyBuilder::fixed()
            .translation(vector![0.0, -0.8])
            .build();
        let ground_handle = rigid_body_set.insert(ground_body);
        let ground_collider = ColliderBuilder::cuboid(2.0, 0.1).build();
        collider_set.insert_with_parent(ground_collider, ground_handle, &mut rigid_body_set);

        // Create ball
        let ball_body = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.5])
            .build();
        let ball_handle = rigid_body_set.insert(ball_body);
        let ball_collider = ColliderBuilder::ball(0.1).restitution(0.7).build();
        collider_set.insert_with_parent(ball_collider, ball_handle, &mut rigid_body_set);

        Self {
            gravity,
            rigid_body_set,
            collider_set,
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            ball_handle,
            input: Input::new(),
            update_count: 0,
            render_count: 0,
        }
    }

    fn update(&mut self, _delta: f32) {
        self.update_count += 1;

        // Handle input - reset ball position on space
        if self.input.is_key_pressed(KeyCode::Space) {
            if let Some(ball) = self.rigid_body_set.get_mut(self.ball_handle) {
                ball.set_translation(vector![0.0, 0.5], true);
                ball.set_linvel(vector![0.0, 0.0], true);
            }
        }

        // Step physics
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    fn get_ball_position(&self) -> Vec2 {
        if let Some(ball) = self.rigid_body_set.get(self.ball_handle) {
            let pos = ball.translation();
            Vec2::new(pos.x, pos.y)
        } else {
            Vec2::new(0.0, 0.0)
        }
    }
}

fn main() {
    env_logger::init();

    println!("=== Windjammer Game Loop Test ===\n");
    println!("Controls:");
    println!("  SPACE - Reset ball position");
    println!("  ESC   - Exit\n");

    // Create event loop and window
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .with_title("Windjammer Game Loop")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("Failed to create window");

    // Initialize wgpu (abbreviated for brevity - same as sprite_test)
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance
        .create_surface(&window)
        .expect("Failed to create surface");

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .expect("Failed to find adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    ))
    .expect("Failed to create device");

    let size = window.inner_size();
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
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &config);

    // Create shader and pipeline
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Sprite Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Sprite Pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
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

    println!("✅ Rendering pipeline created");

    // Create game state
    let mut game_state = GameState::new();

    println!("✅ Game state initialized");
    println!("\n🎮 Game loop running...\n");

    let start_time = Instant::now();

    // Run event loop
    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("\n👋 Window closed.");
                    println!("📊 Stats:");
                    println!("   Updates: {}", game_state.update_count);
                    println!("   Renders: {}", game_state.render_count);
                    println!("   Runtime: {:.2}s", start_time.elapsed().as_secs_f32());
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
                    // Convert winit keycode to our KeyCode
                    let our_key = match key_code {
                        WinitKeyCode::Space => Some(KeyCode::Space),
                        WinitKeyCode::Escape => {
                            elwt.exit();
                            None
                        }
                        _ => None,
                    };

                    if let Some(key) = our_key {
                        match state {
                            ElementState::Pressed => game_state.input.press_key(key),
                            ElementState::Released => game_state.input.release_key(key),
                        }
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
                    // Update game state
                    let delta = 1.0 / 60.0;
                    game_state.update(delta);

                    // Get ball position for rendering
                    let ball_pos = game_state.get_ball_position();

                    // Create vertices for ball sprite
                    let size = 0.1;
                    let vertices = vec![
                        Vertex {
                            position: [ball_pos.x - size, ball_pos.y - size],
                            color: [1.0, 0.0, 0.0, 1.0],
                        },
                        Vertex {
                            position: [ball_pos.x + size, ball_pos.y - size],
                            color: [1.0, 0.0, 0.0, 1.0],
                        },
                        Vertex {
                            position: [ball_pos.x + size, ball_pos.y + size],
                            color: [1.0, 0.0, 0.0, 1.0],
                        },
                        Vertex {
                            position: [ball_pos.x - size, ball_pos.y - size],
                            color: [1.0, 0.0, 0.0, 1.0],
                        },
                        Vertex {
                            position: [ball_pos.x + size, ball_pos.y + size],
                            color: [1.0, 0.0, 0.0, 1.0],
                        },
                        Vertex {
                            position: [ball_pos.x - size, ball_pos.y + size],
                            color: [1.0, 0.0, 0.0, 1.0],
                        },
                    ];

                    use wgpu::util::DeviceExt;
                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                    // Render frame
                    let output = surface
                        .get_current_texture()
                        .expect("Failed to get texture");
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
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
                                            r: 0.1,
                                            g: 0.1,
                                            b: 0.1,
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

                    game_state.render_count += 1;

                    // Print stats every 60 frames
                    if game_state.render_count % 60 == 0 {
                        println!(
                            "Frame {} | Ball: ({:.2}, {:.2}) | Updates: {}",
                            game_state.render_count,
                            ball_pos.x,
                            ball_pos.y,
                            game_state.update_count
                        );
                    }
                }
                _ => {}
            }
        })
        .expect("Event loop error");
}
