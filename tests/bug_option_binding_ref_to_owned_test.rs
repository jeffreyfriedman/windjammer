#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// TDD test: When `if let Some(x) = self.field` binds x from a &mut scrutinee,
/// and x is then passed to a function expecting an owned value, the compiler
/// must add .clone() to the argument to convert &mut T → T.
///
/// Bug: Generated code passes `cam` (which is `&mut CameraData`) directly to
/// `set_camera(cam)` which expects `CameraData`, causing E0308 mismatched types.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_binding_clone_when_passed_as_owned() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_option_binding_ref_to_owned");

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_source = r#"
struct CameraData {
    fov: f32,
    near: f32,
    far: f32,
    name: string
}

struct MeshRenderer {
    fov: f32
}

impl MeshRenderer {
    pub fn set_camera(self, cam: CameraData) {
        self.fov = cam.fov
    }
}

struct VoxelRenderer {
    fov: f32
}

impl VoxelRenderer {
    pub fn update_camera(self, cam: CameraData) {
        self.fov = cam.fov
    }
}

struct UnifiedRenderer {
    last_camera: Option<CameraData>,
    mesh_renderer: MeshRenderer,
    voxel_renderer: VoxelRenderer
}

impl UnifiedRenderer {
    pub fn render_frame(self) {
        if let Some(cam) = self.last_camera {
            self.voxel_renderer.update_camera(cam)
            self.mesh_renderer.set_camera(cam)
        }
    }
}
"#;

    let wj_file = test_dir.join("option_ref_owned.wj");
    fs::write(&wj_file, wj_source).unwrap();

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(test_dir.join("out"))
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(test_dir.join("out").join("option_ref_owned.rs")).unwrap();

    // Key assertion: the generated code must compile with rustc.
    // The binding from `if let Some(cam) = &mut self.last_camera` is &mut CameraData,
    // but set_camera/update_camera expect owned CameraData.
    // The compiler must add .clone() to pass &mut T where T is expected.

    // Check that the code doesn't pass a bare `cam` to set_camera without cloning,
    // when cam is behind a &mut scrutinee.
    // Either: 1) Don't use &mut on scrutinee (use .clone() on the Option)
    //     or: 2) Clone cam when passing as owned argument.
    let has_type_error = generated.contains("set_camera(cam)")
        && generated.contains("&mut self.last_camera");

    if has_type_error {
        // If scrutinee is &mut and cam is passed without clone, this is a bug
        panic!(
            "Bug: Generated code passes &mut binding as owned without .clone().\n\
             The compiler should either:\n\
             1. Not add &mut to scrutinee when binding is passed as owned\n\
             2. Add .clone() to the argument\n\
             Generated code:\n{}",
            generated
        );
    }

    // Verify the code would compile by checking one of:
    // - cam.clone() is used for the argument to set_camera
    // - self.last_camera.clone() is used (no &mut on scrutinee)
    // - some other valid pattern
    let valid = generated.contains("set_camera(cam.clone())")
        || generated.contains("self.last_camera.clone()")
        || (!generated.contains("&mut self.last_camera")
            && generated.contains("set_camera(cam)"));

    assert!(
        valid,
        "Generated code doesn't handle Option binding to owned parameter correctly.\n\
         Generated code:\n{}",
        generated
    );
}
