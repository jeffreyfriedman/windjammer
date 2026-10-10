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

//! exclusive match arms must move an owned formal, not `chunk.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_match_arm_must_move_owned_chunk() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("chunk.clone()");
    assert!(
        !cloned,
        "RED exclusive match arm cloned chunk:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

/// `get_local_voxel` + `should_generate_face(&chunk)` mix.
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
fn module_file_readonly_getter_must_not_mut_then_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_READONLY_GETTER);
    let map = test.compile().expect("getter compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("getter MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("chunk.clone()"),
        "RED readonly getter forced exclusive-arm clone:\n{rs}"
    );
    assert!(
        !rs.contains("fn generate(chunk: &mut Chunk"),
        "RED generate demoted to &mut Chunk:\n{rs}"
    );
    test.cargo_check().expect("getter cargo-check");
}

/// `VoxelChunk` lives in another module; `get_local_voxel` is imported.
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
fn module_file_cross_module_getter_must_not_mut_then_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("chunk.wj", CHUNK_MOD);
    test.add_file("mesh.wj", MESH_MOD);
    let map = test.compile().expect("cross-module compile");
    let rs = map.get("mesh.rs").expect("mesh.rs");
    eprintln!("cross-module mesh.rs:\n{rs}");
    assert!(
        !rs.contains("chunk.clone()"),
        "RED cross-module getter forced exclusive-arm clone:\n{rs}"
    );
    assert!(
        !rs.contains("fn generate(chunk: &mut Chunk")
            && !rs.contains("fn generate(chunk: &mut crate::chunk::Chunk"),
        "RED generate demoted to &mut Chunk:\n{rs}"
    );
    test.cargo_check().expect("cross-module cargo-check");
}
