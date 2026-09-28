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
    feature = "codegen_tests",
))]

//! WDB-417: exclusive match arms must move an owned formal, not `chunk.clone()`.
//!
//! Product `mesh_generator.rs`:
//!   `chunk: &mut VoxelChunk` then `generate_culled_mesh(chunk.clone())`
//! WJ is `generate_chunk_mesh(chunk: VoxelChunk, …)` with `generate_culled_mesh(chunk)`
//! in exclusive arms — no `.clone()`.
//! Distinct from WDB-407 (`Vec` `new` demote), WDB-415 (clone inside `if`),
//! and WDB-414 (`new(self.scene)`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Chunk {
    pub label: string,
}

pub enum Strat {
    Naive,
    Culled,
    Greedy,
}

pub fn gen_naive(chunk: Chunk) -> i32 {
    1
}

pub fn gen_culled(chunk: Chunk) -> i32 {
    2
}

pub fn gen_greedy(chunk: Chunk) -> i32 {
    3
}

pub fn generate(chunk: Chunk, strategy: Strat) -> i32 {
    match strategy {
        Strat::Naive => gen_naive(chunk),
        Strat::Culled => gen_culled(chunk),
        Strat::Greedy => gen_greedy(chunk),
    }
}
"#;

#[test]
fn wdb417_module_file_match_arm_must_move_owned_chunk() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-417 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-417 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("chunk.clone()");
    assert!(
        !cloned,
        "WDB-417 RED: exclusive match arm cloned chunk:\n{rs}"
    );
    test.cargo_check().expect("WDB-417 cargo-check");
}

/// Product shape: `get_local_voxel` + `should_generate_face(&chunk)` mix.
/// Naive-only method use must not force `generate(chunk: &mut Chunk)` and then
/// `gen_culled(chunk.clone())` for the owned exclusive arms.
const SRC_READONLY_GETTER: &str = r#"
use std::collections::HashMap

pub enum Voxel {
    Air,
    Stone,
}

impl Voxel {
    pub fn is_solid(self) -> bool {
        match self {
            Voxel::Air => false,
            Voxel::Stone => true,
        }
    }
}

pub struct Chunk {
    pub size: i32,
    data: HashMap<i32, Voxel>,
}

impl Chunk {
    pub fn get_local(self, x: i32) -> Voxel {
        if let Some(v) = self.data.get(x) {
            v
        } else {
            Voxel::Air
        }
    }
}

pub enum Strat {
    Naive,
    Culled,
    Greedy,
}

fn gen_naive(chunk: Chunk) -> i32 {
    let v = chunk.get_local(0)
    if v.is_solid() {
        1
    } else {
        0
    }
}

fn should_face(chunk: Chunk, x: i32) -> bool {
    let v = chunk.get_local(x)
    !v.is_solid()
}

fn gen_culled(chunk: Chunk) -> i32 {
    let v = chunk.get_local(0)
    if should_face(chunk, 0) {
        2
    } else {
        0
    }
}

fn gen_greedy(chunk: Chunk) -> i32 {
    let v = chunk.get_local(0)
    if should_face(chunk, 0) {
        3
    } else {
        0
    }
}

pub fn generate(chunk: Chunk, strategy: Strat) -> i32 {
    match strategy {
        Strat::Naive => gen_naive(chunk),
        Strat::Culled => gen_culled(chunk),
        Strat::Greedy => gen_greedy(chunk),
    }
}
"#;

#[test]
fn wdb417_module_file_readonly_getter_must_not_mut_then_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_READONLY_GETTER);
    let map = test.compile().expect("WDB-417 getter compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-417 getter MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("chunk.clone()"),
        "WDB-417 RED: readonly getter forced exclusive-arm clone:\n{rs}"
    );
    assert!(
        !rs.contains("fn generate(chunk: &mut Chunk"),
        "WDB-417 RED: generate demoted to &mut Chunk:\n{rs}"
    );
    test.cargo_check().expect("WDB-417 getter cargo-check");
}

/// Product: `VoxelChunk` lives in another module; `get_local_voxel` is imported.
const MOD: &str = r#"
pub mod chunk
pub mod mesh
"#;

const CHUNK_MOD: &str = r#"
use std::collections::HashMap

pub enum Voxel {
    Air,
    Stone,
}

impl Voxel {
    pub fn is_solid(self) -> bool {
        match self {
            Voxel::Air => false,
            Voxel::Stone => true,
        }
    }
}

pub struct Chunk {
    pub size: i32,
    data: HashMap<i32, Voxel>,
}

impl Chunk {
    pub fn set_local(self, x: i32, voxel: Voxel) {
        self.data.insert(x, voxel)
    }

    pub fn get_local(self, x: i32) -> Voxel {
        if let Some(v) = self.data.get(x) {
            v
        } else {
            Voxel::Air
        }
    }
}
"#;

const MESH_MOD: &str = r#"
use crate::chunk::Chunk

pub enum Strat {
    Naive,
    Culled,
    Greedy,
}

fn gen_naive(chunk: Chunk) -> i32 {
    let v = chunk.get_local(0)
    if v.is_solid() {
        1
    } else {
        0
    }
}

fn should_face(chunk: Chunk, x: i32) -> bool {
    let v = chunk.get_local(x)
    !v.is_solid()
}

fn gen_culled(chunk: Chunk) -> i32 {
    let v = chunk.get_local(0)
    if should_face(chunk, 0) {
        2
    } else {
        0
    }
}

fn gen_greedy(chunk: Chunk) -> i32 {
    let v = chunk.get_local(0)
    if should_face(chunk, 0) {
        3
    } else {
        0
    }
}

pub fn generate(chunk: Chunk, strategy: Strat) -> i32 {
    match strategy {
        Strat::Naive => gen_naive(chunk),
        Strat::Culled => gen_culled(chunk),
        Strat::Greedy => gen_greedy(chunk),
    }
}
"#;

#[test]
fn wdb417_module_file_cross_module_getter_must_not_mut_then_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("chunk.wj", CHUNK_MOD);
    test.add_file("mesh.wj", MESH_MOD);
    let map = test.compile().expect("WDB-417 cross-module compile");
    let rs = map.get("mesh.rs").expect("mesh.rs");
    eprintln!("WDB-417 cross-module mesh.rs:\n{rs}");
    assert!(
        !rs.contains("chunk.clone()"),
        "WDB-417 RED: cross-module getter forced exclusive-arm clone:\n{rs}"
    );
    assert!(
        !rs.contains("fn generate(chunk: &mut Chunk")
            && !rs.contains("fn generate(chunk: &mut crate::chunk::Chunk"),
        "WDB-417 RED: generate demoted to &mut Chunk:\n{rs}"
    );
    test.cargo_check().expect("WDB-417 cross-module cargo-check");
}

fn wdb417_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn wdb417_tip_out_game_core_match_arm_must_not_clone_chunk() {
    let mut paths = Vec::new();
    for dir in wdb417_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/mesh_generator.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/rendering/mesh_generator.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("generate_culled_mesh(chunk.clone())")
                    || t.contains("generate_greedy_mesh(chunk.clone())"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-417: mesh_generator product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-417 RED: tip/product match arms cloned chunk in:\n  {}",
        bad_paths.join("\n  ")
    );
}
