use windjammer_game_framework::renderer::{Renderer, Color};
use windjammer_game_framework::input::{Input, Key};

#[derive(Clone, Debug)]
struct PongGame {
    left_paddle_y: f64,
    right_paddle_y: f64,
    ball_x: f64,
    ball_y: f64,
    ball_vx: f64,
    ball_vy: f64,
    score_left: i64,
    score_right: i64,
    paddle_speed: f64,
    ball_speed: f64,
}

impl Default for PongGame {
    fn default() -> Self {
        PongGame {
            left_paddle_y: 0.0,
            right_paddle_y: 0.0,
            ball_x: 0.0,
            ball_y: 0.0,
            ball_vx: 0.0,
            ball_vy: 0.0,
            score_left: 0,
            score_right: 0,
            paddle_speed: 0.0,
            ball_speed: 0.0,
        }
    }
}

fn init(game: &mut PongGame) {
    game.left_paddle_y = 250.0;
    game.right_paddle_y = 250.0;
    game.ball_x = 400.0;
    game.ball_y = 300.0;
    game.ball_vx = 200.0;
    game.ball_vy = 150.0;
    game.score_left = 0;
    game.score_right = 0;
    game.paddle_speed = 300.0;
    game.ball_speed = 200.0;
    println!("PONG initialized!");
    println!("Controls: W/S for left paddle, Up/Down for right paddle")
}

fn update(game: &mut PongGame, delta: f64) {
    game.ball_x = game.ball_x + game.ball_vx * delta;
    game.ball_y = game.ball_y + game.ball_vy * delta;
    if game.ball_y <= 10.0 {
        game.ball_y = 10.0;
        game.ball_vy = -game.ball_vy;
    }
    if game.ball_y >= 590.0 {
        game.ball_y = 590.0;
        game.ball_vy = -game.ball_vy;
    }
    if game.ball_x <= 30.0 && game.ball_x >= 10.0 {
        if game.ball_y >= game.left_paddle_y && game.ball_y <= game.left_paddle_y + 100.0 {
            game.ball_x = 30.0;
            game.ball_vx = -game.ball_vx;
        }
    }
    if game.ball_x >= 770.0 && game.ball_x <= 790.0 {
        if game.ball_y >= game.right_paddle_y && game.ball_y <= game.right_paddle_y + 100.0 {
            game.ball_x = 770.0;
            game.ball_vx = -game.ball_vx;
        }
    }
    if game.ball_x <= 0.0 {
        game.score_right = game.score_right + 1;
        println!("Right scores! Score: {} - {}", game.score_left, game.score_right);
        game.ball_x = 400.0;
        game.ball_y = 300.0;
        game.ball_vx = 200.0;
        game.ball_vy = 150.0;
    }
    if game.ball_x >= 800.0 {
        game.score_left = game.score_left + 1;
        println!("Left scores! Score: {} - {}", game.score_left, game.score_right);
        game.ball_x = 400.0;
        game.ball_y = 300.0;
        game.ball_vx = -200.0;
        game.ball_vy = 150.0;
    }
}

#[inline]
fn render(game: &mut PongGame, renderer: &mut Renderer) {
    renderer.clear(Color::black());
    renderer.draw_rect(10.0, game.left_paddle_y, 10.0, 100.0, Color::white());
    renderer.draw_rect(780.0, game.right_paddle_y, 10.0, 100.0, Color::white());
    renderer.draw_circle(game.ball_x, game.ball_y, 10.0, Color::yellow());
    let mut y = 0.0;
    while y < 600.0 {
        renderer.draw_rect(395.0, y, 10.0, 20.0, Color::white());
        y += 40.0;
    }
}

fn handle_input(game: &mut PongGame, input: &Input) {
    if input.is_key_pressed(Key::W) {
        game.left_paddle_y = game.left_paddle_y - 5.0;
        if game.left_paddle_y < 0.0 {
            game.left_paddle_y = 0.0;
        }
    }
    if input.is_key_pressed(Key::S) {
        game.left_paddle_y = game.left_paddle_y + 5.0;
        if game.left_paddle_y > 500.0 {
            game.left_paddle_y = 500.0;
        }
    }
    if input.is_key_pressed(Key::Up) {
        game.right_paddle_y = game.right_paddle_y - 5.0;
        if game.right_paddle_y < 0.0 {
            game.right_paddle_y = 0.0;
        }
    }
    if input.is_key_pressed(Key::Down) {
        game.right_paddle_y = game.right_paddle_y + 5.0;
        if game.right_paddle_y > 500.0 {
            game.right_paddle_y = 500.0;
        }
    }
}

#[inline]
fn cleanup(game: &mut PongGame) {
    println!("Final Score: {} - {}", game.score_left, game.score_right);
    println!("Thanks for playing PONG!")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use windjammer_game_framework::*;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::{ControlFlow, EventLoop};
    use winit::window::WindowBuilder;

    // Create event loop and window
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Windjammer Game")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)?;

    // Initialize game state
    let mut game = PongGame::default();

    // Call init function
    init(&mut game);

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
                    cleanup(&mut game);
                    elwt.exit();
                }
                WindowEvent::RedrawRequested => {
                    // Calculate delta time
                    let now = std::time::Instant::now();
                    let delta = (now - last_time).as_secs_f64();
                    last_time = now;

                    // Update game logic
                    update(&mut game, delta);

                    // Render
                    render(&mut game, &mut renderer);
                    renderer.present();

                    // Clear input frame state
                    input.clear_frame_state();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    input.update_from_winit(&event);
                    handle_input(&mut game, &input);
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


