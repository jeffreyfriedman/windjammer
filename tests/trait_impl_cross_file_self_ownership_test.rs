use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

/// Simulate single-file compilation where the trait definition is NOT available.
/// This is the real-world scenario: `voxel_gpu_renderer.wj` is compiled separately
/// from `render_port.wj`, so the analyzer doesn't have the trait's ownership info.
fn compile_impl_only(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_cross_file_trait_impl_readonly_method_uses_borrowed_self() {
    // Bug: When a trait impl method doesn't mutate self and the trait is defined
    // in another file (not in analyzed_trait_methods), the compiler defaults to
    // &mut self. It should use &self based on body analysis.
    //
    // Real-world case: VoxelGPURenderer::get_output_buffer() -> Vec<u8>
    // returns Vec::new() without touching self, but generated &mut self.
    let code = r#"
use crate::rendering::render_port::RenderPort

pub struct VoxelRenderer {
    buffer_id: u32,
}

impl RenderPort for VoxelRenderer {
    fn get_output() -> Vec<u8> {
        Vec::new()
    }
}
"#;

    let output = compile_impl_only(code);
    println!("Generated:\n{}", output);

    assert!(
        output.contains("fn get_output(&self)"),
        "Cross-file trait impl with read-only body should generate &self, not &mut self.\nGenerated:\n{}",
        output
    );
    assert!(
        !output.contains("fn get_output(&mut self)"),
        "Should NOT generate &mut self for a method that doesn't mutate self.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_cross_file_trait_impl_mutating_method_uses_mut_self() {
    // When the impl method DOES mutate self, it should correctly generate &mut self
    // even when the trait is cross-file.
    let code = r#"
use crate::rendering::render_port::RenderPort

pub struct VoxelRenderer {
    frame_count: u32,
}

impl RenderPort for VoxelRenderer {
    fn render_frame() {
        self.frame_count = self.frame_count + 1
    }
}
"#;

    let output = compile_impl_only(code);
    println!("Generated:\n{}", output);

    assert!(
        output.contains("fn render_frame(&mut self)"),
        "Cross-file trait impl with mutation should generate &mut self.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_cross_file_trait_impl_void_method_defaults_to_mut_self() {
    // Void methods (no return type) should default to &mut self in cross-file
    // trait impls, matching the trait declaration heuristic.
    // The trait declaration defaults to &mut self for methods without a return type.
    let code = r#"
use crate::game::Port

pub struct NullRenderer {
    id: u32,
}

impl Port for NullRenderer {
    fn initialize() {
    }

    fn shutdown() {
    }
}
"#;

    let output = compile_impl_only(code);
    println!("Generated:\n{}", output);

    // Void methods with empty bodies default to &mut self
    // (matching trait declaration heuristic: no return type → &mut self)
    assert!(
        output.contains("fn initialize(&mut self)"),
        "Void cross-file trait impl method should default to &mut self.\nGenerated:\n{}",
        output
    );
    assert!(
        output.contains("fn shutdown(&mut self)"),
        "Void cross-file trait impl method should default to &mut self.\nGenerated:\n{}",
        output
    );
}
