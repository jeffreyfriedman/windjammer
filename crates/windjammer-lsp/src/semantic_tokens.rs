// Semantic highlighting provider for Windjammer LSP
//
// Provides context-aware syntax coloring beyond simple textmate grammars

use tower_lsp::lsp_types::*;
use windjammer::parser::{FunctionDecl, Item, Parameter, Program, Type as WJType};

/// Token type indices for LSP semantic highlighting
/// These map to the semantic token types registered with the client
pub const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::TYPE,
    SemanticTokenType::STRUCT,
    SemanticTokenType::ENUM,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
];

fn token_type_to_index(token_type: SemanticTokenType) -> u32 {
    SEMANTIC_TOKEN_TYPES
        .iter()
        .position(|t| t == &token_type)
        .unwrap_or(0) as u32
}

/// Provides semantic token information for syntax highlighting
pub struct SemanticTokensProvider {
    program: Option<Program<'static>>,
    source: String,
}

impl SemanticTokensProvider {
    pub fn new() -> Self {
        SemanticTokensProvider {
            program: None,
            source: String::new(),
        }
    }

    pub fn update_program(&mut self, program: Program, source: String) {
        self.program = Some(program);
        self.source = source;
    }

    /// Generate semantic tokens for the entire document
    pub fn get_semantic_tokens(&self) -> Option<Vec<SemanticToken>> {
        let program = self.program.as_ref()?;
        let mut tokens = Vec::new();

        // Collect tokens from all top-level items
        for item in &program.items {
            self.collect_item_tokens(item, &mut tokens);
        }

        // Sort tokens by position (line, then character)
        tokens.sort_by(|a, b| {
            if a.delta_line != b.delta_line {
                a.delta_line.cmp(&b.delta_line)
            } else {
                a.delta_start.cmp(&b.delta_start)
            }
        });

        // Convert to delta encoding
        let delta_tokens = self.encode_delta(&tokens);

        Some(delta_tokens)
    }

    fn collect_item_tokens(&self, item: &Item, tokens: &mut Vec<SemanticToken>) {
        match item {
            Item::Function {
                decl: func,
                location: _,
            } => {
                self.collect_function_tokens(func, tokens);
            }
            Item::Struct {
                decl: s,
                location: _,
            } => {
                // Add struct name token
                if let Ok(pos) = self.find_identifier_position(&s.name, 0) {
                    tokens.push(SemanticToken {
                        delta_line: pos.0,
                        delta_start: pos.1,
                        length: s.name.len() as u32,
                        token_type: token_type_to_index(SemanticTokenType::STRUCT),
                        token_modifiers_bitset: 0,
                    });
                }
            }
            Item::Enum {
                decl: e,
                location: _,
            } => {
                // Add enum name token
                if let Ok(pos) = self.find_identifier_position(&e.name, 0) {
                    tokens.push(SemanticToken {
                        delta_line: pos.0,
                        delta_start: pos.1,
                        length: e.name.len() as u32,
                        token_type: token_type_to_index(SemanticTokenType::ENUM),
                        token_modifiers_bitset: 0,
                    });
                }
            }
            _ => {}
        }
    }

    fn collect_function_tokens(&self, func: &FunctionDecl, tokens: &mut Vec<SemanticToken>) {
        // Add function name token
        if let Ok(pos) = self.find_identifier_position(&func.name, 0) {
            tokens.push(SemanticToken {
                delta_line: pos.0,
                delta_start: pos.1,
                length: func.name.len() as u32,
                token_type: token_type_to_index(SemanticTokenType::FUNCTION),
                token_modifiers_bitset: 0,
            });
        }

        // Add parameter tokens
        for param in &func.parameters {
            self.collect_parameter_tokens(param, tokens);
        }
    }

