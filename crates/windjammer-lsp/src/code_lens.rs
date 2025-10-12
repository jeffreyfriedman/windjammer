// Code lens provider for Windjammer LSP
// Shows inferred types, optimization hints, and other contextual information

use crate::analyzer::AnalyzedProgram;
use crate::parser::{Expression, Function, Item, Statement};
use tower_lsp::lsp_types::*;

pub struct CodeLensProvider {
    program: Option<AnalyzedProgram>,
}

impl CodeLensProvider {
    pub fn new() -> Self {
        CodeLensProvider { program: None }
    }

    pub fn update_program(&mut self, program: AnalyzedProgram) {
        self.program = Some(program);
    }

    pub fn get_code_lenses(&self, uri: &Url) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        if let Some(program) = &self.program {
            // Generate code lenses for functions
            for item in &program.items {
                match item {
                    Item::Function(func) => {
                        lenses.extend(self.generate_function_lenses(func));
                    }
                    Item::Struct(s) => {
                        lenses.extend(self.generate_struct_lenses(s));
                    }
                    Item::Static(name, ty, _) => {
                        lenses.extend(self.generate_static_lenses(name, ty));
                    }
                    _ => {}
                }
            }

            // Generate optimization hint lenses
            if let Some(func_analysis) = program.analyzed_functions.first() {
                lenses.extend(self.generate_optimization_lenses(func_analysis));
            }
        }

        lenses
    }

    fn generate_function_lenses(&self, func: &Function) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        // Show inferred parameter types
        for (i, param) in func.parameters.iter().enumerate() {
            let inferred_mode = self.infer_parameter_mode(param);

            if let Some(mode) = inferred_mode {
                lenses.push(CodeLens {
                    range: Range {
                        start: Position {
                            line: param.line as u32,
                            character: param.column as u32,
                        },
                        end: Position {
                            line: param.line as u32,
                            character: (param.column + param.name.len()) as u32,
                        },
                    },
                    command: Some(Command {
                        title: format!("💡 Inferred: {}", mode),
                        command: "windjammer.showInferredType".to_string(),
                        arguments: None,
                    }),
                    data: None,
                });
            }
        }

        // Show function complexity
        let complexity = self.calculate_complexity(func);
        if complexity > 10 {
            lenses.push(CodeLens {
                range: Range {
                    start: Position {
                        line: func.line as u32,
                        character: 0,
                    },
                    end: Position {
                        line: func.line as u32,
                        character: 10,
                    },
                },
                command: Some(Command {
                    title: format!("⚠️ Complexity: {} (consider refactoring)", complexity),
                    command: "windjammer.showComplexity".to_string(),
                    arguments: None,
                }),
                data: None,
            });
        }

        lenses
    }

    fn generate_struct_lenses(&self, s: &Struct) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        // Show struct size hint
        let estimated_size = self.estimate_struct_size(s);

        lenses.push(CodeLens {
            range: Range {
                start: Position {
                    line: s.line as u32,
                    character: 0,
                },
                end: Position {
                    line: s.line as u32,
                    character: 10,
                },
            },
            command: Some(Command {
                title: format!("📏 Estimated size: {} bytes", estimated_size),
                command: "windjammer.showStructSize".to_string(),
                arguments: None,
            }),
            data: None,
        });

        lenses
    }

    fn generate_static_lenses(&self, name: &str, ty: &Type) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        // Check if this static can be promoted to const
        if self.can_promote_to_const(name) {
            lenses.push(CodeLens {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                command: Some(Command {
                    title: "⚡ Can be promoted to const (Phase 7 optimization)".to_string(),
                    command: "windjammer.promoteToConst".to_string(),
                    arguments: None,
                }),
                data: None,
            });
        }

        lenses
    }

    fn generate_optimization_lenses(&self, analysis: &FunctionAnalysis) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        // Show defer drop optimizations
        if !analysis.defer_drop_optimizations.is_empty() {
            lenses.push(CodeLens {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                command: Some(Command {
                    title: format!(
                        "⚡ {} defer drop optimization(s) applied",
                        analysis.defer_drop_optimizations.len()
                    ),
                    command: "windjammer.showOptimizations".to_string(),
                    arguments: None,
                }),
                data: None,
            });
        }

        // Show SmallVec optimizations
        if !analysis.smallvec_optimizations.is_empty() {
            lenses.push(CodeLens {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                command: Some(Command {
                    title: format!(
                        "📦 {} SmallVec optimization(s) applied (Phase 8)",
                        analysis.smallvec_optimizations.len()
                    ),
                    command: "windjammer.showOptimizations".to_string(),
                    arguments: None,
                }),
                data: None,
            });
        }

        // Show Cow optimizations
        if !analysis.cow_optimizations.is_empty() {
            lenses.push(CodeLens {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                command: Some(Command {
                    title: format!(
                        "🐄 {} Cow optimization(s) applied (Phase 9)",
                        analysis.cow_optimizations.len()
                    ),
                    command: "windjammer.showOptimizations".to_string(),
                    arguments: None,
                }),
                data: None,
            });
        }

        lenses
    }

    // Helper methods

    fn infer_parameter_mode(&self, param: &Parameter) -> Option<String> {
        // Simplified inference - in real implementation, use full analysis
        if param.name.starts_with("mut_") {
            Some("&mut".to_string())
        } else if param.ty.is_simple() {
            Some("Copy".to_string())
        } else {
            Some("&".to_string())
        }
    }

    fn calculate_complexity(&self, func: &Function) -> u32 {
        let mut complexity = 1;

        // Count control flow statements
        for stmt in &func.body {
            complexity += match stmt {
                Statement::If { .. } => 1,
                Statement::Match { .. } => 1,
                Statement::While { .. } => 1,
                Statement::For { .. } => 1,
                _ => 0,
            };
        }

        complexity
    }

    fn estimate_struct_size(&self, s: &Struct) -> usize {
        let mut size = 0;

        for field in &s.fields {
            size += match field.ty.name.as_str() {
                "int" | "i32" => 4,
                "i64" => 8,
                "float" | "f32" => 4,
                "f64" => 8,
                "bool" => 1,
                "string" => 24, // String is 3 pointers
                _ => 8,         // Pointer size
            };
        }

        size
    }

    fn can_promote_to_const(&self, name: &str) -> bool {
        // Check if static value is compile-time evaluable
        // Simplified check - real implementation would analyze the initializer
        true
    }
}

// Integration with LSP server
impl CodeLensProvider {
    pub fn provide_code_lens(&self, params: CodeLensParams) -> Vec<CodeLens> {
        self.get_code_lenses(&params.text_document.uri)
    }

    pub fn resolve_code_lens(&self, lens: CodeLens) -> CodeLens {
        // Could add more detailed information on resolve
        lens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_parameter_mode() {
        let provider = CodeLensProvider::new();

        // Test different parameter types
        let param_copy = Parameter {
            name: "count".to_string(),
            ty: Type {
                name: "int".to_string(),
            },
            line: 0,
            column: 0,
        };

        assert_eq!(
            provider.infer_parameter_mode(&param_copy),
            Some("Copy".to_string())
        );
    }

    #[test]
    fn test_calculate_complexity() {
        let provider = CodeLensProvider::new();

        let func = Function {
            name: "test".to_string(),
            parameters: vec![],
            return_type: None,
            body: vec![Statement::If { /* ... */ }, Statement::Match { /* ... */ }],
            line: 0,
        };

        assert_eq!(provider.calculate_complexity(&func), 3); // 1 base + 2 control flow
    }
}
