#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// Bug: calling a method on a deeply nested field (self.a.b.method(vec, count))
/// fails to auto-borrow the Vec argument when the method parameter is inferred as &Vec.
///
/// The compiler correctly infers the method signature as fn method(&mut self, data: &Vec<f32>, count: u32),
/// but at the call site it generates `method(vec_var, count)` instead of `method(&vec_var, count)`.
///
/// Root cause: infer_type_name may fail for deeply nested field accesses
/// (self.renderer.voxel_renderer), returning None, which prevents signature
/// lookup. Without the signature, should_add_ref falls through to false.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_field_method_auto_borrow_vec() {
    let code = r#"
pub struct GpuBackend {
    pub buffer_id: u32,
    pub light_count: u32,
}

impl GpuBackend {
    pub fn upload_lights(self, light_floats: Vec<f32>, count: u32) {
        let _len = light_floats.len()
        self.light_count = count
    }
}

pub struct Renderer {
    pub gpu: GpuBackend,
}

pub struct Game {
    pub renderer: Renderer,
}

impl Game {
    pub fn init(self) {
        let mut floats: Vec<f32> = Vec::new()
        floats.push(1.0)
        floats.push(2.0)
        floats.push(3.0)
        self.renderer.gpu.upload_lights(floats, 3)
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    println!("Generated:\n{}", generated);

    assert!(
        success,
        "Nested field method call with Vec arg must compile. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("upload_lights(&floats") || generated.contains("upload_lights(& floats"),
        "Must auto-borrow owned Vec when method param is inferred as &Vec<f32>. Generated:\n{}",
        generated
    );
}

/// Same bug pattern but through two levels of nesting with cross-crate metadata.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_field_method_auto_borrow_cross_crate() {
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    let tmp = TempDir::new().expect("tempdir");

    // "Engine" crate
    let engine_src = tmp.path().join("engine_src");
    let engine_gen = tmp.path().join("engine_gen");
    let rendering_dir = engine_src.join("rendering");
    fs::create_dir_all(&rendering_dir).expect("mkdir");

    fs::write(
        rendering_dir.join("mod.wj"),
        "pub mod gpu_renderer;\n",
    ).unwrap();

    fs::write(
        rendering_dir.join("gpu_renderer.wj"),
        r#"
pub struct VoxelGPURenderer {
    pub buffer_id: u32,
    pub point_light_count: u32,
}

impl VoxelGPURenderer {
    pub fn upload_point_lights(self, light_floats: Vec<f32>, count: u32) {
        let _len = light_floats.len()
        self.point_light_count = count
    }
}

pub struct UnifiedRenderer {
    pub voxel_renderer: VoxelGPURenderer,
}
"#,
    ).unwrap();

    let engine_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            engine_src.to_str().unwrap(),
            "--output",
            engine_gen.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("engine build");

    assert!(
        engine_build.status.success(),
        "engine build failed:\n{}",
        String::from_utf8_lossy(&engine_build.stderr)
    );

    let metadata_path = engine_gen.join("metadata.json");
    assert!(metadata_path.exists(), "engine must emit metadata.json");

    let meta_content = fs::read_to_string(&metadata_path).expect("read metadata");
    println!("Engine metadata:\n{}", meta_content);

    // "Game" crate consuming engine metadata
    let game_src = tmp.path().join("game_src");
    fs::create_dir_all(&game_src).expect("mkdir");

    fs::write(
        game_src.join("game.wj"),
        r#"
use engine::rendering::gpu_renderer::{UnifiedRenderer, VoxelGPURenderer}

pub struct MainApp {
    pub renderer: UnifiedRenderer,
}

impl MainApp {
    pub fn init(self) {
        let mut light_data: Vec<f32> = Vec::new()
        light_data.push(1.0)
        light_data.push(2.0)
        light_data.push(3.0)
        self.renderer.voxel_renderer.upload_point_lights(light_data, 3)
    }
}
"#,
    ).unwrap();

    let game_gen = tmp.path().join("game_gen");
    let game_build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            game_src.join("game.wj").to_str().unwrap(),
            "--output",
            game_gen.to_str().unwrap(),
            "--no-cargo",
            "--metadata",
            &format!("engine={}", metadata_path.display()),
        ])
        .output()
        .expect("game build");

    assert!(
        game_build.status.success(),
        "game build failed:\n{}",
        String::from_utf8_lossy(&game_build.stderr)
    );

    // Also check engine generated code to verify signature
    let engine_gen_rs = engine_gen.join("rendering").join("gpu_renderer.rs");
    if engine_gen_rs.exists() {
        let engine_rs = fs::read_to_string(&engine_gen_rs).unwrap();
        println!("Engine generated:\n{}", engine_rs);
    }

    let generated = fs::read_to_string(game_gen.join("game.rs")).expect("game.rs");
    println!("Generated game:\n{}", generated);

    // Also dump game build stderr for debug output
    let game_stderr = String::from_utf8_lossy(&game_build.stderr);
    if !game_stderr.is_empty() {
        println!("Game build stderr:\n{}", game_stderr);
    }

    assert!(
        generated.contains("upload_point_lights(&light_data")
            || generated.contains("upload_point_lights(& light_data"),
        "Cross-crate nested field method call must auto-borrow Vec. Generated:\n{}",
        generated
    );
}
