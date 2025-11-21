use windjammer_game_framework::renderer::{Renderer, Color};
use windjammer_game_framework::input::{Input, Key, MouseButton};
use windjammer_game_framework::math::{Vec3, Mat4};
use windjammer_game_framework::ecs::*;
use windjammer_game_framework::game_app::GameApp;



struct Platformer {
    score: i64,
}

impl Default for Platformer {
    fn default() -> Self {
        Platformer {
            score: 0,
        }
    }
}

#[inline]
fn init(game: &mut Platformer) {
    println!("🎮 2D Platformer Starting!");
    println!("Controls: Arrow keys to move, Space to jump");
    game.score = 0;
}

fn update(game: &mut Platformer, delta: f64, input: &Input) {
    let move_speed = 200.0;
    let jump_force = 400.0;
    if input.is_key_pressed(Key::Left) {
        println!("Moving left")
    }
    if input.is_key_pressed(Key::Right) {
        println!("Moving right")
    }
    if input.is_key_just_pressed(Key::Space) {
        println!("Jump!");
        game.score = game.score + 1;
    }
    if game.score > 0 && game.score % 10 == 0 {
        println!("Score: {}", game.score)
    }
}

#[inline]
fn render(game: &mut Platformer, renderer: &mut Renderer) {
    renderer.clear(Color::rgb(0.5, 0.7, 1.0));
    renderer.draw_rect(100.0, 300.0, 50.0, 50.0, Color::green());
    renderer.draw_rect(0.0, 550.0, 800.0, 50.0, Color::rgb(0.6, 0.4, 0.2));
    renderer.draw_rect(200.0, 450.0, 150.0, 20.0, Color::rgb(0.6, 0.4, 0.2));
    renderer.draw_rect(400.0, 350.0, 150.0, 20.0, Color::rgb(0.6, 0.4, 0.2));
    renderer.draw_rect(600.0, 250.0, 150.0, 20.0, Color::rgb(0.6, 0.4, 0.2))
}

// Generated: ECS world wrapper
struct GameWorld {
    world: windjammer_game_framework::ecs::World,
    game_entity: windjammer_game_framework::ecs::Entity,
}

impl GameWorld {
    fn new() -> Self {
        use windjammer_game_framework::ecs::*;
        let mut world = World::new();
        
        // Spawn game entity with game component
        let game_entity = world.spawn()
            .with(Platformer::default())
            .build();
        
        Self { world, game_entity }
    }
    
    fn game_mut(&mut self) -> &mut Platformer {
        self.world.get_component_mut::<Platformer>(self.game_entity).unwrap()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use windjammer_game_framework::*;
    use windjammer_game_framework::ecs::*;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    // Create event loop and window
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Windjammer Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)?;

    // Initialize ECS world
    let mut game_world = GameWorld::new();

    // Call init function
    init(game_world.game_mut());

    // Initialize renderer
    let window_ref: &'static winit::window::Window = unsafe { std::mem::transmute(&window) };
    let mut renderer = pollster::block_on(renderer::Renderer::new(window_ref))?;

    // Initialize input
    let mut input = input::Input::new();

    // Game loop
    let mut last_time = std::time::Instant::now();

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                WindowEvent::RedrawRequested => {
                    // Calculate delta time
                    let now = std::time::Instant::now();
                    let delta = (now - last_time).as_secs_f64();
                    last_time = now;

                    // Update game logic
                    update(game_world.game_mut(), delta, &input);

                    // Update ECS systems (scene graph, etc.)
                    SceneGraph::update_transforms(&mut game_world.world);

                    // Render
                    render(game_world.game_mut(), &mut renderer);
                    renderer.present();

                    // Clear input frame state
                    input.clear_frame_state();
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    })?;

    Ok(())
}