    fn collect_parameter_tokens(&self, param: &Parameter, tokens: &mut Vec<SemanticToken>) {
        // Add parameter name token
        if let Ok(pos) = self.find_identifier_position(&param.name, 0) {
            tokens.push(SemanticToken {
                delta_line: pos.0,
                delta_start: pos.1,
                length: param.name.len() as u32,
                token_type: token_type_to_index(SemanticTokenType::PARAMETER),
                token_modifiers_bitset: 0,
            });
        }

        // Add type token
        self.collect_type_tokens(&param.type_, tokens);
    }

    fn collect_type_tokens(&self, ty: &WJType, tokens: &mut Vec<SemanticToken>) {
        let type_name = match ty {
            WJType::Custom(name) => name.as_str(),
            WJType::Generic(name) => name.as_str(),
            WJType::Parameterized(base, params) => {
                // Handle base type
                if let Ok(pos) = self.find_identifier_position(base, 0) {
                    tokens.push(SemanticToken {
                        delta_line: pos.0,
                        delta_start: pos.1,
                        length: base.len() as u32,
                        token_type: token_type_to_index(SemanticTokenType::TYPE),
                        token_modifiers_bitset: 0,
                    });
                }
                // Recursively handle type parameters
                for param in params {
                    self.collect_type_tokens(param, tokens);
                }
                return;
            }
            WJType::Vec(inner) | WJType::Option(inner) => {
                self.collect_type_tokens(inner, tokens);
                return;
            }
            WJType::Array(inner, _size) => {
                self.collect_type_tokens(inner, tokens);
                return;
            }
            WJType::Result(ok, err) => {
                self.collect_type_tokens(ok, tokens);
                self.collect_type_tokens(err, tokens);
                return;
            }
            WJType::Tuple(types) => {
                for t in types {
                    self.collect_type_tokens(t, tokens);
                }
                return;
            }
            WJType::Reference(inner) | WJType::MutableReference(inner) => {
                self.collect_type_tokens(inner, tokens);
                return;
            }
            WJType::TraitObject(name) => name.as_str(),
            WJType::Associated(base, _assoc) => base.as_str(),
            // Primitive types and infer - don't need highlighting
            WJType::Int
            | WJType::Int32
            | WJType::Uint
            | WJType::Float
            | WJType::Bool
            | WJType::String
            | WJType::Infer
            | WJType::FunctionPointer { .. } => return,
        };

        if let Ok(pos) = self.find_identifier_position(type_name, 0) {
            tokens.push(SemanticToken {
                delta_line: pos.0,
                delta_start: pos.1,
                length: type_name.len() as u32,
                token_type: token_type_to_index(SemanticTokenType::TYPE),
                token_modifiers_bitset: 0,
            });
        }
    }

    /// Find the position (line, column) of an identifier in the source
    fn find_identifier_position(
        &self,
        identifier: &str,
        start_offset: usize,
    ) -> Result<(u32, u32), ()> {
        let source_bytes = self.source.as_bytes();
        if let Some(offset) = self.source[start_offset..].find(identifier) {
            let absolute_offset = start_offset + offset;
            let mut line = 0u32;
            let mut col = 0u32;

            for &byte in source_bytes[..absolute_offset].iter() {
                if byte == b'\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }
            }

            Ok((line, col))
        } else {
            Err(())
        }
    }

    /// Convert absolute position tokens to delta-encoded tokens
    fn encode_delta(&self, tokens: &[SemanticToken]) -> Vec<SemanticToken> {
        let mut result = Vec::new();
        let mut prev_line = 0;
        let mut prev_col = 0;

        for token in tokens {
            let delta_line = token.delta_line - prev_line;
            let delta_start = if delta_line == 0 {
                token.delta_start - prev_col
            } else {
                token.delta_start
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.token_modifiers_bitset,
            });

            prev_line = token.delta_line;
            prev_col = token.delta_start;
        }

        result
    }
}

impl Default for SemanticTokensProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to get semantic token legend for client registration
pub fn get_semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
        token_modifiers: vec![],
    }
}
