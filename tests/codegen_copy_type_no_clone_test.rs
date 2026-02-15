/// TDD Test: Generated Rust should NOT emit .clone() on Copy types
///
/// Bug: When a variable of a Copy type (f32, i32, bool) is used multiple times,
/// the auto-clone analysis marks it as needing .clone(). But Copy types are
/// implicitly copied — .clone() is unnecessary noise.
///
/// Discovered via dogfooding: The platformer game generates code like
///   `tile_width.clone()`, `self.player.y.clone()`, `self.tile_size.clone()`
/// where tile_width is i32, player.y is f32, and tile_size is f32.
///
/// Root Cause: The auto-clone analysis doesn't know variable types. The codegen
/// has a name-based heuristic (checking for "i", "j", "index" etc.) to skip
/// .clone() on Copy types, but it misses most real variable names.
///
/// Fix: Track variable types in the codegen and check is_copy_type() before
/// emitting .clone().

fn compile_to_rust(source: &str) -> String {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, source).unwrap();

    let output_dir = temp_dir.path().join("build");
    std::fs::create_dir_all(&output_dir).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        panic!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated = output_dir.join("test.rs");
    std::fs::read_to_string(&generated).unwrap_or_else(|_| {
        panic!(
            "No test.rs generated. stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn test_i32_variable_reused_no_clone() {
    // i32 is Copy — reusing it should NOT generate .clone()
    let code = compile_to_rust(
        r#"
struct Grid {
    width: i32,
    height: i32,
    cells: Vec<bool>,
}

impl Grid {
    fn new() -> Grid {
        let width = 10
        let height = 10
        let mut cells = Vec::new()
        for _i in 0..(width * height) {
            cells.push(false)
        }
        Grid { width, height, cells }
    }
}

fn main() {
    let grid = Grid::new()
    println("{}", grid.width)
}
"#,
    );

    // width and height are i32 — they should NEVER have .clone()
    assert!(
        !code.contains("width.clone()"),
        "i32 variable 'width' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("height.clone()"),
        "i32 variable 'height' should not need .clone(). Generated:\n{}",
        code
    );
}

#[test]
fn test_f32_field_access_no_clone() {
    // Accessing f32 fields through self should NOT generate .clone()
    let code = compile_to_rust(
        r#"
struct Player {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Player {
    fn new() -> Player {
        Player { x: 0.0, y: 0.0, width: 24.0, height: 32.0 }
    }
}

struct Game {
    player: Player,
    tile_size: f32,
}

impl Game {
    fn new() -> Game {
        Game {
            player: Player::new(),
            tile_size: 32.0,
        }
    }

    fn check_collision(self, next_x: f32, next_y: f32) -> bool {
        next_x > self.player.x &&
        next_y > self.player.y &&
        next_x < self.player.x + self.player.width &&
        next_y < self.player.y + self.player.height
    }

    fn get_tile_pos(self, x: f32) -> f32 {
        x / self.tile_size
    }
}

fn main() {
    let game = Game::new()
    println("{}", game.tile_size)
}
"#,
    );

    // f32 fields are Copy — should NEVER have .clone()
    assert!(
        !code.contains("self.player.y.clone()"),
        "f32 field 'self.player.y' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.player.x.clone()"),
        "f32 field 'self.player.x' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.player.width.clone()"),
        "f32 field 'self.player.width' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.player.height.clone()"),
        "f32 field 'self.player.height' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.tile_size.clone()"),
        "f32 field 'self.tile_size' should not need .clone(). Generated:\n{}",
        code
    );
}

#[test]
fn test_bool_variable_no_clone() {
    // bool is Copy — should NOT generate .clone()
    let code = compile_to_rust(
        r#"
fn process(active: bool, visible: bool) -> bool {
    if active {
        visible
    } else {
        false
    }
}

fn main() {
    let result = process(true, false)
    println("{}", result)
}
"#,
    );

    assert!(
        !code.contains("active.clone()"),
        "bool variable 'active' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("visible.clone()"),
        "bool variable 'visible' should not need .clone(). Generated:\n{}",
        code
    );
}

