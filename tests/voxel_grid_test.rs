/// TDD Phase 1: Voxel Grid Data Structure
/// 
/// Goal: Create basic voxel grid for storing 3D voxel data
/// This is the foundation for MagicaVoxel-quality rendering in Windjammer!

use std::path::PathBuf;
use std::fs;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_voxel_grid_creation() {
    // RED: Test creation of voxel grid
    let test_dir = std::env::temp_dir().join(format!(
        "wj_voxel_grid_{}_{}",
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

impl VoxelGrid {
    fn new(width: i32, height: i32, depth: i32) -> VoxelGrid {
        let size = width * height * depth;
        VoxelGrid {
            width: width,
            height: height,
            depth: depth,
            data: Vec::new(), // Will initialize with size
        }
    }
    
    fn width(self) -> i32 {
        self.width
    }
    
    fn height(self) -> i32 {
        self.height
    }
    
    fn depth(self) -> i32 {
        self.depth
    }
}

fn main() {
    let grid = VoxelGrid::new(16, 16, 16);
    println("Grid created: {}x{}x{}", grid.width(), grid.height(), grid.depth());
}
"#;
    
    fs::write(test_dir.join("voxel_grid.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("voxel_grid.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for VoxelGrid creation");

    let rust_code = fs::read_to_string(test_dir.join("build/voxel_grid.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Verify struct is generated correctly
    assert!(rust_code.contains("struct VoxelGrid"));
    assert!(rust_code.contains("width: i32"));
    assert!(rust_code.contains("fn new("));
    assert!(rust_code.contains("fn width("));

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_voxel_set_get() {
    // RED: Test setting and getting voxel values
    let test_dir = std::env::temp_dir().join(format!(
        "wj_voxel_setget_{}_{}",
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

impl VoxelGrid {
    fn new(width: i32, height: i32, depth: i32) -> VoxelGrid {
        let size = (width * height * depth) as usize;
        let mut data = Vec::new();
        for i in 0..size {
            data.push(0);
        }
        VoxelGrid {
            width: width,
            height: height,
            depth: depth,
            data: data,
        }
    }
    
    fn get(self, x: i32, y: i32, z: i32) -> u8 {
        if !self.is_valid(x, y, z) {
            return 0;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx]
    }
    
    fn set(self, x: i32, y: i32, z: i32, value: u8) {
        if !self.is_valid(x, y, z) {
            return;
        }
        let idx = (x + y * self.width + z * self.width * self.height) as usize;
        self.data[idx] = value;
    }
    
    fn is_valid(self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < self.width &&
        y >= 0 && y < self.height &&
        z >= 0 && z < self.depth
    }
}

fn main() {
    let mut grid = VoxelGrid::new(8, 8, 8);
    grid.set(2, 3, 4, 255);
    let value = grid.get(2, 3, 4);
    println("Voxel at (2,3,4): {}", value);
    
    let empty = grid.get(0, 0, 0);
    println("Voxel at (0,0,0): {}", empty);
}
"#;
    
    fs::write(test_dir.join("voxel_setget.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("voxel_setget.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for voxel set/get");

    let rust_code = fs::read_to_string(test_dir.join("build/voxel_setget.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Verify methods are generated
    assert!(rust_code.contains("fn get("));
    assert!(rust_code.contains("fn set("));
    assert!(rust_code.contains("fn is_valid("));

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_voxel_color() {
    // RED: Test voxel color structure
    let test_dir = std::env::temp_dir().join(format!(
        "wj_voxel_color_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct VoxelColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl VoxelColor {
    fn new(r: u8, g: u8, b: u8, a: u8) -> VoxelColor {
        VoxelColor { r: r, g: g, b: b, a: a }
    }
    
    fn from_hex(hex: u32) -> VoxelColor {
        let r = ((hex >> 24) & 0xFF) as u8;
        let g = ((hex >> 16) & 0xFF) as u8;
        let b = ((hex >> 8) & 0xFF) as u8;
        let a = (hex & 0xFF) as u8;
        VoxelColor { r: r, g: g, b: b, a: a }
    }
    
    fn to_hex(self) -> u32 {
        let r_shifted = (self.r as u32) << 24;
        let g_shifted = (self.g as u32) << 16;
        let b_shifted = (self.b as u32) << 8;
        let a_u32 = self.a as u32;
        r_shifted | g_shifted | b_shifted | a_u32
    }
}

fn main() {
    let color = VoxelColor::new(255, 128, 64, 255);
    println("Color: R={} G={} B={} A={}", color.r, color.g, color.b, color.a);
    
    let from_hex = VoxelColor::from_hex(0xFF8040FF);
    println("From hex: R={} G={} B={}", from_hex.r, from_hex.g, from_hex.b);
    
    let back_to_hex = color.to_hex();
    println("To hex: 0x{:08X}", back_to_hex);
}
"#;
    
    fs::write(test_dir.join("voxel_color.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("voxel_color.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for VoxelColor");

    let rust_code = fs::read_to_string(test_dir.join("build/voxel_color.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Verify color methods
    assert!(rust_code.contains("struct VoxelColor"));
    assert!(rust_code.contains("fn from_hex("));
    assert!(rust_code.contains("fn to_hex("));

    fs::remove_dir_all(test_dir).ok();
}
