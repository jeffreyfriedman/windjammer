/// Dogfooding test: Real-world shader compilation
///
/// Tests that realistic shader code compiles to valid WGSL
use std::fs;
use std::process::Command;

fn transpile_wj_to_wgsl(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_wgsl_dogfood_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("shader.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("wgsl")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}",
            stderr, stdout
        );
    }

    let wgsl_file = out_dir.join("shader.wgsl");
    let content = fs::read_to_string(&wgsl_file).expect("Failed to read generated WGSL file");

    let _ = fs::remove_dir_all(&test_dir);

    content
}

#[test]
fn test_camera_uniforms_struct() {
    // Real-world camera uniform buffer layout
    let source = r#"
pub struct CameraUniforms {
    view_matrix: mat4x4<float>,
    proj_matrix: mat4x4<float>,
    inv_view: mat4x4<float>,
    inv_proj: mat4x4<float>,
    position: vec3<float>,
    screen_size: vec2<float>,
    near_plane: float,
    far_plane: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Verify struct is generated
    assert!(generated.contains("struct CameraUniforms"));
    assert!(generated.contains("view_matrix: mat4x4<f32>"));
    assert!(generated.contains("position: vec3<f32>"));
    
    // Should have padding after position (vec3)
    assert!(generated.contains("_pad"));
}

#[test]
fn test_ray_aabb_intersection() {
    // Real-world ray-AABB intersection function from voxel renderer
    let source = r#"
pub fn ray_aabb(
    origin: vec3<float>,
    inv_dir: vec3<float>,
    box_min: vec3<float>,
    box_max: vec3<float>
) -> vec2<float> {
    let t1 = (box_min - origin) * inv_dir
    let t2 = (box_max - origin) * inv_dir
    let tmin = min(t1, t2)
    let tmax = max(t1, t2)
    let t_near = max(max(tmin.x, tmin.y), tmin.z)
    let t_far = min(min(tmax.x, tmax.y), tmax.z)
    vec2(t_near, t_far)
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Verify function signature
    assert!(generated.contains("fn ray_aabb"));
    assert!(generated.contains("vec3<f32>"));
    assert!(generated.contains("-> vec2<f32>"));
    
    // Verify return statement
    assert!(generated.contains("return"));
}

#[test]
fn test_complete_simple_shader() {
    // Complete simple shader with structs and functions
    let source = r#"
pub struct Ray {
    origin: vec3<float>,
    direction: vec3<float>,
}

pub struct Hit {
    position: vec3<float>,
    distance: float,
    normal: vec3<float>,
    material_id: uint,
}

pub fn sphere_intersect(ray: Ray, center: vec3<float>, radius: float) -> float {
    let oc = ray.origin - center
    let a = dot(ray.direction, ray.direction)
    let b = 2.0 * dot(oc, ray.direction)
    let c = dot(oc, oc) - radius * radius
    let discriminant = b * b - 4.0 * a * c
    
    if discriminant < 0.0 {
        return -1.0
    }
    
    (-b - sqrt(discriminant)) / (2.0 * a)
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Verify structs
    assert!(generated.contains("struct Ray"));
    assert!(generated.contains("struct Hit"));
    
    // Verify function
    assert!(generated.contains("fn sphere_intersect"));
    
    // Verify control flow
    assert!(generated.contains("if"));
    assert!(generated.contains("return"));
}

#[test]
fn test_gbuffer_pixel_layout() {
    // Real G-buffer pixel format from game engine
    let source = r#"
pub struct GBufferPixel {
    position: vec3<float>,
    normal: vec3<float>,
    material_id: float,
    depth: float,
    geometry_source: float,
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("struct GBufferPixel"));
    
    // Should have padding between vec3 fields
    assert!(generated.contains("_pad"));
}

#[test]
fn test_svo_node_functions() {
    // Sparse Voxel Octree node decoding functions
    let source = r#"
pub fn svo_get_material(node: uint) -> uint {
    node & 255
}

pub fn svo_is_leaf(node: uint) -> bool {
    (node & 256) != 0
}

pub fn svo_child_ptr(node: uint) -> uint {
    node >> 9
}

pub fn get_octant(p: vec3<float>, center: vec3<float>) -> uint {
    let mut idx = 0
    if p.x >= center.x {
        idx = idx | 1
    }
    if p.y >= center.y {
        idx = idx | 2
    }
    if p.z >= center.z {
        idx = idx | 4
    }
    idx
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Verify all functions generated
    assert!(generated.contains("fn svo_get_material"));
    assert!(generated.contains("fn svo_is_leaf"));
    assert!(generated.contains("fn svo_child_ptr"));
    assert!(generated.contains("fn get_octant"));
    
    // Verify bitwise operations
    assert!(generated.contains("&"));
    assert!(generated.contains("|"));
    assert!(generated.contains(">>"));
}
