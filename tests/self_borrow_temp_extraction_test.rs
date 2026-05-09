//! TDD: Compiler should automatically extract temporaries when both
//! the method receiver and arguments borrow `self`, preventing E0499.
//!
//! Pattern: `self.field.method(self.other_method())`
//! The compiler must generate:
//!   let __tmp = self.other_method();
//!   self.field.method(__tmp);
//! instead of:
//!   self.field.method(self.other_method());

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_self_field_method_with_self_method_arg() {
    let source = r#"
struct SfxPlayer {
    volume: f32,
    last_variant: i32,
}

impl SfxPlayer {
    fn play(self, seed: i32) {
        self.last_variant = seed
    }
}

struct SoundManager {
    sfx: SfxPlayer,
    counter: i32,
}

impl SoundManager {
    fn next_rng(self) -> i32 {
        self.counter = self.counter + 1
        self.counter
    }

    fn play_sound(self) {
        self.sfx.play(self.next_rng())
    }
}
"#;
    let code = test_utils::compile_single(source);

    // The generated Rust must compile without E0499
    let result = test_utils::verify_rust_compiles(&code);
    assert!(
        result.is_ok(),
        "Generated Rust should compile without E0499 double-borrow error.\n\
         Generated code:\n{}\n\nRustc error:\n{}",
        code,
        result.unwrap_err()
    );
}

#[test]
fn test_self_field_method_with_multiple_self_args() {
    let source = r#"
struct Buffer {
    data: Vec<f32>,
}

impl Buffer {
    fn write(self, index: i32, value: f32) {
        self.data[0] = value
    }
}

struct Processor {
    buf: Buffer,
    position: i32,
    scale: f32,
}

impl Processor {
    fn current_pos(self) -> i32 {
        self.position
    }

    fn current_scale(self) -> f32 {
        self.scale
    }

    fn process(self) {
        self.buf.write(self.current_pos(), self.current_scale())
    }
}
"#;
    let code = test_utils::compile_single(source);

    let result = test_utils::verify_rust_compiles(&code);
    assert!(
        result.is_ok(),
        "Generated Rust should compile with multiple self-referencing args.\n\
         Generated code:\n{}\n\nRustc error:\n{}",
        code,
        result.unwrap_err()
    );
}

#[test]
fn test_self_field_update_with_self_method() {
    let source = r#"
struct Timer {
    elapsed: f32,
}

impl Timer {
    fn tick(self, dt: f32) {
        self.elapsed = self.elapsed + dt
    }
}

struct Game {
    timer: Timer,
    speed: f32,
}

impl Game {
    fn get_speed(self) -> f32 {
        self.speed
    }

    fn update(self, dt: f32) {
        self.timer.tick(dt * self.get_speed())
    }
}
"#;
    let code = test_utils::compile_single(source);

    let result = test_utils::verify_rust_compiles(&code);
    assert!(
        result.is_ok(),
        "Generated Rust should compile when self.method() used in expression arg.\n\
         Generated code:\n{}\n\nRustc error:\n{}",
        code,
        result.unwrap_err()
    );
}
