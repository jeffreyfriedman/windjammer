#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_struct_with_mixed_numeric_fields_generates_to_bytes() {
    let source = r#"
        struct DenoiseParams {
            alpha: f32,
            sigma_color: f32,
            sigma_normal: f32,
            frame_count: u32,
        }
    "#;
    let output = test_utils::compile_single(source);

    // Should generate a to_bytes() method
    assert!(
        output.contains("fn to_bytes"),
        "Should generate to_bytes() method. Got:\n{}",
        output
    );
    // Should use to_ne_bytes() for each field
    assert!(
        output.contains("self.alpha.to_ne_bytes()"),
        "Should serialize alpha with to_ne_bytes(). Got:\n{}",
        output
    );
    assert!(
        output.contains("self.frame_count.to_ne_bytes()"),
        "Should serialize frame_count with to_ne_bytes(). Got:\n{}",
        output
    );
}

#[test]
fn test_all_f32_struct_generates_to_bytes() {
    let source = r#"
        struct LightConfig {
            intensity: f32,
            radius: f32,
            falloff: f32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("fn to_bytes"),
        "All-f32 struct should generate to_bytes(). Got:\n{}",
        output
    );
}

#[test]
fn test_all_u32_struct_generates_to_bytes() {
    let source = r#"
        struct ScreenSize {
            width: u32,
            height: u32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("fn to_bytes"),
        "All-u32 struct should generate to_bytes(). Got:\n{}",
        output
    );
}

#[test]
fn test_struct_with_string_does_not_generate_to_bytes() {
    let source = r#"
        struct UserConfig {
            name: String,
            age: u32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        !output.contains("fn to_bytes"),
        "Struct with String field should NOT generate to_bytes(). Got:\n{}",
        output
    );
}

#[test]
fn test_struct_with_vec_does_not_generate_to_bytes() {
    let source = r#"
        struct ItemList {
            count: u32,
            items: Vec<f32>,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        !output.contains("fn to_bytes"),
        "Struct with Vec field should NOT generate to_bytes(). Got:\n{}",
        output
    );
}

#[test]
fn test_struct_with_bool_generates_to_bytes() {
    let source = r#"
        struct Flags {
            enabled: bool,
            count: u32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("fn to_bytes"),
        "Struct with bool+u32 should generate to_bytes(). Got:\n{}",
        output
    );
    // Bool should be serialized as u32 (GPU bools are 4 bytes)
    assert!(
        output.contains("if self.enabled { 1u32 } else { 0u32 }"),
        "Bool fields should serialize as u32 for GPU compatibility. Got:\n{}",
        output
    );
}

#[test]
fn test_struct_with_i32_generates_to_bytes() {
    let source = r#"
        struct Offset {
            x: i32,
            y: i32,
            scale: f32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("fn to_bytes"),
        "Struct with i32+f32 should generate to_bytes(). Got:\n{}",
        output
    );
    assert!(
        output.contains("self.x.to_ne_bytes()"),
        "i32 fields should use to_ne_bytes(). Got:\n{}",
        output
    );
}

#[test]
fn test_struct_with_fixed_array_generates_to_bytes() {
    let source = r#"
        struct Transform {
            matrix: [f32; 16],
            position: f32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("fn to_bytes"),
        "Struct with [f32; N] array should generate to_bytes(). Got:\n{}",
        output
    );
    // Array elements should each be serialized
    assert!(
        output.contains("for __el in &self.matrix"),
        "Array fields should iterate elements. Got:\n{}",
        output
    );
}

#[test]
fn test_to_bytes_returns_vec_u8() {
    let source = r#"
        struct Params {
            value: f32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("-> Vec<u8>"),
        "to_bytes() should return Vec<u8>. Got:\n{}",
        output
    );
}

#[test]
fn test_empty_struct_does_not_generate_to_bytes() {
    let source = r#"
        struct Empty {}
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        !output.contains("fn to_bytes"),
        "Empty struct should not generate to_bytes(). Got:\n{}",
        output
    );
}

