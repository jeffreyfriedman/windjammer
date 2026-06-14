#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

// TDD: Method argument passthrough mut inference
//
// Bug: When a function passes a parameter as a NON-SELF argument to a method
// call (e.g., `cache.clear(grid, x, y, z)` where grid is the second param,
// not self), the analyzer fails to propagate the callee's &mut inference
// back to the caller's parameter.
//
// Specific pattern from game dogfooding:
//   pub fn spawn_next_wave(..., grid: VoxelGrid, mannequin: MannequinCache, ...) {
//       mannequin.clear(grid, ox, fy + 1, oz)    // grid is arg #2 of clear()
//       mannequin.place(grid, vx, fy + 1, vz, mat)  // grid is arg #2 of place()
//   }
//
// MannequinCache::clear(self, grid: VoxelGrid, ...) infers grid: &mut VoxelGrid
// But spawn_next_wave(grid: VoxelGrid) stays as owned instead of &mut VoxelGrid
//
// Root cause: The passthrough inference in infer_parameter_ownership doesn't
// handle method calls where the parameter is passed as a non-receiver argument.

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_method_arg_passthrough_infers_mut() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    std::fs::write(
        src.join("types.wj"),
        r#"
pub struct Grid {
    pub cells: Vec<i32>
}

impl Grid {
    pub fn set(self, idx: i32, val: i32) {
        self.cells[idx as usize] = val
    }

    pub fn get(self, idx: i32) -> i32 {
        self.cells[idx as usize]
    }
}

pub struct Cache {
    pub size: i32
}

impl Cache {
    pub fn clear(self, grid: Grid, x: i32, y: i32, z: i32) {
        grid.set(x, 0)
    }

    pub fn place(self, grid: Grid, x: i32, y: i32, z: i32, val: i32) {
        grid.set(x, val)
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("ops.wj"),
        r#"
use crate::types::{Grid, Cache}

pub fn do_work(items: Vec<i32>, grid: Grid, cache: Cache) {
    let mut j = 0
    while j < items.len() {
        cache.clear(grid, j as i32, 1, 0)
        j = j + 1
    }
    let mut i = 0
    while i < items.len() {
        cache.place(grid, i as i32, 1, 0, items[i])
        i = i + 1
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("game.wj"),
        r#"
use crate::types::{Grid, Cache}
use crate::ops::do_work

pub struct Game {
    pub grid: Grid,
    pub cache: Cache,
    pub items: Vec<i32>
}

impl Game {
    pub fn update(self) {
        do_work(self.items, self.grid, self.cache)
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let ops_rs = std::fs::read_to_string(build.join("ops.rs")).unwrap();
    let game_rs = std::fs::read_to_string(build.join("game.rs")).unwrap();

    println!("=== ops.rs ===\n{}", ops_rs);
    println!("=== game.rs ===\n{}", game_rs);

    // grid should be inferred as &mut Grid because:
    // - cache.clear(grid, ...) passes grid to Cache::clear which takes grid: &mut Grid
    // - cache.place(grid, ...) passes grid to Cache::place which takes grid: &mut Grid
    assert!(
        ops_rs.contains("grid: &mut Grid"),
        "do_work should have grid: &mut Grid (passthrough from cache.clear/place). Generated:\n{}",
        ops_rs
    );

    // Call site should match with &mut
    assert!(
        game_rs.contains("&mut self.grid"),
        "Call site should pass &mut self.grid. Generated:\n{}",
        game_rs
    );

    // Should NOT use .clone() for grid
    assert!(
        !game_rs.contains("self.grid.clone()"),
        "Should not clone grid. Generated:\n{}",
        game_rs
    );

    // Inside do_work, passing grid to cache.clear/place should NOT add &mut
    // since grid is already &mut Grid
    assert!(
        !ops_rs.contains("&mut grid"),
        "Should not double-borrow grid with &mut. Generated:\n{}",
        ops_rs
    );
}

#[test]
fn test_method_arg_passthrough_single_file() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // Same pattern but all in one file
    std::fs::write(
        src.join("all.wj"),
        r#"
pub struct Grid {
    pub cells: Vec<i32>
}

impl Grid {
    pub fn set(self, idx: i32, val: i32) {
        self.cells[idx as usize] = val
    }
}

pub struct Cache {
    pub size: i32
}

impl Cache {
    pub fn clear(self, grid: Grid, x: i32) {
        grid.set(x, 0)
    }

    pub fn place(self, grid: Grid, x: i32, val: i32) {
        grid.set(x, val)
    }
}

pub fn process(grid: Grid, cache: Cache, count: i32) {
    let mut i = 0
    while i < count {
        cache.clear(grid, i)
        cache.place(grid, i, i * 10)
        i = i + 1
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let all_rs = std::fs::read_to_string(build.join("all.rs")).unwrap();
    println!("=== all.rs ===\n{}", all_rs);

    // grid should be &mut Grid in process() because it's passed to
    // cache.clear() and cache.place() which both take grid: &mut Grid
    assert!(
        all_rs.contains("fn process(grid: &mut Grid"),
        "process should have grid: &mut Grid. Generated:\n{}",
        all_rs
    );

    // Should not double-borrow inside process
    assert!(
        !all_rs.contains("cache.clear(&mut grid") && !all_rs.contains("cache.place(&mut grid"),
        "Should not double-borrow grid. Generated:\n{}",
        all_rs
    );
}

/// Reproduce a spawn_next_wave-style pattern:
/// - Many parameters (8)
/// - grid passed to method args (cache.clear/place) which mutate it
/// - grid ALSO passed to a free function (convert) which only reads it
/// - grid also used in conditional paths
#[test]
fn test_method_arg_passthrough_complex_spawn_pattern() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(src.join("voxel")).unwrap();

    // Grid type with mutation methods
    std::fs::write(
        src.join("voxel/grid.wj"),
        r#"
pub struct VoxelGrid {
    pub cells: Vec<u8>,
    pub width: i32
}

impl VoxelGrid {
    pub fn set(self, x: i32, y: i32, z: i32, val: u8) {
        let idx = (y * self.width * self.width + z * self.width + x) as usize
        if idx < self.cells.len() {
            self.cells[idx] = val
        }
    }

    pub fn get(self, x: i32, y: i32, z: i32) -> u8 {
        let idx = (y * self.width * self.width + z * self.width + x) as usize
        if idx < self.cells.len() { self.cells[idx] } else { 0 }
    }

    pub fn width(self) -> i32 { self.width }
    pub fn height(self) -> i32 { self.width }
    pub fn depth(self) -> i32 { self.width }
}
"#,
    )
    .unwrap();

    // SVO convert - read-only use of grid
    std::fs::write(
        src.join("voxel/svo_convert.wj"),
        r#"
use crate::voxel::grid::VoxelGrid

pub fn voxelgrid_to_svo_flat(grid: VoxelGrid) -> Vec<u32> {
    let w = grid.width()
    let mut result = Vec::new()
    result.push(w as u32)
    result
}
"#,
    )
    .unwrap();

    // Cache type with methods that take grid as non-self arg
    std::fs::write(
        src.join("characters.wj"),
        r#"
use crate::voxel::grid::VoxelGrid

pub struct MannequinCache {
    pub data: Vec<u8>
}

impl MannequinCache {
    pub fn clear(self, grid: VoxelGrid, cx: i32, cy: i32, cz: i32) {
        grid.set(cx, cy, cz, 0)
    }

    pub fn place(self, grid: VoxelGrid, cx: i32, cy: i32, cz: i32, mat: u8) {
        grid.set(cx, cy, cz, mat)
    }
}
"#,
    )
    .unwrap();

    // Enemy type
    std::fs::write(
        src.join("enemy.wj"),
        r#"
pub struct EnemyState {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub health: f32
}

impl EnemyState {
    pub fn is_alive(self) -> bool {
        self.health > 0.0
    }
}
"#,
    )
    .unwrap();

    // Renderer type
    std::fs::write(
        src.join("renderer.wj"),
        r#"
pub struct Renderer {
    pub active: bool
}

impl Renderer {
    pub fn upload_svo(self, svo: Vec<u32>, size: f32) {
        self.active = true
    }
}
"#,
    )
    .unwrap();

    // The critical function - matching spawn_next_wave pattern exactly
    std::fs::write(
        src.join("spawning.wj"),
        r#"
use crate::voxel::grid::VoxelGrid
use crate::voxel::svo_convert
use crate::characters::MannequinCache
use crate::enemy::EnemyState
use crate::renderer::Renderer

pub fn spawn_next_wave(
    enemies: Vec<EnemyState>,
    positions: Vec<(i32, i32, i32)>,
    grid: VoxelGrid,
    mannequin: MannequinCache,
    renderer: Renderer,
    current_wave: i32,
) {
    let fy = 1
    let mut j = 0
    while j < enemies.len() {
        if !enemies[j].is_alive() {
            let (ox, _oy, oz) = positions[j]
            mannequin.clear(grid, ox, fy + 1, oz)
        }
        j = j + 1
    }
    enemies.clear()
    positions.clear()

    let count = 8 + current_wave * 2
    let mut i = 0
    while i < count {
        let x = 29 + i
        let z = 29
        let vx = x
        let vz = z
        mannequin.place(grid, vx, fy + 1, vz, 1)
        i = i + 1
    }
    let svo = svo_convert::voxelgrid_to_svo_flat(grid)
    renderer.upload_svo(svo, 64.0)
}
"#,
    )
    .unwrap();

    // Game struct that calls spawn_next_wave
    std::fs::write(
        src.join("game.wj"),
        r#"
use crate::voxel::grid::VoxelGrid
use crate::characters::MannequinCache
use crate::enemy::EnemyState
use crate::renderer::Renderer
use crate::spawning::spawn_next_wave

pub struct Game {
    pub enemies: Vec<EnemyState>,
    pub positions: Vec<(i32, i32, i32)>,
    pub grid: VoxelGrid,
    pub mannequin: MannequinCache,
    pub renderer: Renderer,
    pub current_wave: i32
}

impl Game {
    pub fn do_spawn(self) {
        spawn_next_wave(
            self.enemies,
            self.positions,
            self.grid,
            self.mannequin,
            self.renderer,
            self.current_wave
        )
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let spawning_rs = std::fs::read_to_string(build.join("spawning.rs")).unwrap();
    let game_rs = std::fs::read_to_string(build.join("game.rs")).unwrap();

    println!("=== spawning.rs ===\n{}", spawning_rs);
    println!("=== game.rs ===\n{}", game_rs);

    // grid should be &mut VoxelGrid because cache.clear/place mutate it
    assert!(
        spawning_rs.contains("grid: &mut VoxelGrid"),
        "spawn_next_wave should have grid: &mut VoxelGrid. Generated:\n{}",
        spawning_rs
    );

    // Call site should pass &mut self.grid
    assert!(
        game_rs.contains("&mut self.grid"),
        "Call site should pass &mut self.grid. Generated:\n{}",
        game_rs
    );

    // Should not double-borrow grid inside spawn_next_wave
    assert!(
        !spawning_rs.contains("&mut grid"),
        "Should not double-borrow grid. Generated:\n{}",
        spawning_rs
    );
}
