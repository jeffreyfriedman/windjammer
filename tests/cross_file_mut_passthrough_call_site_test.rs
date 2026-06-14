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

// TDD: Cross-file mut-borrow passthrough at call sites
//
// Bug: When file A defines `fn mutate(grid: VoxelGrid)` where grid is mutated
// through a method call like `cache.clear(grid, ...)`, the analyzer correctly
// infers `grid: &mut VoxelGrid`. But when file B calls this function with
// `mutate(self.grid)`, the call site generates `self.grid` (owned) or
// `self.grid.clone()` instead of `&mut self.grid`.
//
// This is the root cause of cross-module SOLID extraction failures:
// extracting methods to free functions in separate modules produces ownership
// mismatches between call sites and function signatures.
//
// There are TWO bugs:
// Bug A: Call-site doesn't generate &mut for cross-file functions with &mut params
// Bug B: When a parameter is &mut T and passed to another function expecting &mut T,
//        the codegen generates `&mut param` (double borrow) instead of just `param`

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_cross_file_mut_passthrough_call_site() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // File 1: A struct with a mutating method, and a free function that
    // passes through to that method
    std::fs::write(
        src.join("grid_ops.wj"),
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
    pub width: i32,
    pub data: Vec<i32>
}

impl Cache {
    pub fn clear(self, grid: Grid, x: i32) {
        grid.set(x, 0)
    }

    pub fn place(self, grid: Grid, x: i32, val: i32) {
        grid.set(x, val)
    }
}

pub fn sync_entities(
    entities: Vec<i32>,
    grid: Grid,
    cache: Cache,
) -> bool {
    let mut changed = false
    let mut i = 0
    while i < entities.len() {
        cache.clear(grid, i as i32)
        cache.place(grid, i as i32, entities[i])
        changed = true
        i = i + 1
    }
    changed
}
"#,
    )
    .unwrap();

    // File 2: Calls the free function from file 1, passing self.field
    std::fs::write(
        src.join("game.wj"),
        r#"
use crate::grid_ops::{Grid, Cache, sync_entities}

pub struct Game {
    pub grid: Grid,
    pub cache: Cache,
    pub entities: Vec<i32>
}

impl Game {
    pub fn new() -> Game {
        Game {
            grid: Grid { cells: Vec::new() },
            cache: Cache { width: 10, data: Vec::new() },
            entities: Vec::new()
        }
    }

    pub fn update(self) {
        let changed = sync_entities(self.entities, self.grid, self.cache)
        if changed {
            println!("entities synced")
        }
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let grid_ops_rs = std::fs::read_to_string(build.join("grid_ops.rs")).unwrap();
    let game_rs = std::fs::read_to_string(build.join("game.rs")).unwrap();

    println!("=== grid_ops.rs ===\n{}", grid_ops_rs);
    println!("=== game.rs ===\n{}", game_rs);

    // Bug A: grid parameter in sync_entities should be &mut Grid
    // because cache.clear(grid, ...) and cache.place(grid, ...) both mutate grid
    assert!(
        grid_ops_rs.contains("grid: &mut Grid"),
        "sync_entities should have grid: &mut Grid. Generated:\n{}",
        grid_ops_rs
    );

    // Bug A: cache parameter in sync_entities should be & or &mut Cache
    // (cache.clear and cache.place read cache fields but mutate grid through it)
    assert!(
        grid_ops_rs.contains("cache: &Cache") || grid_ops_rs.contains("cache: &mut Cache"),
        "sync_entities should borrow cache. Generated:\n{}",
        grid_ops_rs
    );

    // Bug A: Call site in game.rs should use &mut for grid
    assert!(
        game_rs.contains("&mut self.grid"),
        "Call site should pass &mut self.grid. Generated:\n{}",
        game_rs
    );

    // Bug B: Inside sync_entities, passing grid to cache.clear/place should NOT
    // generate &mut grid (double borrow). The grid parameter is already &mut Grid,
    // so passing it should just be `grid` or `&mut *grid`, not `&mut grid`.
    assert!(
        !grid_ops_rs.contains("&mut grid"),
        "Should not double-borrow grid with &mut grid. Generated:\n{}",
        grid_ops_rs
    );
}

#[test]
fn test_cross_file_call_site_no_clone_for_mut_params() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    std::fs::create_dir_all(&src).unwrap();

    // File 1: function that mutates a Vec parameter
    std::fs::write(
        src.join("utils.wj"),
        r#"
pub fn add_items(items: Vec<i32>, count: i32) {
    let mut i = 0
    while i < count {
        items.push(i)
        i = i + 1
    }
}
"#,
    )
    .unwrap();

    // File 2: calls the function passing self.field
    std::fs::write(
        src.join("container.wj"),
        r#"
use crate::utils::add_items

pub struct Container {
    pub items: Vec<i32>
}

impl Container {
    pub fn new() -> Container {
        Container { items: Vec::new() }
    }

    pub fn populate(self, count: i32) {
        add_items(self.items, count)
    }
}
"#,
    )
    .unwrap();

    let result = build_project_ext(&src, &build, CompilationTarget::Rust, false, false, &[]);
    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    let utils_rs = std::fs::read_to_string(build.join("utils.rs")).unwrap();
    let container_rs = std::fs::read_to_string(build.join("container.rs")).unwrap();

    println!("=== utils.rs ===\n{}", utils_rs);
    println!("=== container.rs ===\n{}", container_rs);

    // add_items should have items: &mut Vec<i32> (mutated via push)
    assert!(
        utils_rs.contains("items: &mut Vec<i32>"),
        "add_items should have items: &mut Vec<i32>. Generated:\n{}",
        utils_rs
    );

    // Call site should use &mut, NOT .clone()
    assert!(
        container_rs.contains("&mut self.items"),
        "Call site should pass &mut self.items, not clone. Generated:\n{}",
        container_rs
    );
    assert!(
        !container_rs.contains("self.items.clone()"),
        "Call site should NOT clone items. Generated:\n{}",
        container_rs
    );
}
