# 🔧 Compiler ECS Integration

## Goal: Generate ECS-Based Game Code

When a Windjammer developer writes:

```windjammer
use std::game::*

@game
struct MyGame {
    score: int,
}

@init
fn init(game: MyGame) {
    println!("Game starting!")
}

@update
fn update(game: MyGame, delta: float) {
    game.score += 1
}

@render
fn render(game: MyGame, renderer: Renderer) {
    renderer.clear(Color::black())
}
```

The compiler should generate:

```rust
use windjammer_game_framework::prelude::*;
use windjammer_game_framework::ecs::*;

// User's game struct becomes a component
#[derive(Debug, Clone)]
struct MyGame {
    score: i32,
}

// Generated: ECS world wrapper
struct GameWorld {
    world: World,
    game_entity: Entity,
}

impl GameWorld {
    fn new() -> Self {
        let mut world = World::new();
        
        // Spawn game entity with MyGame component
        let game_entity = world.spawn()
            .with(MyGame { score: 0 })
            .build();
        
        Self { world, game_entity }
    }
    
    fn game_mut(&mut self) -> &mut MyGame {
        self.world.get_component_mut::<MyGame>(self.game_entity).unwrap()
    }
}

// User's init function
fn init(game: &mut MyGame) {
    println!("Game starting!");
}

// User's update function  
fn update(game: &mut MyGame, delta: f32) {
    game.score += 1;
}

// User's render function
fn render(game: &MyGame, renderer: &mut Renderer) {
    renderer.clear(Color::black());
}

// Generated main function with ECS integration
fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    
    // Call init
    init(game_world.game_mut());
    
    // Initialize renderer
    let window_ref: &'static winit::window::Window = unsafe { std::mem::transmute(&window) };
    let mut renderer = pollster::block_on(Renderer::new(window_ref))?;
    
    // Initialize input
    let mut input = Input::new();
    
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
                    let delta = (now - last_time).as_secs_f32();
                    last_time = now;
                    
                    // Update game logic
                    update(game_world.game_mut(), delta);
                    
                    // Update ECS systems (transform propagation, etc.)
                    SceneGraph::update_transforms(&mut game_world.world);
                    
                    // Render
                    render(game_world.game_mut(), &mut renderer);
                    
                    renderer.present();
                    input.clear_frame_state();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    input.update_from_winit(&event);
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
```

---

## Implementation Plan

### Phase 1: Basic ECS Integration ✅ (Current)

**Status**: In Progress

**What to Generate**:
1. Wrap user's `@game` struct as a component
2. Create `GameWorld` wrapper with ECS `World`
3. Spawn game entity with game component
4. Call user's `@init`, `@update`, `@render` functions
5. Integrate scene graph updates

**Files to Modify**:
- `src/codegen/rust/generator.rs::generate_game_main()`
- Add ECS imports to generated code
- Wrap game struct access through ECS

### Phase 2: Component Detection

**What to Generate**:
- Detect user-defined components (structs with `@component` or used in ECS)
- Generate component registration
- Generate queries for user systems

**Example**:
```windjammer
@component
struct Position {
    x: float,
    y: float,
}

@system
fn move_system(query: Query<(Position, Velocity)>, delta: float) {
    for (pos, vel) in query {
        pos.x += vel.x * delta
    }
}
```

Generates:
```rust
#[derive(Debug, Clone)]
struct Position {
    x: f32,
    y: f32,
}

fn move_system(world: &mut World, delta: f32) {
    let storage_pos = world.get_storage::<Position>();
    let storage_vel = world.get_storage::<Velocity>();
    
    if let (Some(pos_storage), Some(vel_storage)) = (storage_pos, storage_vel) {
        for (entity, pos) in pos_storage.iter_mut() {
            if let Some(vel) = vel_storage.get(entity) {
                pos.x += vel.x * delta;
            }
        }
    }
}
```

### Phase 3: System Scheduling

**What to Generate**:
- Detect `@system` decorated functions
- Add systems to scheduler
- Run systems in game loop

### Phase 4: Advanced Features

- Entity spawning from Windjammer code
- Component queries with filters
- System dependencies
- Parallel system execution

---

## Current Status

✅ ECS Core implemented
✅ Scene graph implemented
🔄 Compiler integration (in progress)
⏳ Component detection
⏳ System scheduling
⏳ Advanced features

---

## Next Steps

1. Update `generate_game_main()` to use ECS
2. Test with simple game example
3. Add component detection
4. Add system scheduling
5. Create comprehensive test games

**Let's make it happen!** 🚀

