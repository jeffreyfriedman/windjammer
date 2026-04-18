use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let mut generator = codegen::CodeGenerator::new(signatures, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_vec3_variable_plus_vec3_no_cast() {
    let source = r#"
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    impl Vec3 {
        fn new(x: f32, y: f32, z: f32) -> Vec3 {
            Vec3 { x: x, y: y, z: z }
        }
    }

    struct Scene {
        min: Vec3,
        max: Vec3,
    }

    impl Scene {
        fn calc_camera(self) -> Vec3 {
            let center = (self.min + self.max) * 0.5
            let size = 10.0
            let dist = size * 1.5
            center + Vec3::new(dist, dist * 0.5, dist)
        }
    }
    "#;
    let output = compile_and_get_rust(source);
    eprintln!("Generated:\n{}", output);

    assert!(
        !output.contains("center as f32"),
        "Vec3 variable should NOT get spurious 'as f32' cast. Got:\n{}",
        output
    );
}

#[test]
fn test_vec3_param_minus_vec3_no_cast() {
    let source = r#"
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    struct Cluster {
        bounds_min: Vec3,
        bounds_max: Vec3,
    }

    fn select_lod(cluster: Cluster, camera_pos: Vec3, screen_height: f32) -> i32 {
        let center = (cluster.bounds_min + cluster.bounds_max) * 0.5
        let distance = (center - camera_pos).length()
        0
    }
    "#;
    let output = compile_and_get_rust(source);
    eprintln!("Generated:\n{}", output);

    assert!(
        !output.contains("camera_pos as f32"),
        "Vec3 parameter should NOT get spurious 'as f32' cast. Got:\n{}",
        output
    );
}
