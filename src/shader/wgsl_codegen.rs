//! WGSL code generation from .wjsl ShaderModule

use crate::shader::ast::{AccessMode, ScalarType, ShaderModule, Type};

fn type_to_wgsl(ty: &Type) -> String {
    match ty {
        Type::Scalar(s) => match s {
            ScalarType::F32 => "f32".to_string(),
            ScalarType::F64 => "f64".to_string(),
            ScalarType::U32 => "u32".to_string(),
            ScalarType::I32 => "i32".to_string(),
            ScalarType::Bool => "bool".to_string(),
        },
        Type::Vec2(s) => format!("vec2<{}>", type_to_wgsl(&Type::Scalar(*s))),
        Type::Vec3(s) => format!("vec3<{}>", type_to_wgsl(&Type::Scalar(*s))),
        Type::Vec4(s) => format!("vec4<{}>", type_to_wgsl(&Type::Scalar(*s))),
        Type::Mat4 => "mat4x4<f32>".to_string(),
        Type::Array(inner) => format!("array<{}>", type_to_wgsl(inner)),
        Type::Struct(name) => name.clone(),
    }
}

fn access_to_wgsl(access: AccessMode) -> &'static str {
    match access {
        AccessMode::Read => "read",
        AccessMode::Write => "write",
        AccessMode::ReadWrite => "read_write",
    }
}

/// Generate WGSL source code from a ShaderModule
pub fn generate_wgsl(shader: &ShaderModule) -> String {
    let mut output = String::new();

    // Generate uniform struct if we have uniforms
    if !shader.uniforms.is_empty() {
        output.push_str("struct Uniforms {\n");
        for uniform in &shader.uniforms {
            output.push_str(&format!(
                "    {}: {},\n",
                uniform.name,
                type_to_wgsl(&uniform.ty)
            ));
        }
        output.push_str("}\n\n");
    }

    // Generate bindings - uniforms in a single struct
    let mut binding = 0u32;
    if !shader.uniforms.is_empty() {
        output.push_str(&format!(
            "@group(0) @binding({}) var<uniform> uniforms: Uniforms;\n",
            binding
        ));
        binding += 1;
    }

    for storage in &shader.storage {
        output.push_str(&format!(
            "@group(0) @binding({}) var<storage, {}> {}: {};\n",
            binding,
            access_to_wgsl(storage.access),
            storage.name,
            type_to_wgsl(&storage.ty)
        ));
        binding += 1;
    }

    if !shader.uniforms.is_empty() || !shader.storage.is_empty() {
        output.push('\n');
    }

    // Generate compute shader entry point
    output.push_str("@compute @workgroup_size(8, 8)\n");
    output.push_str("fn main(@builtin(global_invocation_id) id: vec3<u32>) {\n");
    let has_screen_size = shader
        .uniforms
        .iter()
        .any(|u| u.name == "screen_size");
    if has_screen_size {
        output.push_str("    let width = u32(uniforms.screen_size.x);\n");
        output.push_str("    let height = u32(uniforms.screen_size.y);\n");
        output.push_str("    if (id.x >= width || id.y >= height) {\n");
        output.push_str("        return;\n");
        output.push_str("    }\n");
    }
    output.push_str("    // TODO: Add shader body from .wjsl functions\n");
    output.push_str("}\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader::parse_shader;

    #[test]
    fn test_generate_wgsl_uniform_struct() {
        let source = "shader S { uniform screen_size: Vec2<f32> }";
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("struct Uniforms"));
        assert!(wgsl.contains("screen_size: vec2<f32>"));
        assert!(wgsl.contains("var<uniform> uniforms: Uniforms"));
    }

    #[test]
    fn test_generate_wgsl_storage_binding() {
        let source = "shader S { storage output: array<Vec4<f32>> }";
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("var<storage, read_write> output"));
        assert!(wgsl.contains("array<vec4<f32>>"));
    }

    #[test]
    fn test_generate_wgsl_compute_entry() {
        let source = "shader S { uniform x: Vec2<f32> }";
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("@compute @workgroup_size(8, 8)"));
        assert!(wgsl.contains("fn main"));
        assert!(wgsl.contains("global_invocation_id"));
    }

    #[test]
    fn test_generate_wgsl_vec_types() {
        let source = r#"
        shader S {
            uniform a: Vec2<f32>
            uniform b: Vec3<f32>
            uniform c: Vec4<f32>
        }
        "#;
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("vec2<f32>"));
        assert!(wgsl.contains("vec3<f32>"));
        assert!(wgsl.contains("vec4<f32>"));
    }

    #[test]
    fn test_generate_wgsl_mat4() {
        let source = "shader S { uniform m: Mat4 }";
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("mat4x4<f32>"));
    }

    #[test]
    fn test_generate_wgsl_multiple_bindings() {
        let source = r#"
        shader S {
            uniform screen_size: Vec2<f32>
            storage output: array<Vec4<f32>>
        }
        "#;
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("@binding(0)"));
        assert!(wgsl.contains("@binding(1)"));
    }

    #[test]
    fn test_generate_wgsl_u32_types() {
        let source = "shader S { uniform count: Vec2<u32> }";
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("vec2<u32>"));
    }

    #[test]
    fn test_generate_wgsl_empty_shader() {
        let source = "shader Empty { }";
        let module = parse_shader(source).unwrap();
        let wgsl = generate_wgsl(&module);

        assert!(wgsl.contains("fn main"));
        assert!(wgsl.contains("@compute"));
    }

    #[test]
    fn test_type_to_wgsl_scalars() {
        assert_eq!(type_to_wgsl(&Type::Scalar(ScalarType::F32)), "f32");
        assert_eq!(type_to_wgsl(&Type::Scalar(ScalarType::U32)), "u32");
    }

    #[test]
    fn test_type_to_wgsl_vectors() {
        assert_eq!(
            type_to_wgsl(&Type::Vec4(ScalarType::F32)),
            "vec4<f32>"
        );
    }
}
