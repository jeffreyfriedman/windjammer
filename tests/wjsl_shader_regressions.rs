/// TDD: Test real shader files that are failing transpilation
///
/// Bug: Multiple shaders fail with WJSL syntax/type errors:
/// - voxel_raymarch.wjsl: "Unknown identifier 'voxel'"
/// - voxel_lighting.wjsl: "Expected semicolon, found FloatLiteral(25.0)"
/// - point_light/area_light.wjsl: "Invalid operands for *: mat4x4 and mat4x4"
#[cfg(any(
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
fn transpile(source: &str) -> Result<String, String> {
    windjammer::wjsl::transpile_wjsl(source).map_err(|e| e.to_string())
}

#[cfg(any(
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
mod shader_tests {
    use super::transpile;

    #[test]
    fn test_voxel_raymarch_let_mut_pattern() {
        let source = r#"
@fragment
fn main() {
    let pos = vec3(0.0);
    let inv_vs = 1.0;
    let vs = 1.0;
    
    let mut voxel = floor(pos * inv_vs) * vs;
    voxel = voxel + vec3(1.0);
}
"#;
        let result = transpile(source);
        assert!(result.is_ok(), 
            "voxel_raymarch pattern should transpile: {:?}", result.err());
        let wgsl = result.unwrap();
        assert!(wgsl.contains("var voxel"), 
            "Should convert 'let mut voxel' to 'var voxel': {}", wgsl);
    }

    #[test]
    fn test_voxel_lighting_semicolon_issue() {
        let source = r#"
@fragment
fn main() {
    let radius = 25.0;
    let x = radius * 2.0;
}
"#;
        let result = transpile(source);
        assert!(result.is_ok(), 
            "simple float literal assignment should transpile: {:?}", result.err());
    }

    #[test]
    fn test_mat4_multiply() {
        let source = r#"
@fragment
fn main() {
    let view = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
    let proj = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    );
    let vp = proj * view;
}
"#;
        let result = transpile(source);
        assert!(result.is_ok(), 
            "mat4x4 * mat4x4 should transpile: {:?}", result.err());
    }

    #[test]
    fn test_ssgi_rparen_issue() {
        let source = r#"
@fragment
fn main() {
    let x = some_func();
    let y = other_func(1.0, 2.0);
}

fn some_func() -> f32 {
    return 1.0;
}

fn other_func(a: f32, b: f32) -> f32 {
    return a + b;
}
"#;
        let result = transpile(source);
        assert!(result.is_ok(), 
            "function calls with empty/normal args should transpile: {:?}", result.err());
    }

    #[test]
    fn test_break_in_for_loop() {
        let source = r#"
@compute @workgroup_size(1, 1, 1)
fn main() {
    for (var i = 0; i < 10; i = i + 1) {
        if (i > 5) {
            break;
        }
    }
}
"#;
        let result = transpile(source);
        assert!(result.is_ok(), "break statement should transpile: {:?}", result.err());
        assert!(result.unwrap().contains("break"), "WGSL should contain break");
    }

    #[test]
    fn test_vec4_constructor_trailing_comma() {
        let source = r#"
@fragment
fn main() {
    let v = vec4<u32>(
        0u,
        1u,
        2u,
        3u,
    );
    let _ = v.x;
}
"#;
        let result = transpile(source);
        assert!(
            result.is_ok(),
            "vec4 constructor with trailing comma should transpile: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_array_constructor_expression() {
        let source = r#"
@fragment
fn main() {
    let values = array<i32, 3>(1, 2, 3);
    let _ = values[0];
}
"#;
        let result = transpile(source);
        assert!(
            result.is_ok(),
            "array<T, N>(...) constructor should transpile: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_workgroup_size_single_dimension() {
        let source = r#"
@compute @workgroup_size(64)
fn main() {
}
"#;
        let result = transpile(source);
        assert!(
            result.is_ok(),
            "@workgroup_size(64) should default y/z to 1: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_var_storage_comma_access() {
        let source = r#"
struct SourceVertex {
    px: f32,
}

@group(0) @binding(0) var<storage, read> source_vertices: array<SourceVertex>;
@group(0) @binding(1) var<storage, read_write> output_vertices: array<SourceVertex>;

@compute @workgroup_size(64, 1, 1)
fn main() {
    output_vertices[0] = source_vertices[0];
}
"#;
        let result = transpile(source);
        assert!(
            result.is_ok(),
            "var<storage, read> bindings should transpile: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_discard_statement() {
        let source = r#"
@fragment
fn fs_main() -> vec4<f32> {
    if (true) {
        discard;
    }
    return vec4(1.0);
}
"#;
        let result = transpile(source);
        assert!(result.is_ok(), "discard statement should transpile: {:?}", result.err());
        assert!(result.unwrap().contains("discard"));
    }

    #[test]
    fn test_struct_fields_with_location_and_builtin() {
        let source = r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4(in.position, 1.0);
    out.world_position = in.position;
    return out;
}
"#;
        let result = transpile(source);
        assert!(
            result.is_ok(),
            "struct @location/@builtin fields should transpile: {:?}",
            result.err()
        );
        let wgsl = result.unwrap();
        assert!(wgsl.contains("@location(0)"));
        assert!(wgsl.contains("@builtin(position)"));
    }

    #[test]
    fn test_var_uniform_and_texture_bindings() {
        let source = r#"
struct MaterialUniforms {
    albedo: vec4<f32>,
}

@group(0) @binding(0) var<uniform> material: MaterialUniforms;
@group(0) @binding(1) var albedo_texture: texture_2d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;

@fragment
fn fs_main() -> vec4<f32> {
    return vec4(1.0);
}
"#;
        let result = transpile(source);
        assert!(
            result.is_ok(),
            "var<uniform> and var texture/sampler bindings should transpile: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_user_function_call_statement() {
        let source = r#"
@group(0) @binding(0) uniform camera: mat4x4;

var<private> planes: array<vec4, 6>;

@compute @workgroup_size(64, 1, 1)
fn visibility_pass() {
    extract_frustum_planes();
    let _ = frustum_intersects_aabb(vec3(0.0), vec3(1.0));
}

fn extract_frustum_planes() -> bool {
    planes[0] = vec4(1.0);
    return true;
}

fn frustum_intersects_aabb(bounds_min: vec3, bounds_max: vec3) -> bool {
    return true;
}
"#;
        let result = transpile(source);
        assert!(
            result.is_ok(),
            "void user function call statement should transpile: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_continue_in_for_loop() {
        let source = r#"
@compute @workgroup_size(1, 1, 1)
fn main() {
    for (var i = 0; i < 10; i = i + 1) {
        if (i < 3) {
            continue;
        }
    }
}
"#;
        let result = transpile(source);
        assert!(result.is_ok(), "continue statement should transpile: {:?}", result.err());
    }
}
