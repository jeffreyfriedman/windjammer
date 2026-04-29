//! Compiler integration: BFS SVO builder (Windjammer → Rust) preserves the layout invariant that
//! every interior node's eight children occupy eight consecutive node indices.
//!
//! Node encoding (matches `svo_convert.wj` / GPU traversal):
//! - Bits 0–7: material id
//! - Bit 8: leaf flag (`LEAF_FLAG = 0x100`)
//! - Bits 9–31: child pointer (index of first child; eight children are at ptr..ptr+7)
//!
//! Nested `cargo test` uses `target/svo_bfs_layout_verify_target/` so dependencies are not rebuilt
//! for every temp project (first run on a clean tree can still take several minutes).

#[path = "integration_test_helpers.rs"]
mod integration_test_helpers;

use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use integration_test_helpers::MultiFileTest;

/// Same lock as `integration_test_helpers` pattern: serialized `cargo` to reduce flakes under parallel test runs.
static CARGO_TEST_LOCK: Mutex<()> = Mutex::new(());

fn windjammer_runtime_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime")
}

/// `pub mod stem;` for every `stem.rs` in `build/` except `lib.rs`.
fn write_flat_lib_rs(build_dir: &Path) -> io::Result<()> {
    let mut stems: Vec<String> = Vec::new();
    for entry in fs::read_dir(build_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("rs")
            && path.file_stem().and_then(|s| s.to_str()) != Some("lib")
        {
            let stem = path
                .file_stem()
                .expect("stem")
                .to_string_lossy()
                .into_owned();
            // `mod.rs` is the compiler's crate-root aggregator; do not `pub mod mod;` (invalid) or double-include modules.
            if stem == "mod" {
                continue;
            }
            stems.push(stem);
        }
    }
    stems.sort();
    let body: String = stems
        .into_iter()
        .map(|s| format!("pub mod {};\n", s))
        .collect();
    fs::write(build_dir.join("lib.rs"), body)
}

fn write_verify_cargo_toml_with_test(build_dir: &Path) -> io::Result<()> {
    let runtime = windjammer_runtime_path();
    let runtime_display = runtime
        .canonicalize()
        .unwrap_or(runtime)
        .display()
        .to_string();
    let cargo = format!(
        r#"[package]
name = "wj_svo_bfs_layout_verify"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
windjammer-runtime = {{ path = "{}" }}
smallvec = "1.13"
serde = {{ version = "1.0", features = ["derive"] }}

[lib]
path = "lib.rs"
name = "wj_svo_bfs_layout_verify"

[[test]]
name = "svo_bfs_layout_generated"
path = "tests/svo_bfs_layout_generated.rs"
"#,
        runtime_display
    );
    fs::write(build_dir.join("Cargo.toml"), cargo)
}

fn write_generated_layout_test(build_dir: &Path) -> io::Result<()> {
    let test_dir = build_dir.join("tests");
    fs::create_dir_all(&test_dir)?;
    fs::write(
        test_dir.join("svo_bfs_layout_generated.rs"),
        r##"use wj_svo_bfs_layout_verify::harness;

const LEAF_FLAG: u32 = 0x100;

fn is_leaf(node: u32) -> bool {
    (node & LEAF_FLAG) != 0
}

fn child_base(node: u32) -> usize {
    (node >> 9) as usize
}

/// Validates BFS SVO layout: interior nodes point at 8 contiguous indices; ranges are disjoint;
/// leaves have the leaf flag set.
fn assert_bfs_svo_layout(svo: &[u32]) {
    assert!(!svo.is_empty(), "SVO must be non-empty");
    assert!(
        !is_leaf(svo[0]),
        "fixture must yield an interior root; root node = 0x{:08x}",
        svo[0]
    );

    let mut child_blocks: Vec<(usize, usize)> = Vec::new();

    for (i, &node) in svo.iter().enumerate() {
        if is_leaf(node) {
            assert!(
                (node & LEAF_FLAG) != 0,
                "leaf at index {} must have leaf flag set (node=0x{:08x})",
                i,
                node
            );
            continue;
        }

        let base = child_base(node);
        let end = base + 8;
        assert!(
            end <= svo.len(),
            "interior node at {}: child block [{}, {}) exceeds len {}",
            i,
            base,
            end,
            svo.len()
        );

        for k in 0..8 {
            let ci = base + k;
            assert!(
                ci < svo.len(),
                "interior at {}: child index {} out of bounds",
                i,
                ci
            );
        }

        child_blocks.push((base, end));
    }

    child_blocks.sort_by_key(|r| r.0);
    for w in child_blocks.windows(2) {
        assert!(
            w[0].1 <= w[1].0,
            "child blocks overlap: {:?} and {:?}",
            w[0],
            w[1]
        );
    }

    let root_base = child_base(svo[0]);
    for k in 0..8 {
        assert!(
            root_base + k < svo.len(),
            "root children must be in-bounds"
        );
    }
}

#[test]
fn windjammer_bfs_svo_children_are_eight_contiguous_indices() {
    let svo = harness::sample_svo_nodes();
    assert_bfs_svo_layout(&svo);
}
"##,
    )
}

