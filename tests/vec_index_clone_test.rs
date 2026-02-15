//! TDD Test: Vec indexing of non-Copy types needs .clone()
//!
//! Bug: When passing `vec[i]` to a function that takes ownership of a non-Copy type,
//! the codegen generates `func(vec[i as usize])` which tries to move out of the Vec.
//! Rust doesn't allow moving out of an index.
//!
//! Expected: `func(vec[i as usize].clone())`
//! Actual:   `func(vec[i as usize])`

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_dir(prefix: &str) -> std::path::PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::path::PathBuf::from(format!(
        "/tmp/wj-test-vec-idx-{}-{}-{}",
        prefix, pid, id
    ))
}

fn compile_wj_to_rust(wj_source: &str, test_name: &str) -> (String, bool) {
    let input_dir = unique_dir(test_name);
    let output_dir = unique_dir(&format!("{}-out", test_name));
    std::fs::create_dir_all(&input_dir).unwrap();

    let wj_file = input_dir.join("test.wj");
    std::fs::write(&wj_file, wj_source).unwrap();

    let wj_binary = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/wj");

    let _output = Command::new(&wj_binary)
        .args(["build", wj_file.to_str().unwrap(), "--output", output_dir.to_str().unwrap()])
        .output()
        .expect("Failed to run wj compiler");

    let rs_file = output_dir.join("test.rs");
    let rust_code = std::fs::read_to_string(&rs_file).unwrap_or_default();

    let compiles = if !rust_code.is_empty() {
        let bin_output = output_dir.join("test_bin");
        let rustc_output = Command::new("rustc")
            .args(["--edition", "2021", "--crate-type", "bin", rs_file.to_str().unwrap(), "-o", bin_output.to_str().unwrap()])
            .output()
            .expect("Failed to run rustc");
        if !rustc_output.status.success() {
            eprintln!("rustc stderr: {}", String::from_utf8_lossy(&rustc_output.stderr));
        }
        rustc_output.status.success()
    } else {
        false
    };

    let _ = std::fs::remove_dir_all(&input_dir);
    let _ = std::fs::remove_dir_all(&output_dir);

    (rust_code, compiles)
}

#[test]
fn test_vec_index_non_copy_passed_to_function() {
    let source = r#"
enum GameEvent {
    PlayerMove(f32, f32),
    ItemPickup(string),
    None,
}

fn describe_event(event: GameEvent) -> string {
    match event {
        GameEvent::PlayerMove(x, y) => "moved",
        GameEvent::ItemPickup(item) => "pickup",
        GameEvent::None => "none",
    }
}

fn main() {
    let events: Vec<GameEvent> = vec![
        GameEvent::PlayerMove(1.0, 2.0),
        GameEvent::ItemPickup("Sword"),
        GameEvent::None,
    ]
    let mut i = 0
    while i < 3 {
        let desc = describe_event(events[i])
        println("${desc}")
        i = i + 1
    }
}
"#;

    let (rust_code, compiles) = compile_wj_to_rust(source, "vec-idx-noncopy");

    println!("Generated Rust:\n{}", rust_code);

    // The generated code should add .clone() when indexing into Vec<NonCopy>
    // and passing to a function that takes ownership
    assert!(
        rust_code.contains(".clone()"),
        "Expected .clone() for non-Copy type indexed from Vec.\nGenerated:\n{}",
        rust_code
    );

    assert!(compiles, "Generated Rust should compile successfully");
}

#[test]
fn test_vec_index_copy_type_no_clone() {
    let source = r#"
fn sum_vec(nums: Vec<i32>) -> i32 {
    let mut total = 0
    let mut i = 0
    while i < 3 {
        total = total + nums[i]
        i = i + 1
    }
    total
}

fn main() {
    let nums: Vec<i32> = vec![10, 20, 30]
    println("Sum: ${sum_vec(nums)}")
}
"#;

    let (rust_code, compiles) = compile_wj_to_rust(source, "vec-idx-copy");

    println!("Generated Rust:\n{}", rust_code);

    // Copy types (i32) should NOT get .clone() when indexed
    // (nums[i] is fine because i32 is Copy)
    assert!(compiles, "Copy type Vec indexing should compile without .clone()");
}
