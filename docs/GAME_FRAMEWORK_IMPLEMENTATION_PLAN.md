# 🎮 Game Framework Implementation Plan

**Goal:** Create a fully working game framework that follows Windjammer philosophy

**Status:** In Progress  
**Priority:** HIGH (validates language for real-world use)

---

## 🎯 **Success Criteria**

A user can write this code:

```windjammer
use game::prelude::*

@game
struct PongGame {
    ball: Ball,
    score: int,
}

@init
fn init(game: PongGame) {
    game.ball = Ball::new(0.0, 0.0)
}

@update
fn update(game: PongGame, delta: float) {
    game.ball.x += 1.0
}

@render
fn render(game: PongGame, renderer: Renderer) {
    renderer.draw_rect(game.ball.bounds(), Color::white())
}

fn main() {
    run_game("PONG", 800, 600)
}
```

And when they run `wj build pong.wj && cd build && cargo run`:
- ✅ Window opens (800x600)
- ✅ White rectangle renders
- ✅ Rectangle moves across screen
- ✅ 60 FPS smooth
- ✅ ESC closes window

**NO Rust syntax, NO crate exposure, JUST WORKS.**

---

## 📋 **Implementation Phases**

### **Phase 1: Decorator Parsing** ✅ (Already Done)

We already have:
- ✅ `Decorator` struct in AST
- ✅ Parser support for `@decorator_name`
- ✅ Decorator arguments parsing

Need to add:
- [ ] Validate game-specific decorators (`@game`, `@init`, etc.)
- [ ] Ensure only one `@game` struct
- [ ] Ensure only one of each function decorator

### **Phase 2: Ownership Inference for Decorators**

**Current State:**
- We have `OwnershipHint` enum with `Inferred` option
- Auto-clone system exists for variables

**Need to Add:**
- [ ] Analyze decorator function bodies
- [ ] Determine if parameters are modified → `&mut`
- [ ] Determine if parameters are only read → `&`
- [ ] Generate correct Rust signatures

**Algorithm:**
```
For each @update/@render/@input function:
  1. Find all uses of each parameter
  2. If parameter.field is assigned → needs &mut
  3. If parameter.method() is called → check method signature
  4. If only parameter.field is read → needs &
  5. Generate Rust signature with inferred ownership
```

**Example:**
```windjammer
@update
fn update(game: PongGame, delta: float) {
    game.score += 1  // Modifying game.score
}
```

**Compiler Analysis:**
- `game.score += 1` is an assignment
- `game` needs to be mutable
- But not taking ownership
- → Generate: `fn update(game: &mut PongGame, delta: f64)`

### **Phase 3: Game Loop Codegen**

**Input:** Windjammer file with decorators

**Output:** Rust code with game loop

**Steps:**
1. Find `@game` struct → this is the game state
2. Find `@init` function → call once at startup
3. Find `@update` function → call every frame
4. Find `@render` function → call every frame
5. Find `@input` function → call when input changes
6. Generate `main()` that wires everything together

**Generated Code Template:**
```rust
// User's structs and functions (with inferred ownership)
struct PongGame { ... }
fn init(game: &mut PongGame) { ... }  // Inferred &mut
fn update(game: &mut PongGame, delta: f64) { ... }  // Inferred &mut
fn render(game: &PongGame, renderer: &mut Renderer) { ... }  // Inferred &, &mut

// Generated game loop
fn main() {
    use windjammer_game_framework::prelude::*;
    
    let mut game = PongGame::default();
    init(&mut game);
    
    let config = WindowConfig {
        title: "PONG".to_string(),
        width: 800,
        height: 600,
    };
    
    let mut window = Window::new(config).unwrap();
    let mut renderer = Renderer::new(&window);
    let mut input = Input::new();
    let mut time = Time::new();
    
    window.run(move |event| {
        match event {
            Event::Update => {
                let delta = time.delta();
                update(&mut game, delta);
            }
            Event::Render => {
                renderer.begin_frame();
                render(&game, &mut renderer);
                renderer.end_frame();
            }
            Event::Input(input_event) => {
                input.process(input_event);
                if let Some(input_fn) = input_handler {
                    input_fn(&mut game, &input);
                }
            }
            Event::Quit => return false,
        }
        true
    });
}
```

### **Phase 4: Connect to windjammer-game-framework**

**Current State:**
- `windjammer-game-framework` exists with Rust API
- Has `Window`, `Renderer`, `Input` types
- Has wgpu backend

**Need to Do:**
- [ ] Ensure `Window`, `Renderer`, `Input` are in public API
- [ ] Ensure they work with generated code
- [ ] Test that window opens
- [ ] Test that rendering works
- [ ] Test that input works

**Bridge Code:**
```rust
// In windjammer-game-framework/src/lib.rs
pub mod prelude {
    pub use crate::window::{Window, WindowConfig, Event};
    pub use crate::rendering::Renderer;
    pub use crate::input::Input;
    pub use crate::time::Time;
    pub use crate::math::{Vec2, Color};
}
```

### **Phase 5: Test with Real PONG**

**Create:** `examples/games/pong/main.wj`

