#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD Test: Parameter should NOT get &mut when its field methods are readonly
///
/// Problem: fn process(data: SomeStruct) where data.field.method() is called
/// The compiler incorrectly infers &mut for `data` because it doesn't recognize
/// that `method()` on data.field is read-only.
///
/// Example from UnifiedRenderer:
///   fn camera_data_to_gpu_state(camera: CameraData) -> GpuCameraState {
///       let arr = camera.view_matrix.to_array()
///       ...
///   }
/// Generates: fn camera_data_to_gpu_state(camera: &mut CameraData) -- WRONG
/// Expected:  fn camera_data_to_gpu_state(camera: CameraData) or (camera: &CameraData)
use std::process::Command;

#[test]
fn test_param_field_method_call_no_mut_inference() {
    let source = r#"
struct Matrix {
    data: Vec<f32>
}

impl Matrix {
    pub fn to_array(self) -> Vec<f32> {
        self.data.clone()
    }

    pub fn inverse(self) -> Matrix {
        Matrix { data: self.data.clone() }
    }
}

struct CameraData {
    view_matrix: Matrix,
    proj_matrix: Matrix,
    fov: f32,
}

struct GpuState {
    view_arr: Vec<f32>,
    proj_arr: Vec<f32>,
    inv_view_arr: Vec<f32>,
}

fn camera_to_gpu(camera: CameraData) -> GpuState {
    let view_arr = camera.view_matrix.to_array()
    let proj_arr = camera.proj_matrix.to_array()
    let inv_view_arr = camera.view_matrix.inverse().to_array()
    GpuState {
        view_arr: view_arr,
        proj_arr: proj_arr,
        inv_view_arr: inv_view_arr,
    }
}

fn main() {
    let cam = CameraData {
        view_matrix: Matrix { data: vec![1.0, 0.0, 0.0, 1.0] },
        proj_matrix: Matrix { data: vec![2.0, 0.0, 0.0, 2.0] },
        fov: 1.2,
    }
    let gpu = camera_to_gpu(cam)
    println("{}", gpu.view_arr.len())
}
"#;

    let _tmp = tempfile::tempdir().unwrap();
    let temp_dir = _tmp.path();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let output_dir = temp_dir.join(&test_id);

    let source_file = temp_dir.join("test_param_field_method.wj");
    std::fs::write(&source_file, source).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .arg("build")
        .arg(source_file.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Find the generated .rs file
    let rs_file = output_dir.join("test_param_field_method.rs");
    assert!(rs_file.exists(), "Generated .rs file not found");

    let generated = std::fs::read_to_string(&rs_file).unwrap();

    // The function should NOT have &mut on its parameter.
    // camera_to_gpu should take owned CameraData or &CameraData, NOT &mut CameraData
    assert!(
        !generated.contains("camera: &mut CameraData"),
        "BUG: camera parameter incorrectly inferred as &mut!\nGenerated:\n{}",
        generated
    );

    // It should either be owned or borrowed (read-only)
    assert!(
        generated.contains("fn camera_to_gpu(camera: CameraData)")
            || generated.contains("fn camera_to_gpu(camera: &CameraData)"),
        "Expected camera_to_gpu to take owned or &CameraData, got something else.\nGenerated:\n{}",
        generated
    );
}
