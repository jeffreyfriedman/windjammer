// Semantic highlighting provider for Windjammer LSP
//
// Provides context-aware syntax coloring beyond simple textmate grammars

use tower_lsp::lsp_types::*;
use windjammer::parser::{FunctionDecl, Item, Program, Statement, Type};

/// Provides semantic token information for syntax highlighting
pub struct SemanticTokensProvider {
    program: Option<Program>,
}

impl SemanticTokensProvider {
    pub fn new() -> Self {
        Self { program: None }
    }

    pub fn update_program(&mut self, program: Program) {
        self.program = Some(program);
    }

    /// Generate semantic tokens for the entire document
    pub fn get_semantic_tokens(&self) -> Option<Vec<SemanticToken>> {
        // TODO: Implement proper semantic token generation
        // Currently returns empty to avoid compilation errors
        // Need to:
        // 1. Track line/column positions in AST
        // 2. Map SemanticTokenType to u32 indices
        // 3. Calculate delta encoding properly
        let _program = self.program.as_ref()?;

        // Return empty tokens for now
        Some(Vec::new())
    }

    fn collect_tokens_from_item(&self, item: &Item, tokens: &mut Vec<SemanticToken>) {
        match item {
            Item::Function(func) => self.collect_tokens_from_function(func, tokens),
            Item::Struct(struct_decl) => {
                // Struct name
                tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: struct_decl.name.len() as u32,
                    token_type: SemanticTokenType::STRUCT.into(),
                    token_modifiers_bitset: 0,
                });

                // Struct fields
                for field in &struct_decl.fields {
                    tokens.push(SemanticToken {
                        delta_line: 0,
                        delta_start: 0,
                        length: field.name.len() as u32,
                        token_type: SemanticTokenType::PROPERTY.into(),
                        token_modifiers_bitset: 0,
                    });
                }
            }
            Item::Enum(enum_decl) => {
                // Enum name
                tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: enum_decl.name.len() as u32,
                    token_type: SemanticTokenType::ENUM.into(),
                    token_modifiers_bitset: 0,
                });

                // Enum variants
                for variant in &enum_decl.variants {
                    tokens.push(SemanticToken {
                        delta_line: 0,
                        delta_start: 0,
                        length: variant.name.len() as u32,
                        token_type: SemanticTokenType::ENUM_MEMBER.into(),
                        token_modifiers_bitset: 0,
                    });
                }
            }
            Item::Trait(trait_decl) => {
                // Trait name
                tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: trait_decl.name.len() as u32,
                    token_type: SemanticTokenType::INTERFACE.into(),
                    token_modifiers_bitset: 0,
                });
            }
            Item::Impl(impl_decl) => {
                for method in &impl_decl.methods {
                    if let Item::Function(func) = method {
                        self.collect_tokens_from_function(func, tokens);
                    }
                }
            }
            Item::Const { name, .. } | Item::Static { name, .. } => {
                tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: name.len() as u32,
                    token_type: SemanticTokenType::VARIABLE.into(),
                    token_modifiers_bitset: SemanticTokenModifier::READONLY.bits(),
                });
            }
            _ => {}
        }
    }

    fn collect_tokens_from_function(&self, func: &FunctionDecl, tokens: &mut Vec<SemanticToken>) {
        // Function name
        tokens.push(SemanticToken {
            delta_line: 0,
            delta_start: 0,
            length: func.name.len() as u32,
            token_type: SemanticTokenType::FUNCTION.into(),
            token_modifiers_bitset: 0,
        });

        // Parameters
        for param in &func.parameters {
            tokens.push(SemanticToken {
                delta_line: 0,
                delta_start: 0,
                length: param.name.len() as u32,
                token_type: SemanticTokenType::PARAMETER.into(),
                token_modifiers_bitset: 0,
            });

            self.collect_tokens_from_type(&param.type_, tokens);
        }

        // Return type
        if let Some(ref return_type) = func.return_type {
            self.collect_tokens_from_type(return_type, tokens);
        }

        // Function body
        for stmt in &func.body {
            self.collect_tokens_from_statement(stmt, tokens);
        }
    }

    fn collect_tokens_from_statement(&self, stmt: &Statement, tokens: &mut Vec<SemanticToken>) {
        match stmt {
            Statement::Let { name, .. } => {
                tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: name.len() as u32,
                    token_type: SemanticTokenType::VARIABLE.into(),
                    token_modifiers_bitset: 0,
                });
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for s in then_block {
                    self.collect_tokens_from_statement(s, tokens);
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        self.collect_tokens_from_statement(s, tokens);
                    }
                }
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::Loop { body } => {
                for s in body {
                    self.collect_tokens_from_statement(s, tokens);
                }
            }
            _ => {}
        }
    }

    fn collect_tokens_from_type(&self, ty: &Type, tokens: &mut Vec<SemanticToken>) {
        match ty {
            Type::Custom(name) => {
                tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: name.len() as u32,
                    token_type: SemanticTokenType::TYPE.into(),
                    token_modifiers_bitset: 0,
                });
            }
            Type::Parameterized(name, params) => {
                tokens.push(SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: name.len() as u32,
                    token_type: SemanticTokenType::TYPE.into(),
                    token_modifiers_bitset: 0,
                });
                for param in params {
                    self.collect_tokens_from_type(param, tokens);
                }
            }
            Type::Vec(inner) | Type::Reference(inner) | Type::MutableReference(inner) => {
                self.collect_tokens_from_type(inner, tokens);
            }
            Type::Tuple(types) => {
                for ty in types {
                    self.collect_tokens_from_type(ty, tokens);
                }
            }
            _ => {}
        }
    }
}

/// Get the legend for semantic tokens
pub fn get_semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::MODIFICATION,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ],
    }
}