**Content:**
```windjammer
use game::prelude::*

@game
struct PongGame {
    ball_x: float,
    ball_y: float,
    ball_dx: float,
    ball_dy: float,
}

@init
fn init(game: PongGame) {
    game.ball_x = 0.0
    game.ball_y = 0.0
    game.ball_dx = 0.01
    game.ball_dy = 0.008
}

@update
fn update(game: PongGame, delta: float) {
    game.ball_x += game.ball_dx
    game.ball_y += game.ball_dy
    
    if game.ball_y > 1.0 || game.ball_y < -1.0 {
        game.ball_dy = -game.ball_dy
    }
}

@render
fn render(game: PongGame, renderer: Renderer) {
    renderer.clear(Color::black())
    renderer.draw_rect(game.ball_x, game.ball_y, 0.04, 0.04, Color::yellow())
}

fn main() {
    run_game("PONG", 800, 600)
}
```

**Test:**
```bash
wj build examples/games/pong/main.wj --output build
cd build && cargo run
```

**Expected:**
- Window opens
- Yellow square bounces around
- Smooth 60 FPS
- ESC closes

---

## 🔧 **Technical Details**

### **Decorator Validation**

In `src/analyzer.rs` or new `src/game_analyzer.rs`:

```rust
fn validate_game_decorators(program: &Program) -> Result<(), Error> {
    let mut game_struct = None;
    let mut init_fn = None;
    let mut update_fn = None;
    let mut render_fn = None;
    
    for item in &program.items {
        match item {
            Item::Struct { decl, .. } => {
                if has_decorator(&decl.decorators, "game") {
                    if game_struct.is_some() {
                        return Err("Multiple @game structs found");
                    }
                    game_struct = Some(decl);
                }
            }
            Item::Function { decl, .. } => {
                if has_decorator(&decl.decorators, "init") {
                    if init_fn.is_some() {
                        return Err("Multiple @init functions found");
                    }
                    init_fn = Some(decl);
                }
                // ... similar for @update, @render, @input
            }
        }
    }
    
    if game_struct.is_none() {
        return Err("No @game struct found");
    }
    
    Ok(())
}
```

### **Ownership Inference**

In `src/ownership_analyzer.rs`:

```rust
fn infer_decorator_function_ownership(func: &FunctionDecl) -> Vec<OwnershipMode> {
    let mut ownership = vec![];
    
    for param in &func.parameters {
        let mode = if is_modified_in_body(&param.name, &func.body) {
            OwnershipMode::MutBorrowed
        } else if is_read_in_body(&param.name, &func.body) {
            OwnershipMode::Borrowed
        } else {
            OwnershipMode::Owned
        };
        ownership.push(mode);
    }
    
    ownership
}

fn is_modified_in_body(param_name: &str, body: &[Statement]) -> bool {
    for stmt in body {
        match stmt {
            Statement::Assignment { target, .. } => {
                if target_references_param(target, param_name) {
                    return true;
                }
            }
            // ... check other statement types
        }
    }
    false
}
```

### **Codegen**

In `src/codegen/rust/generator.rs`:

```rust
fn generate_game_main(&mut self, program: &Program) -> String {
    let game_struct = find_game_struct(program);
    let init_fn = find_decorator_fn(program, "init");
    let update_fn = find_decorator_fn(program, "update");
    let render_fn = find_decorator_fn(program, "render");
    
    format!(r#"
fn main() {{
    use windjammer_game_framework::prelude::*;
    
    let mut game = {}::default();
    {}(&mut game);
    
    let config = WindowConfig {{
        title: "Game".to_string(),
        width: 800,
        height: 600,
    }};
    
    let mut window = Window::new(config).unwrap();
    let mut renderer = Renderer::new(&window);
    let mut time = Time::new();
    
    window.run(move |event| {{
        match event {{
            Event::Update => {{
                let delta = time.delta();
                {}(&mut game, delta);
            }}
            Event::Render => {{
                renderer.begin_frame();
                {}(&game, &mut renderer);
                renderer.end_frame();
            }}
            Event::Quit => return false,
        }}
        true
    }});
}}
"#, game_struct.name, init_fn.name, update_fn.name, render_fn.name)
}
```

---

## ⏱️ **Time Estimates**

| Phase | Estimated Time | Complexity |
|-------|---------------|------------|
| Phase 1: Decorator Validation | 1-2 hours | Medium |
| Phase 2: Ownership Inference | 2-3 hours | High |
| Phase 3: Game Loop Codegen | 2-3 hours | High |
| Phase 4: Framework Connection | 1-2 hours | Medium |
| Phase 5: Test & Debug | 2-4 hours | High |
| **Total** | **8-14 hours** | **High** |

---

## 🎯 **Milestones**

1. ✅ **M1:** Decorator validation works
2. ✅ **M2:** Ownership inference works
3. ✅ **M3:** Game loop generates
4. ✅ **M4:** Compiles to Rust
5. ✅ **M5:** Window opens
6. ✅ **M6:** Renders something
7. ✅ **M7:** Full PONG works

---

## 🚧 **Current Blockers**

1. Need to implement ownership analysis for decorator functions
2. Need to generate game loop boilerplate
3. Need to ensure `windjammer-game-framework` public API is correct
4. Need to test end-to-end

---

## 📝 **Notes**

- This is a **substantial** piece of work
- It validates Windjammer for real-world use cases
- It proves the decorator system works
- It proves ownership inference works
- It proves we can hide Rust complexity

**This is worth doing right, even if it takes time.**

