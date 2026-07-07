#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

#[path = "common/wjsl_shader_fixtures.rs"]
mod wjsl_shader_fixtures;

#[test]
fn test_lighting_shader_transpile_output() {
    let shader_path = wjsl_shader_fixtures::fixture_shader_path("voxel_lighting.wjsl");
    let source = std::fs::read_to_string(&shader_path).unwrap();
    let base_dir = wjsl_shader_fixtures::shader_fixtures_dir();

    let wgsl = windjammer::wjsl::transpile_wjsl_with_includes(&source, &base_dir)
        .expect("WJSL transpilation should succeed");

    // Print the main function and nearby lines
    let lines: Vec<&str> = wgsl.lines().collect();
    let mut in_main = false;
    let mut brace_depth = 0i32;
    eprintln!("=== TRANSPILED WGSL: main function ===");
    for line in &lines {
        if line.contains("fn main(") {
            in_main = true;
        }
        if in_main {
            eprintln!("{}", line);
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
            if brace_depth <= 0 && in_main {
                break;
            }
        }
    }
    eprintln!("=== END ===");

    // Print GBufferPixel struct
    eprintln!("=== GBufferPixel struct ===");
    let mut in_struct = false;
    for line in &lines {
        if line.contains("struct GBufferPixel") {
            in_struct = true;
        }
        if in_struct {
            eprintln!("{}", line);
            if line.contains('}') {
                break;
            }
        }
    }
    eprintln!("=== END ===");

    // Check GBufferPixel uses vec4 for normal (not vec3)
    assert!(
        wgsl.contains("normal: vec4<f32>"),
        "GBufferPixel.normal should be vec4<f32>, not vec3. Found:\n{}",
        wgsl.lines()
            .filter(|l| l.contains("normal"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Check that gbuf.normal.w access is preserved
    assert!(
        wgsl.contains("normal.w") || wgsl.contains("material_id"),
        "Material ID access via normal.w should be preserved in WGSL"
    );

    // Check screen_size binding
    assert!(
        wgsl.contains("screen_size"),
        "screen_size uniform binding should exist"
    );

    // Check LightingParams struct exists
    assert!(
        wgsl.contains("LightingParams"),
        "LightingParams struct should be present"
    );
}

#[test]
fn test_raymarch_shader_transpile_output() {
    let shader_path = wjsl_shader_fixtures::fixture_shader_path("voxel_raymarch.wjsl");
    let source = std::fs::read_to_string(&shader_path).unwrap();
    let base_dir = wjsl_shader_fixtures::shader_fixtures_dir();

    let wgsl = windjammer::wjsl::transpile_wjsl_with_includes(&source, &base_dir)
        .expect("WJSL transpilation should succeed");

    eprintln!("=== RAYMARCH TRANSPILED WGSL (GBufferPixel section) ===");
    for line in wgsl.lines() {
        if line.contains("GBufferPixel")
            || line.contains("position")
            || line.contains("normal")
            || line.contains("depth")
            || line.contains("geometry")
            || line.contains("_pad")
        {
            eprintln!("{}", line);
        }
    }
    eprintln!("=== END ===");

    // Both shaders should have the same GBufferPixel definition
    assert!(
        wgsl.contains("normal: vec4<f32>"),
        "Raymarch GBufferPixel.normal should also be vec4<f32>"
    );
}