#[test]
fn test_copy_type_in_struct_literal_no_clone() {
    // When Copy-type locals are used in a struct literal after being used
    // in a for-loop range, they should NOT need .clone()
    let code = compile_to_rust(
        r#"
struct Level {
    width: i32,
    height: i32,
    tile_size: f32,
    tiles: Vec<bool>,
}

impl Level {
    fn new() -> Level {
        let width = 25
        let height = 15
        let tile_size = 32.0

        let mut tiles = Vec::new()
        for _i in 0..(width * height) {
            tiles.push(false)
        }

        Level { width, height, tile_size, tiles }
    }
}

fn main() {
    let level = Level::new()
    println("{} {} {}", level.width, level.height, level.tile_size)
}
"#,
    );

    assert!(
        !code.contains("width.clone()"),
        "i32 local 'width' should not need .clone() in struct literal. Generated:\n{}",
        code
    );
    assert!(
        !code.contains("height.clone()"),
        "i32 local 'height' should not need .clone() in struct literal. Generated:\n{}",
        code
    );
    assert!(
        !code.contains("tile_size.clone()"),
        "f32 local 'tile_size' should not need .clone() in struct literal. Generated:\n{}",
        code
    );
}

#[test]
fn test_self_f32_field_passed_to_method_no_clone() {
    // DOGFOODING BUG: In the platformer game, self.player.y (f32) is used
    // multiple times in a method body and passed as an argument to another
    // method. The auto-clone incorrectly adds .clone() because it sees
    // the same value used in multiple places, but f32 is Copy.
    //
    // Pattern from platformer.wj:
    //   let next_x = self.player.x + self.player.vx * delta
    //   if !self.rect_collides(next_x, self.player.y, self.player.width, self.player.height) {
    //       self.player.x = next_x
    //   }
    let code = compile_to_rust(
        r#"
struct Player {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    width: f32,
    height: f32,
}

impl Player {
    fn new() -> Player {
        Player { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, width: 24.0, height: 32.0 }
    }
}

struct Game {
    player: Player,
    tile_size: f32,
}

impl Game {
    fn new() -> Game {
        Game { player: Player::new(), tile_size: 32.0 }
    }

    fn rect_collides(self, x: f32, y: f32, w: f32, h: f32) -> bool {
        x > 0.0 && y > 0.0 && w > 0.0 && h > 0.0
    }

    fn update(self, delta: f32) {
        let next_x = self.player.x + self.player.vx * delta
        if !self.rect_collides(next_x, self.player.y, self.player.width, self.player.height) {
            self.player.x = next_x
        }
        let next_y = self.player.y + self.player.vy * delta
        if !self.rect_collides(self.player.x, next_y, self.player.width, self.player.height) {
            self.player.y = next_y
        }
    }
}

fn main() {
    let mut game = Game::new()
    game.update(0.016)
}
"#,
    );

    // f32 fields are Copy — should NEVER have .clone() even when used multiple times
    assert!(
        !code.contains("self.player.y.clone()"),
        "f32 field 'self.player.y' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.player.x.clone()"),
        "f32 field 'self.player.x' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.player.width.clone()"),
        "f32 field 'self.player.width' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.player.height.clone()"),
        "f32 field 'self.player.height' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.tile_size.clone()"),
        "f32 field 'self.tile_size' should not need .clone(). Generated:\n{}",
        code
    );
}

#[test]
fn test_self_f32_field_in_render_no_clone() {
    // DOGFOODING BUG: self.tile_size (f32) used twice in draw_rect call
    // gets .clone() added to the second usage. f32 is Copy, no clone needed.
    let code = compile_to_rust(
        r#"
struct Game {
    tile_size: f32,
    camera_x: f32,
    camera_y: f32,
}

impl Game {
    fn new() -> Game {
        Game { tile_size: 32.0, camera_x: 0.0, camera_y: 0.0 }
    }

    fn render(self) {
        let wx = 5.0 * self.tile_size - self.camera_x
        let wy = 3.0 * self.tile_size - self.camera_y
        println("{} {} {} {}", wx, wy, self.tile_size, self.tile_size)
    }
}

fn main() {
    let game = Game::new()
    game.render()
}
"#,
    );

    assert!(
        !code.contains("self.tile_size.clone()"),
        "f32 field 'self.tile_size' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.camera_x.clone()"),
        "f32 field 'self.camera_x' should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.camera_y.clone()"),
        "f32 field 'self.camera_y' should not need .clone(). Generated:\n{}",
        code
    );
}

