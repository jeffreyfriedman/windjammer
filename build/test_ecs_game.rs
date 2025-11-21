use windjammer_game_framework::renderer::{Renderer, Color};
use windjammer_game_framework::input::{Input, Key, MouseButton};
use windjammer_game_framework::math::{Vec3, Mat4};
use windjammer_game_framework::ecs::*;
use windjammer_game_framework::game_app::GameApp;



struct TestGame {
    frame_count: i64,
    score: i64,
}

impl Default for TestGame {
    fn default() -> Self {
        TestGame {
            frame_count: 0,
            score: 0,
        }
    }
}

#[inline]
fn init(game: &mut TestGame) {
    println!("🎮 Test Game Initialized!");
    println!("ECS integration working!");
    game.frame_count = 0;
    game.score = 0;
}

#[inline]
fn update(game: &mut TestGame, delta: f64, input: &Input) {
    game.frame_count = game.frame_count + 1;
    game.score = game.score + 1;
    if game.frame_count % 60 == 0 {
        println!("Frame: {}, Score: {}, Delta: {}", game.frame_count, game.score, delta)
    }
}

#[inline]
fn render(game: &mut TestGame, renderer: &mut Renderer) {
    renderer.clear(Color::rgb(0.1, 0.1, 0.3))
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
            .with(TestGame::default())
            .build();
        
        Self { world, game_entity }
    }
    
    fn game_mut(&mut self) -> &mut TestGame {
        self.world.get_component_mut::<TestGame>(self.game_entity).unwrap()
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


