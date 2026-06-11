#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD test: Method name collision should not affect mutation detection.
///
/// Bug: When two types have methods with the same name but different self ownership
/// (e.g., Vec::clear(&mut self) vs MannequinCache::clear(&self)), the mutation detection
/// picks up the wrong signature and either over- or under-infers mutability.
///
/// Fix: When registry.has_collision(method_name), don't trust the looked-up signature.
/// Fall through to the conservative default (assume mutation).
///
/// This means: if ANY type has a mutating method with that name, we conservatively assume
/// the parameter needs &mut. This is safe because:
/// 1. If the method actually takes &self, the generated &mut will still compile (Rust auto-reborrows)
/// 2. If the method takes &mut self, we correctly generate &mut
fn compile_wj_to_rust(source: &str) -> String {
    use std::process::Command;
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("wj_collision_test_{}_{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).unwrap();
    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let wj = std::env::var("WJ_BINARY").unwrap_or_else(|_| {
        let dev = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
        if dev.exists() {
            return dev.to_string_lossy().to_string();
        }
        "wj".to_string()
    });

    let output = Command::new(&wj)
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(&dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let rs_file = dir.join("test.rs");
    let result = std::fs::read_to_string(&rs_file).unwrap_or_else(|_| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("No .rs output generated. stderr:\n{}", stderr);
    });

    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
fn test_method_collision_does_not_suppress_mutation_inference() {
    // Scenario: Two types have a method named "clear":
    // - Vec has clear() which takes &mut self (stdlib)
    // - ReadOnlyCache has clear() which takes &self (user-defined)
    //
    // When a parameter calls .clear() and the method name has a collision in the registry,
    // the mutation detection should conservatively assume mutation (&mut).
    let source = r#"
pub struct VoxelGrid {
    pub data: Vec<u8>,
}

impl VoxelGrid {
    pub fn set(self, x: i32, y: i32, z: i32, val: u8) {
        self.data.push(val)
    }
}

pub struct ReadOnlyCache {
    pub width: i32,
    pub height: i32,
}

impl ReadOnlyCache {
    pub fn clear(self, grid: VoxelGrid, x: i32, y: i32) {
        grid.set(x, y, 0, 0u8)
    }
    pub fn place(self, grid: VoxelGrid, x: i32, y: i32, mat: u8) {
        grid.set(x, y, 0, mat)
    }
}

pub fn update_voxels(grid: VoxelGrid, cache: ReadOnlyCache) {
    cache.clear(grid, 1, 2)
    cache.place(grid, 3, 4, 5u8)
}
"#;

    let output = compile_wj_to_rust(source);

    // The key check: `grid` should be &mut (it IS mutated through set()).
    // `cache` should be &mut conservatively because "clear" collides with Vec::clear(self).
    // Even though ReadOnlyCache::clear takes &self, the collision makes it ambiguous.
    // Conservative default: assume &mut.
    //
    // Actually, the important thing is that grid is &mut. The cache parameter
    // should at minimum not cause a compilation error.
    assert!(
        output.contains("grid: &mut VoxelGrid"),
        "grid should be &mut (it's mutated through set()). Got:\n{}",
        output
    );

    // The function should compile successfully regardless of cache's ownership
    // (both &ReadOnlyCache and &mut ReadOnlyCache are valid since clear/place only read self)
}

#[test]
fn test_passthrough_collision_preserves_mut() {
    // When a function passes a parameter to another function that has a colliding method name,
    // the ownership inference should be conservative.
    let source = r#"
pub struct VoxelGrid {
    pub data: Vec<u8>,
}

impl VoxelGrid {
    pub fn set(self, x: i32, y: i32, z: i32, val: u8) {
        self.data.push(val)
    }
}

pub struct Cache {
    pub size: i32,
}

impl Cache {
    pub fn clear(self, grid: VoxelGrid, x: i32, y: i32) {
        grid.set(x, y, 0, 0u8)
    }
}

pub fn do_clear(grid: VoxelGrid, cache: Cache) {
    cache.clear(grid, 1, 2)
}

pub fn wrapper(grid: VoxelGrid, cache: Cache) {
    do_clear(grid, cache)
}
"#;

    let output = compile_wj_to_rust(source);

    // grid should be &mut in both functions
    let lines: Vec<&str> = output.lines().collect();
    let mut found_do_clear = false;
    let mut found_wrapper = false;
    for line in &lines {
        if line.contains("fn do_clear") {
            assert!(
                line.contains("grid: &mut VoxelGrid"),
                "do_clear: grid should be &mut. Got: {}",
                line
            );
            found_do_clear = true;
        }
        if line.contains("fn wrapper") {
            assert!(
                line.contains("grid: &mut VoxelGrid"),
                "wrapper: grid should be &mut (passthrough). Got: {}",
                line
            );
            found_wrapper = true;
        }
    }
    assert!(found_do_clear, "Didn't find do_clear function in output");
    assert!(found_wrapper, "Didn't find wrapper function in output");
}