/// TDD Test: `let index = expr as usize; arr[index]` should NOT generate `arr[index as usize]`
///
/// Bug: When a variable is already declared with `as usize`, the codegen adds
/// another `as usize` when it's used as an array index, producing:
///   `let index = (y * width + x) as usize;`
///   `self.tiles[index as usize] = solid;`
/// The double cast is redundant and noisy.
///
/// Discovered via dogfooding: platformer.wj has:
///   `let index = (y * self.tile_width + x) as usize`
///   `self.tiles_solid[index] = solid`
/// Generated Rust produces `self.tiles_solid[index as usize]` — double cast.
#[test]
fn test_no_double_as_usize_cast() {
    let code = compile_to_rust(
        r#"
struct Grid {
    width: i32,
    height: i32,
    cells: Vec<bool>,
}

impl Grid {
    fn set_cell(x: i32, y: i32, val: bool) {
        let index = (y * self.width + x) as usize
        self.cells[index] = val
    }

    fn get_cell(x: i32, y: i32) -> bool {
        let index = (y * self.width + x) as usize
        self.cells[index]
    }
}

fn main() {
    let g = Grid { width: 10, height: 10, cells: Vec::new() }
}
"#,
    );

    // The variable `index` is already `as usize` — using it as an array index
    // should NOT add another `as usize`.
    assert!(
        !code.contains("index as usize"),
        "Variable 'index' is already usize — should not be cast again. Generated:\n{}",
        code
    );
    // Verify the correct pattern exists: `index` used directly as array index
    assert!(
        code.contains("self.cells[index]"),
        "Array indexing should use 'index' directly (already usize). Generated:\n{}",
        code
    );
}

/// TDD Test: Method calls returning Copy types (bool) should NOT get .clone()
///
/// Bug: `input.is_key_down(Key::W).clone()` — is_key_down returns bool,
/// which is Copy. The .clone() is unnecessary.
///
/// Discovered via dogfooding: pong.wj and space_invaders.wj generate
/// `.clone()` on bool-returning method calls when the result is passed
/// as a function argument.
#[test]
fn test_method_call_returning_bool_no_clone() {
    let code = compile_to_rust(
        r#"
struct Input {
    keys: Vec<bool>,
}

impl Input {
    fn is_key_down(key: i32) -> bool {
        true
    }
}

struct Paddle {
    y: f32,
    speed: f32,
}

impl Paddle {
    fn update(delta: f32, up: bool, down: bool) {
        if up {
            self.y = self.y - self.speed * delta
        }
        if down {
            self.y = self.y + self.speed * delta
        }
    }
}

fn main() {
    let input = Input { keys: Vec::new() }
    let p = Paddle { y: 300.0, speed: 200.0 }
    p.update(0.016, input.is_key_down(0), input.is_key_down(1))
}
"#,
    );

    assert!(
        !code.contains("is_key_down(0).clone()") && !code.contains("is_key_down(1).clone()"),
        "Method returning bool should not need .clone(). Generated:\n{}",
        code
    );
}

/// TDD Test: Exact pong pattern — method call result on borrowed param passed to method on self field
///
/// Bug: `self.left_paddle.update(delta, input.is_key_down(Key::W).clone(), ...)`
/// The `.clone()` on `input.is_key_down(Key::W)` is unnecessary because:
/// 1. `is_key_down` returns `bool` (Copy type)  
/// 2. Even if `input` is `&Input`, the *return value* is owned
///
/// The key difference from the simpler test above: here `input` is a *parameter*
/// inferred as borrowed (`&Input`), and the method call result is passed as an
/// argument to a method call on `self.field`.
#[test]
fn test_borrowed_param_method_call_result_no_clone() {
    let code = compile_to_rust(
        r#"
struct Input {
    dummy: i32,
}

impl Input {
    fn is_key_down(key: i32) -> bool {
        true
    }
}

struct Paddle {
    y: f32,
    speed: f32,
}

impl Paddle {
    fn update(delta: f32, up: bool, down: bool) {
        if up {
            self.y = self.y - self.speed * delta
        }
        if down {
            self.y = self.y + self.speed * delta
        }
    }
}

struct Game {
    left_paddle: Paddle,
    right_paddle: Paddle,
}

impl Game {
    fn update(delta: f32, input: Input) {
        self.left_paddle.update(delta, input.is_key_down(0), input.is_key_down(1))
        self.right_paddle.update(delta, input.is_key_down(2), input.is_key_down(3))
    }
}

fn main() {
    let g = Game {
        left_paddle: Paddle { y: 0.0, speed: 200.0 },
        right_paddle: Paddle { y: 0.0, speed: 200.0 },
    }
}
"#,
    );

    assert!(
        !code.contains("is_key_down(0).clone()")
            && !code.contains("is_key_down(1).clone()")
            && !code.contains("is_key_down(2).clone()")
            && !code.contains("is_key_down(3).clone()"),
        "Method returning bool should not need .clone() even on borrowed param. Generated:\n{}",
        code
    );
}