#[test]
fn test_to_bytes_with_extend_from_slice() {
    let source = r#"
        struct Pair {
            x: f32,
            y: u32,
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("extend_from_slice"),
        "to_bytes() should use extend_from_slice for field serialization. Got:\n{}",
        output
    );
}

#[test]
fn test_generated_to_bytes_compiles_with_rustc() {
    let source = r#"
        struct GpuParams {
            alpha: f32,
            sigma: f32,
            count: u32,
            offset: i32,
        }

        fn main() {
            let p = GpuParams { alpha: 0.5, sigma: 1.0, count: 42, offset: -3 };
            let bytes = p.to_bytes();
            assert_eq!(bytes.len(), 16);
        }
    "#;
    let output = test_utils::compile_single(source);

    let _tmp = tempfile::tempdir().unwrap();

    let test_dir = _tmp
        .path()
        .join(format!("wj_to_bytes_rustc_{}", std::process::id()));

    std::fs::create_dir_all(&test_dir).unwrap();
    std::fs::write(test_dir.join("main.rs"), &output).unwrap();

    let rustc = std::process::Command::new("rustc")
        .arg(test_dir.join("main.rs"))
        .arg("-o")
        .arg(test_dir.join("main"))
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&rustc.stderr);
    assert!(
        rustc.status.success(),
        "Generated code should compile with rustc. Errors:\n{}\n\nGenerated code:\n{}",
        stderr,
        output
    );

    // Run the binary to verify runtime correctness
    let run = std::process::Command::new(test_dir.join("main"))
        .output()
        .expect("Failed to run compiled binary");
    assert!(
        run.status.success(),
        "Generated to_bytes() should produce correct output at runtime"
    );
}

#[test]
fn test_generated_to_bytes_runtime_bit_correctness() {
    let source = r#"
        struct MixedTypes {
            float_val: f32,
            uint_val: u32,
        }

        fn main() {
            let m = MixedTypes { float_val: 1.0, uint_val: 1 };
            let bytes = m.to_bytes();

            // f32(1.0) = 0x3F800000 = [0, 0, 128, 63] in little-endian
            // u32(1)   = 0x00000001 = [1, 0,   0,  0] in little-endian
            // These MUST be different! If they're the same, type conversion happened.
            let f32_bytes = &bytes[0..4];
            let u32_bytes = &bytes[4..8];
            assert!(f32_bytes != u32_bytes,
                "f32(1.0) and u32(1) must have different byte representations!");
        }
    "#;
    let output = test_utils::compile_single(source);

    let _tmp2 = tempfile::tempdir().unwrap();

    let test_dir = _tmp2
        .path()
        .join(format!("wj_to_bytes_bits_{}", std::process::id()));

    std::fs::create_dir_all(&test_dir).unwrap();
    std::fs::write(test_dir.join("main.rs"), &output).unwrap();

    let rustc = std::process::Command::new("rustc")
        .arg(test_dir.join("main.rs"))
        .arg("-o")
        .arg(test_dir.join("main"))
        .output()
        .expect("Failed to run rustc");
    assert!(
        rustc.status.success(),
        "Bit correctness test should compile. Errors:\n{}",
        String::from_utf8_lossy(&rustc.stderr)
    );

    let run = std::process::Command::new(test_dir.join("main"))
        .output()
        .expect("Failed to run");
    assert!(
        run.status.success(),
        "f32 and u32 must produce different byte patterns (type-safe serialization)"
    );
}

#[test]
fn test_struct_with_user_impl_still_gets_to_bytes() {
    let source = r#"
        struct Params {
            value: f32,
            count: u32,
        }

        impl Params {
            fn new(v: f32, c: u32) -> Params {
                Params { value: v, count: c }
            }
        }
    "#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("fn to_bytes"),
        "Struct with user impl should still get to_bytes(). Got:\n{}",
        output
    );
    assert!(
        output.contains("fn new"),
        "User impl should be preserved. Got:\n{}",
        output
    );
}
