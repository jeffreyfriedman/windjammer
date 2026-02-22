/// TDD Phase 2: Greedy Meshing Algorithm
/// 
/// Goal: Convert voxel grid to optimized triangle mesh
/// This enables rendering voxels with 90%+ triangle reduction!

use std::path::PathBuf;
use std::fs;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_face_extraction_single_voxel() {
    // RED: Test face extraction for isolated voxel
    let test_dir = std::env::temp_dir().join(format!(
        "wj_face_extract_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct VoxelGrid {
    width: i32,
    height: i32,
    depth: i32,
    data: Vec<u8>,
}

enum Direction {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

struct VoxelFace {
    x: i32,
    y: i32,
    z: i32,
    direction: Direction,
}

impl VoxelGrid {
    fn new(width: i32, height: i32, depth: i32) -> VoxelGrid {
        let size = (width * height * depth) as usize;
        let mut data = Vec::new();
        for i in 0..size {
            data.push(0);
        }
        VoxelGrid { width, height, depth, data }
    }
    
    fn get(self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return 0;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx]
    }
    
    fn set(self, x: i32, y: i32, z: i32, value: u8) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx] = value;
    }
}

fn extract_visible_faces(grid: VoxelGrid) -> Vec<VoxelFace> {
    let mut faces = Vec::new();
    
    for x in 0..grid.width {
        for y in 0..grid.height {
            for z in 0..grid.depth {
                let voxel = grid.get(x, y, z);
                if voxel == 0 {
                    continue;
                }
                
                // Check all 6 directions
                if grid.get(x + 1, y, z) == 0 {
                    faces.push(VoxelFace { x, y, z, direction: Direction::PosX });
                }
                if grid.get(x - 1, y, z) == 0 {
                    faces.push(VoxelFace { x, y, z, direction: Direction::NegX });
                }
                if grid.get(x, y + 1, z) == 0 {
                    faces.push(VoxelFace { x, y, z, direction: Direction::PosY });
                }
                if grid.get(x, y - 1, z) == 0 {
                    faces.push(VoxelFace { x, y, z, direction: Direction::NegY });
                }
                if grid.get(x, y, z + 1) == 0 {
                    faces.push(VoxelFace { x, y, z, direction: Direction::PosZ });
                }
                if grid.get(x, y, z - 1) == 0 {
                    faces.push(VoxelFace { x, y, z, direction: Direction::NegZ });
                }
            }
        }
    }
    
    faces
}

fn main() {
    let mut grid = VoxelGrid::new(3, 3, 3);
    grid.set(1, 1, 1, 255);
    
    let faces = extract_visible_faces(grid);
    println("Faces extracted: {}", faces.len());
}
"#;
    
    fs::write(test_dir.join("face_extract.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("face_extract.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for face extraction");

    let rust_code = fs::read_to_string(test_dir.join("build/face_extract.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Verify enum and face extraction function
    assert!(rust_code.contains("enum Direction"));
    assert!(rust_code.contains("struct VoxelFace"));
    assert!(rust_code.contains("fn extract_visible_faces"));

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_quad_structure() {
    // RED: Test quad mesh output structure
    let test_dir = std::env::temp_dir().join(format!(
        "wj_quad_mesh_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}

struct Quad {
    vertices: [Vec3; 4],
    normal: Vec3,
}

impl Quad {
    fn new(v0: Vec3, v1: Vec3, v2: Vec3, v3: Vec3, normal: Vec3) -> Quad {
        Quad {
            vertices: [v0, v1, v2, v3],
            normal: normal,
        }
    }
}

struct VoxelMesh {
    quads: Vec<Quad>,
}

impl VoxelMesh {
    fn new() -> VoxelMesh {
        VoxelMesh { quads: Vec::new() }
    }
    
    fn add_quad(self, quad: Quad) {
        self.quads.push(quad);
    }
    
    fn quad_count(self) -> i32 {
        self.quads.len() as i32
    }
    
    fn triangle_count(self) -> i32 {
        self.quad_count() * 2
    }
}

fn main() {
    let mut mesh = VoxelMesh::new();
    let quad = Quad::new(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0)
    );
    mesh.add_quad(quad);
    
    println("Mesh has {} quads = {} triangles", mesh.quad_count(), mesh.triangle_count());
}
"#;
    
    fs::write(test_dir.join("quad_mesh.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("quad_mesh.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for quad mesh structure");

    let rust_code = fs::read_to_string(test_dir.join("build/quad_mesh.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Verify mesh structures
    assert!(rust_code.contains("struct Vec3"));
    assert!(rust_code.contains("struct Quad"));
    assert!(rust_code.contains("struct VoxelMesh"));
    assert!(rust_code.contains("fn triangle_count("));

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_greedy_meshing_basic() {
    // RED: Test basic greedy meshing (merge adjacent faces)
    let test_dir = std::env::temp_dir().join(format!(
        "wj_greedy_mesh_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct VoxelGrid {
    width: i32,
    height: i32,
    depth: i32,
    data: Vec<u8>,
}

struct Quad {
    x: i32,
    y: i32,
    z: i32,
    width: i32,
    height: i32,
}

impl VoxelGrid {
    fn new(width: i32, height: i32, depth: i32) -> VoxelGrid {
        let size = (width * height * depth) as usize;
        let mut data = Vec::new();
        for i in 0..size {
            data.push(0);
        }
        VoxelGrid { width, height, depth, data }
    }
    
    fn set(self, x: i32, y: i32, z: i32, value: u8) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx] = value;
    }
    
    fn get(self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= self.width || y < 0 || y >= self.height || z < 0 || z >= self.depth {
            return 0;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx]
    }
}

fn greedy_mesh_x_axis(grid: VoxelGrid, z: i32) -> Vec<Quad> {
    let mut quads = Vec::new();
    
    // Sweep through XY plane at fixed Z
    // Try to merge adjacent solid voxels into quads
    for y in 0..grid.height {
        for x in 0..grid.width {
            if grid.get(x, y, z) > 0 {
                // Try to expand horizontally (along X)
                let mut width = 1;
                while x + width < grid.width && grid.get(x + width, y, z) > 0 {
                    width += 1;
                }
                
                // Create quad for this run
                let quad = Quad { x, y, z, width, height: 1 };
                quads.push(quad);
                
                // Mark voxels as processed (would need visited mask in real impl)
            }
        }
    }
    
    quads
}

fn main() {
    let mut grid = VoxelGrid::new(5, 5, 5);
    
    // Create 3 voxels in a row
    grid.set(1, 1, 1, 255);
    grid.set(2, 1, 1, 255);
    grid.set(3, 1, 1, 255);
    
    let quads = greedy_mesh_x_axis(grid, 1);
    println("Generated {} quads for 3 voxels", quads.len());
}
"#;
    
    fs::write(test_dir.join("greedy_mesh.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("greedy_mesh.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for greedy meshing");

    let rust_code = fs::read_to_string(test_dir.join("build/greedy_mesh.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Verify greedy meshing function
    assert!(rust_code.contains("fn greedy_mesh_x_axis"));
    assert!(rust_code.contains("struct Quad"));
    assert!(rust_code.contains("while"));

    fs::remove_dir_all(test_dir).ok();
}