/// TDD Test: Assignment targets must NEVER get .clone()
///
/// Bug: `emitter.lifetime = 1.0` generates `emitter.clone().lifetime = 1.0`
/// This is a SEMANTIC BUG — the mutation applies to the clone, not the original!
///
/// Discovered via dogfooding: particle_demo.wj creates a local `emitter` variable,
/// sets several fields, then the auto-clone analysis marks `emitter` for cloning
/// at later uses. But assignment targets are WRITES, not READS — they must never
/// be cloned.
#[test]
fn test_assignment_target_never_cloned() {
    let code = compile_to_rust(
        r#"
struct Emitter {
    x: f32,
    y: f32,
    r: f32,
    g: f32,
    b: f32,
    lifetime: f32,
    velocity_range: f32,
    particles: Vec<f32>,
}

impl Emitter {
    fn new(x: f32, y: f32) -> Emitter {
        Emitter { x: x, y: y, r: 1.0, g: 1.0, b: 1.0, lifetime: 1.0, velocity_range: 100.0, particles: Vec::new() }
    }

    fn burst(count: i32) {
        // stub
    }
}

fn main() {
    let mut emitter = Emitter::new(100.0, 200.0)
    emitter.r = 1.0
    emitter.g = 0.5
    emitter.b = 0.0
    emitter.velocity_range = 200.0
    emitter.lifetime = 1.0
    emitter.burst(50)
}
"#,
    );

    // Assignment targets must NEVER be cloned
    assert!(
        !code.contains("emitter.clone().lifetime"),
        "Assignment target must not be cloned — mutation would be lost! Generated:\n{}",
        code
    );
    assert!(
        !code.contains("emitter.clone().velocity_range"),
        "Assignment target must not be cloned — mutation would be lost! Generated:\n{}",
        code
    );
    assert!(
        !code.contains("emitter.clone().r"),
        "Assignment target must not be cloned — mutation would be lost! Generated:\n{}",
        code
    );
    // Verify correct pattern: direct field assignment
    assert!(
        code.contains("emitter.lifetime = 1.0"),
        "Should assign directly to emitter.lifetime. Generated:\n{}",
        code
    );
}

/// TDD Test: self.field used multiple times in method args should not clone if Copy
///
/// Bug: `self.button1.update_hover(self.mouse_x.clone(), self.mouse_y.clone())`
/// mouse_x and mouse_y are f32 — Copy types — no clone needed.
///
/// Discovered via dogfooding: ui_demo.wj has self.mouse_x used 4 times in
/// update_buttons(&self), so auto-clone analysis flags it. But f32 is Copy.
#[test]
fn test_self_copy_field_used_multiple_times_no_clone() {
    let code = compile_to_rust(
        r#"
struct Button {
    x: f32,
    y: f32,
}

impl Button {
    fn update_hover(mx: f32, my: f32) {
        // stub
    }
}

struct Game {
    mouse_x: f32,
    mouse_y: f32,
    btn1: Button,
    btn2: Button,
    btn3: Button,
}

impl Game {
    fn update_buttons() {
        self.btn1.update_hover(self.mouse_x, self.mouse_y)
        self.btn2.update_hover(self.mouse_x, self.mouse_y)
        self.btn3.update_hover(self.mouse_x, self.mouse_y)
    }
}

fn main() {
    let g = Game { mouse_x: 0.0, mouse_y: 0.0, btn1: Button { x: 0.0, y: 0.0 }, btn2: Button { x: 0.0, y: 0.0 }, btn3: Button { x: 0.0, y: 0.0 } }
}
"#,
    );

    assert!(
        !code.contains("self.mouse_x.clone()"),
        "f32 field self.mouse_x should not need .clone(). Generated:\n{}",
        code
    );
    assert!(
        !code.contains("self.mouse_y.clone()"),
        "f32 field self.mouse_y should not need .clone(). Generated:\n{}",
        code
    );
}