/// Minimal voxel grid + BFS SVO flatten (same shape as `windjammer-game-core` `svo_convert.wj`).
fn add_svo_bfs_compiler_fixture(test: &mut MultiFileTest) {
    test.add_file(
        "voxel_grid.wj",
        r#"
pub struct VoxelGrid {
    width: i32,
    height: i32,
    depth: i32,
    data: Vec<u8>,
}

impl VoxelGrid {
    pub fn new(width: i32, height: i32, depth: i32) -> VoxelGrid {
        let size = (width * height * depth) as usize
        let mut data = Vec::new()
        let mut i: usize = 0usize
        while i < size {
            data.push(0u8)
            i = i + 1usize
        }
        VoxelGrid {
            width: width,
            height: height,
            depth: depth,
            data: data,
        }
    }

    pub fn get(self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return 0u8
        }
        let pos = (x + y * self.width + z * self.width * self.height) as usize
        self.data[pos]
    }

    pub fn set(self, x: i32, y: i32, z: i32, value: u8) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return
        }
        let pos = (x + y * self.width + z * self.width * self.height) as usize
        self.data[pos] = value
    }

    pub fn width(self) -> i32 { self.width }
    pub fn height(self) -> i32 { self.height }
    pub fn depth(self) -> i32 { self.depth }
}
"#,
    );

    test.add_file(
        "svo_bfs.wj",
        r#"
use voxel_grid::VoxelGrid

const LEAF_FLAG: u32 = 0x100

pub fn voxelgrid_to_svo_flat(grid: VoxelGrid) -> Vec<u32> {
    let w = grid.width()
    let h = grid.height()
    let d = grid.depth()
    let max_size = w.max(h).max(d)

    let mut nodes: Vec<u32> = Vec::new()
    let mut queue_idx: Vec<usize> = Vec::new()
    let mut queue_mx: Vec<i32> = Vec::new()
    let mut queue_my: Vec<i32> = Vec::new()
    let mut queue_mz: Vec<i32> = Vec::new()
    let mut queue_sz: Vec<i32> = Vec::new()

    nodes.push(0u32)
    queue_idx.push(0)
    queue_mx.push(0)
    queue_my.push(0)
    queue_mz.push(0)
    queue_sz.push(max_size)

    let mut front: usize = 0

    while front < queue_idx.len() {
        let node_index = queue_idx[front]
        let min_x = queue_mx[front]
        let min_y = queue_my[front]
        let min_z = queue_mz[front]
        let size = queue_sz[front]
        front = front + 1

        if size == 1 {
            let material = grid.get(min_x, min_y, min_z)
            nodes[node_index] = (material as u32) | LEAF_FLAG
        } else {
            let first_material: u8 = grid.get(min_x, min_y, min_z)
            let mut is_homogeneous = true
            let mut has_any_voxel = first_material != 0u8

            let max_x: i32 = (min_x + size).min(grid.width())
            let max_y: i32 = (min_y + size).min(grid.height())
            let max_z: i32 = (min_z + size).min(grid.depth())

            let mut z: i32 = min_z
            while z < max_z && is_homogeneous {
                let mut y: i32 = min_y
                while y < max_y && is_homogeneous {
                    let mut x: i32 = min_x
                    while x < max_x {
                        let mat: u8 = grid.get(x, y, z)
                        if mat != first_material {
                            is_homogeneous = false
                        }
                        if mat != 0u8 {
                            has_any_voxel = true
                        }
                        x = x + 1i32
                    }
                    y = y + 1i32
                }
                z = z + 1i32
            }

            if is_homogeneous {
                nodes[node_index] = (first_material as u32) | LEAF_FLAG
            } else if !has_any_voxel {
                nodes[node_index] = LEAF_FLAG
            } else {
                let child_base: u32 = nodes.len() as u32
                nodes[node_index] = child_base << 9

                let half = size / 2

                let mut c: i32 = 0
                while c < 8 {
                    let cx = min_x + half * (c & 1)
                    let cy = min_y + half * ((c >> 1) & 1)
                    let cz = min_z + half * ((c >> 2) & 1)

                    let slot = nodes.len()
                    nodes.push(0u32)
                    queue_idx.push(slot)
                    queue_mx.push(cx)
                    queue_my.push(cy)
                    queue_mz.push(cz)
                    queue_sz.push(half)

                    c = c + 1
                }
            }
        }
    }

    nodes
}
"#,
    );

    test.add_file(
        "harness.wj",
        r#"
use voxel_grid::VoxelGrid
use svo_bfs::voxelgrid_to_svo_flat

/// 8³ grid with multiple materials so the root subdivides (interior root) and deeper nodes appear.
pub fn sample_svo_nodes() -> Vec<u32> {
    let mut g = VoxelGrid::new(8, 8, 8)
    g.set(0, 0, 0, 2u8)
    g.set(7, 7, 7, 3u8)
    g.set(4, 4, 4, 4u8)
    voxelgrid_to_svo_flat(g)
}
"#,
    );
}

