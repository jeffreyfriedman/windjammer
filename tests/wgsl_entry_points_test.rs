/// TDD test: WGSL entry points and GPU attributes
///
/// Tests @compute, @vertex, @fragment attributes and workgroup size parsing
use std::fs;
use std::process::Command;

fn transpile_wj_to_wgsl(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_wgsl_entry_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
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

    let wgsl_file = out_dir.join("test.wgsl");
    let content = fs::read_to_string(&wgsl_file).expect("Failed to read generated WGSL file");

    let _ = fs::remove_dir_all(&test_dir);

    content
}

#[test]
fn test_compute_attribute_basic() {
    let source = r#"
@compute(workgroup_size = [8, 8, 1])
pub fn raymarch(id: vec3<uint>) {
    // Compute shader body
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Check for @compute attribute
    assert!(
        generated.contains("@compute"),
        "Should have @compute attribute. Got:\n{}",
        generated
    );
    
    // Check for workgroup size
    assert!(
        generated.contains("@workgroup_size(8, 8, 1)") || generated.contains("@workgroup_size(8,8,1)"),
        "Should have workgroup_size(8, 8, 1). Got:\n{}",
        generated
    );
}

#[test]
fn test_compute_with_1x1x1() {
    let source = r#"
@compute(workgroup_size = [1, 1, 1])
pub fn simple() {
    // Empty compute shader
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("@compute"));
    assert!(
        generated.contains("@workgroup_size(1, 1, 1)") || generated.contains("@workgroup_size(1,1,1)")
    );
}

#[test]
fn test_compute_with_16x16x1() {
    let source = r#"
@compute(workgroup_size = [16, 16, 1])
pub fn large_workgroup() {
    // 256 invocations
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("@compute"));
    assert!(
        generated.contains("@workgroup_size(16, 16, 1)") || generated.contains("@workgroup_size(16,16,1)")
    );
}

#[test]
fn test_builtin_global_invocation_id() {
    let source = r#"
@compute(workgroup_size = [8, 8, 1])
pub fn shader(@builtin(global_invocation_id) id: vec3<uint>) {
    let x = id.x
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Check for builtin attribute
    assert!(
        generated.contains("@builtin(global_invocation_id)"),
        "Should have @builtin(global_invocation_id). Got:\n{}",
        generated
    );
}

#[test]
fn test_builtin_local_invocation_id() {
    let source = r#"
@compute(workgroup_size = [8, 8, 1])
pub fn shader(@builtin(local_invocation_id) local_id: vec3<uint>) {
    let x = local_id.x
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(
        generated.contains("@builtin(local_invocation_id)"),
        "Should have @builtin(local_invocation_id). Got:\n{}",
        generated
    );
}

#[test]
fn test_compute_with_return_type() {
    let source = r#"
@compute(workgroup_size = [8, 8, 1])
pub fn shader(id: vec3<uint>) -> vec4<float> {
    vec4(1.0, 0.0, 0.0, 1.0)
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("@compute"));
    assert!(generated.contains("-> vec4<f32>"));
}

#[test]
fn test_multiple_compute_shaders() {
    let source = r#"
@compute(workgroup_size = [8, 8, 1])
pub fn shader1() {
    // First shader
}

@compute(workgroup_size = [16, 16, 1])
pub fn shader2() {
    // Second shader
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    // Should have both shaders
    assert!(generated.contains("fn shader1"));
    assert!(generated.contains("fn shader2"));
    assert!(generated.contains("@workgroup_size(8, 8, 1)") || generated.contains("@workgroup_size(8,8,1)"));
    assert!(generated.contains("@workgroup_size(16, 16, 1)") || generated.contains("@workgroup_size(16,16,1)"));
}

#[test]
fn test_compute_with_multiple_builtins() {
    let source = r#"
@compute(workgroup_size = [8, 8, 1])
pub fn shader(
    @builtin(global_invocation_id) global_id: vec3<uint>,
    @builtin(local_invocation_id) local_id: vec3<uint>,
    @builtin(workgroup_id) workgroup_id: vec3<uint>
) {
    // Use all IDs
}
"#;

    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);

    assert!(generated.contains("@builtin(global_invocation_id)"));
    assert!(generated.contains("@builtin(local_invocation_id)"));
    assert!(generated.contains("@builtin(workgroup_id)"));
}