/// TDD Test: Local f32 variable from if-else expression used multiple times should NOT clone
///
/// Bug: `draw_rect(x - size.clone() / 2.0, y - size.clone() / 2.0, size.clone(), size.clone(), ...)`
/// `size` is assigned from an if-else expression but evaluates to f32.
///
/// Discovered via dogfooding: animation_demo.wj has `let size = if cond { 64.0 + ... } else { 64.0 }`
/// The type inference doesn't recognize the if-else as returning f32.
#[test]
fn test_local_copy_var_from_if_else_no_clone() {
    let code = compile_to_rust(
        r#"
extern fn draw_rect(x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32)

struct Character {
    x: f32,
    y: f32,
    anim: i32,
}

impl Character {
    fn render() {
        let size = if self.anim == 1 {
            64.0 + 8.0
        } else {
            64.0
        }
        draw_rect(self.x - size / 2.0, self.y - size / 2.0, size, size, 1.0, 0.0, 0.0, 1.0)
    }
}

fn main() {
    let c = Character { x: 0.0, y: 0.0, anim: 0 }
}
"#,
    );

    assert!(
        !code.contains("size.clone()"),
        "Local f32 variable 'size' from if-else should not need .clone(). Generated:\n{}",
        code
    );
}

/// TDD Test: Chained field access through self-borrowed iterator should not clone
/// when the final field is Copy
///
/// Bug: `enemy.velocity.clone().y * delta` in shooter_game.wj
/// `enemy` is from `for enemy in self.enemies` (borrowed from self).
/// `velocity` is a Vec2 (non-Copy), but `velocity.y` is f32 (Copy).
/// Rust auto-deref handles this: (&enemy).velocity.y works fine.
/// Cloning the intermediate struct is wasteful.
///
/// Discovered via dogfooding: shooter_game.wj lines 95, 100
#[test]
fn test_chained_field_access_copy_subfield_no_clone() {
    let code = compile_to_rust(
        r#"
struct Vec2 {
    x: f32,
    y: f32,
}

struct Bullet {
    position: Vec2,
    velocity: Vec2,
}

struct Game {
    bullets: Vec<Bullet>,
}

impl Game {
    fn update(delta: f32) {
        for bullet in self.bullets {
            bullet.position.y += bullet.velocity.y * delta
        }
    }
}

fn main() {
    let g = Game { bullets: Vec::<Bullet>::new() }
}
"#,
    );

    assert!(
        !code.contains("velocity.clone()"),
        "Chained field access should not clone intermediate struct when final field is Copy. Generated:\n{}",
        code
    );
}

/// TDD Test: Comparing .len() with integer literal should not cast to usize
///
/// Bug: `items.len() > (0 as usize)` — Rust infers 0 as usize already.
/// The cast is unnecessary noise.
///
/// Discovered via dogfooding: dialogue_demo.wj, particle_demo.wj, shooter_game.wj
#[test]
fn test_len_comparison_no_redundant_usize_cast() {
    let code = compile_to_rust(
        r#"
fn main() {
    let items: Vec<i32> = Vec::new()
    if items.len() > 0 {
        println!("not empty")
    }
}
"#,
    );

    assert!(
        !code.contains("(0 as usize)"),
        "Integer literal 0 in comparison with .len() should not need (0 as usize). Generated:\n{}",
        code
    );
    // Verify it still compares correctly (just `> 0` or `> 0_usize`)
    assert!(
        code.contains("> 0"),
        "Should still compare with 0. Generated:\n{}",
        code
    );
}

/// TDD Test: .reversed() should translate to .into_iter().rev() in Rust
///
/// Bug: `for idx in items.reversed()` generates `items.reversed()` which
/// is not valid Rust. Should generate `items.into_iter().rev()`.
///
/// Discovered via dogfooding: shooter_game.wj lines 132, 137
#[test]
fn test_reversed_translates_to_rev() {
    let code = compile_to_rust(
        r#"
fn main() {
    let items = [3, 1, 2]
    for item in items.reversed() {
        println!("{}", item)
    }
}
"#,
    );

    assert!(
        !code.contains(".reversed()"),
        ".reversed() should be translated to Rust equivalent. Generated:\n{}",
        code
    );
    assert!(
        code.contains(".rev()") || code.contains(".into_iter().rev()"),
        "Should contain .rev() or .into_iter().rev(). Generated:\n{}",
        code
    );
}
