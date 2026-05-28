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

//! Codegen emits SIMD (`std::arch`) for simple f32 dot loops on native Rust targets.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate_rust(code: &str, filename: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.set_source_file(filename);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn rust_backend_emits_simd_intrinsics_for_f32_dot_loop() {
    let input = r#"
pub fn wj_dot(a: Vec<f32>, b: Vec<f32>) -> f32 {
    let mut sum: f32 = 0.0
    for i in 0..a.len() {
        sum += a[i] * b[i]
    }
    sum
}
"#;

    let generated = parse_and_generate_rust(input, "simd_dot_test.wj");

    #[cfg(target_arch = "x86_64")]
    {
        assert!(
            generated.contains("is_x86_feature_detected!(\"avx\")"),
            "expected runtime AVX dispatch in SIMD emission"
        );
        assert!(
            generated.contains("_mm256_loadu_ps") && generated.contains("_mm_loadu_ps"),
            "expected AVX256 and SSE SIMD paths on x86_64, got excerpt:\n{}",
            &generated[..generated.len().min(3500)]
        );
    }
    #[cfg(target_arch = "aarch64")]
    assert!(
        generated.contains("vld1q_f32"),
        "expected NEON loads in SIMD dot emission"
    );

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    assert!(
        generated.contains("__wj_len"),
        "portable fallback should clamp iterated length (__wj_len)"
    );
}

#[test]
fn rust_backend_emits_simd_intrinsics_for_f32_array_add_loop() {
    let input = r#"
pub fn wj_vadd(out: Vec<f32>, a: Vec<f32>, b: Vec<f32>) -> () {
    for i in 0..out.len() {
        out[i] = a[i] + b[i]
    }
}
"#;

    let generated = parse_and_generate_rust(input, "simd_vadd_test.wj");

    #[cfg(target_arch = "x86_64")]
    assert!(
        generated.contains("_mm_storeu_ps") || generated.contains("_mm256_storeu_ps"),
        "expected SIMD stores for element-wise add"
    );

    #[cfg(target_arch = "aarch64")]
    assert!(
        generated.contains("vst1q_f32"),
        "expected NEON stores in SIMD element-wise add"
    );

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    assert!(
        generated.contains("__wj_len"),
        "portable fallback should clamp length"
    );
}
