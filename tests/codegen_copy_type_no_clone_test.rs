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