/// Multipass compile only: ensures the BFS SVO builder emits expected Rust (runs in default `cargo test`).
#[test]
fn test_bfs_svo_windjammer_codegen_emits_child_pointer_shift() {
    let mut test = MultiFileTest::new();
    add_svo_bfs_compiler_fixture(&mut test);

    let map = test.compile().expect("Windjammer multipass compile");
    let svo_rs = map.get("svo_bfs.rs").expect("svo_bfs.rs");
    assert!(
        svo_rs.contains("<< 9") || svo_rs.contains("<<9"),
        "expected child pointer encoding (<< 9); got:\n{svo_rs}"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_bfs_svo_windjammer_compiles_and_layout_holds() {
    let mut test = MultiFileTest::new();
    add_svo_bfs_compiler_fixture(&mut test);

    let map = test
        .compile()
        .unwrap_or_else(|e| panic!("Windjammer multipass compile failed: {}", e));

    let svo_rs = map.get("svo_bfs.rs").expect("svo_bfs.rs");
    assert!(
        svo_rs.contains("<< 9") || svo_rs.contains("<<9"),
        "expected child pointer encoding (<< 9) in generated Rust; got:\n{svo_rs}"
    );

    let build_dir = test.build_dir();
    write_flat_lib_rs(build_dir).expect("write lib.rs");
    write_verify_cargo_toml_with_test(build_dir).expect("write Cargo.toml");
    write_generated_layout_test(build_dir).expect("write generated layout test");

    let _guard = CARGO_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // Reuse a single target dir under the windjammer crate so windjammer-runtime + serde deps are
    // not rebuilt from scratch for every temp project (fresh tempdir would otherwise cost minutes).
    let shared_target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("svo_bfs_layout_verify_target");

    let test_run = Command::new("cargo")
        .current_dir(build_dir)
        .env("CARGO_TARGET_DIR", &shared_target)
        .args(["test", "--test", "svo_bfs_layout_generated", "--quiet"])
        .output()
        .expect("spawn cargo test");

    assert!(
        test_run.status.success(),
        "layout verification test failed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&test_run.stdout),
        String::from_utf8_lossy(&test_run.stderr)
    );
}
