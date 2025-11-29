// Parser - Windjammer Language Parser
//
// This file contains the complete parser for Windjammer. It is organized into the following sections:
//
// 1. AST TYPES (lines ~3-340)
//    - Type, TypeParam, Parameter, FunctionDecl, StructDecl, EnumDecl, TraitDecl, ImplBlock
//    - Expression, Statement, Pattern, Item, Program
//
// 2. PARSER CORE (lines ~344-400)
//    - Parser struct
//    - Basic utilities: new(), current_token(), advance(), expect(), peek()
//    - Helper: type_to_string()
//
// 3. TOP-LEVEL PARSING (lines ~400-700)
//    - parse() - main entry point
//    - parse_item() - dispatches to item parsers
//    - parse_const_or_static()
//    - parse_use()
//    - parse_decorator() and parse_decorator_arguments()
//
// 4. ITEM PARSING (lines ~700-1500)
//    - parse_impl() - impl blocks with generics and trait impls
//    - parse_trait() - trait definitions
//    - parse_function() - function declarations
//    - parse_parameters()
//    - parse_struct() - struct definitions
//    - parse_enum() - enum definitions with generics
//    - parse_type_params() - generic type parameters with bounds
//    - parse_where_clause() - where clauses
//
// 5. STATEMENT PARSING (lines ~1500-1900)
//    - parse_block_statements()
//    - parse_statement() - dispatches to statement parsers
//    - parse_const_statement(), parse_static_statement()
//    - parse_let(), parse_return()
//    - parse_if(), parse_match()
//    - parse_for(), parse_loop(), parse_while()
//    - parse_go(), parse_defer()
//
// 6. PATTERN PARSING (lines ~1900-2000)
//    - parse_pattern_with_or() - OR patterns
//    - parse_pattern() - all pattern types including enum variants
//
// 7. EXPRESSION PARSING (lines ~2000-2800)
//    - parse_expression() - entry point
//    - parse_ternary_expression() - ternary operator
//    - parse_match_value() - match value with special handling
//    - parse_binary_expression() - operator precedence climbing
//    - get_binary_op() - operator precedence table
//    - parse_primary_expression() - literals, identifiers, calls, etc.
//    - parse_postfix_expression() - method calls, field access, indexing, turbofish
//    - parse_arguments()
//    - parse_closure()
//
// 8. TYPE PARSING (lines ~2800+)
//    - parse_type() - all type variants
//
// TODO: Split this into modules:
//   - parser/mod.rs - Parser struct and utilities
//   - parser/types.rs - Type parsing
//   - parser/patterns.rs - Pattern parsing
//   - parser/expressions.rs - Expression parsing
//   - parser/statements.rs - Statement parsing
//   - parser/items.rs - Top-level item parsing

use crate::lexer::Token;

// Import all AST types from the new parser::ast module
pub use crate::parser::ast::*;

// ============================================================================
// SECTION 2: PARSER CORE
// ============================================================================

pub struct Parser {
    pub(crate) tokens: Vec<crate::lexer::TokenWithLocation>,
    pub(crate) position: usize,
    pub(crate) filename: String,
    #[allow(dead_code)]
    pub(crate) source: String,
}

impl Parser {
    pub fn new(tokens: Vec<crate::lexer::TokenWithLocation>) -> Self {
        Parser {
            tokens,
            position: 0,
            filename: String::new(),
            source: String::new(),
        }
    }

    pub fn new_with_source(
        tokens: Vec<crate::lexer::TokenWithLocation>,
        filename: String,
        source: String,
    ) -> Self {
        Parser {
            tokens,
            position: 0,
            filename,
            source,
        }
    }

    pub(crate) fn current_token(&self) -> &Token {
        self.tokens
            .get(self.position)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    /// Get the current token's location for source mapping
    pub(crate) fn current_location(&self) -> Option<crate::source_map::Location> {
        self.tokens
            .get(self.position)
            .map(|t| crate::source_map::Location {
                file: std::path::PathBuf::from(&self.filename),
                line: t.line,
                column: t.column,
            })
    }

    pub(crate) fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    pub(crate) fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.current_token() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} (at token position {})",
                expected,
                self.current_token(),
                self.position
            ))
        }
    }

    /// Check if there was a newline before the current token
    /// Used for Automatic Semicolon Insertion (ASI) rules
    pub(crate) fn had_newline_before_current(&self) -> bool {
        if self.position == 0 {
            return false;
        }
        
        // Get the previous and current token locations
        if let (Some(prev), Some(curr)) = (
            self.tokens.get(self.position - 1),
            self.tokens.get(self.position),
        ) {
            // If the current token is on a different line than the previous token, there was a newline
            curr.line > prev.line
        } else {
            false
        }
    }

    // ========================================================================
    // SECTION 3: TOP-LEVEL PARSING
    // ========================================================================

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();

        while self.current_token() != &Token::Eof {
            items.push(self.parse_item()?);
            
            // Consume optional semicolon after items (ASI - semicolons are optional)
            if self.current_token() == &Token::Semicolon {
                self.advance();
            }
        }

        Ok(Program { items })
    }

    pub(crate) fn parse_item(&mut self) -> Result<Item, String> {
        // Check for decorators
        let mut decorators = Vec::new();
        while let Token::Decorator(_) = self.current_token() {
            decorators.push(self.parse_decorator()?);
        }

        // Check for pub keyword (for module functions)
        let is_pub = if self.current_token() == &Token::Pub {
            self.advance();
            true
        } else {
            false
        };

        match self.current_token() {
            Token::Extern => {
                self.advance(); // Consume the Extern token
                self.expect(Token::Fn)?;
                let mut func = self.parse_function()?;
                func.is_extern = true;
                func.is_pub = is_pub;
                func.decorators = decorators.clone();
                Ok(Item::Function {
                    decl: func,
                    location: self.current_location(),
                })
            }
            Token::Fn => {
                self.advance(); // Consume the Fn token
                let mut func = self.parse_function()?;
                func.decorators = decorators.clone();
                func.is_pub = is_pub;
                // Check if @async decorator is present
                if decorators.iter().any(|d| d.name == "async") {
                    func.is_async = true;
                }
                Ok(Item::Function {
                    decl: func,
                    location: self.current_location(),
                })
            }
            Token::Async => {
                self.advance();
                self.expect(Token::Fn)?;
                let mut func = self.parse_function()?;
                func.is_async = true;
                func.is_pub = is_pub;
                func.decorators = decorators;
                Ok(Item::Function {
                    decl: func,
                    location: self.current_location(),
                })
            }
            Token::Struct => {
                self.advance();
                let mut struct_decl = self.parse_struct()?;
                struct_decl.decorators = decorators;
                struct_decl.is_pub = is_pub;
                Ok(Item::Struct {
                    decl: struct_decl,
                    location: self.current_location(),
                })
            }
            Token::Enum => {
                self.advance();
                let mut enum_decl = self.parse_enum()?;
                enum_decl.is_pub = is_pub;
                Ok(Item::Enum {
                    decl: enum_decl,
                    location: self.current_location(),
                })
            }
            Token::Trait => {
                self.advance();
                Ok(Item::Trait {
                    decl: self.parse_trait()?,
                    location: self.current_location(),
                })
            }
            Token::Impl => {
                self.advance();
                let mut impl_block = self.parse_impl()?;
                impl_block.decorators = decorators;
                Ok(Item::Impl {
                    block: impl_block,
                    location: self.current_location(),
                })
            }
            Token::Const => {
                self.advance();
                let (name, type_, value) = self.parse_const_or_static()?;
                // For now, we don't store is_pub in the AST (future enhancement)
                // But at least we parse it correctly
                let _ = is_pub; // Suppress unused warning
                Ok(Item::Const {
                    name,
                    type_,
                    value,
                    location: self.current_location(),
                })
            }
            Token::Static => {
                self.advance();
                let mutable = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let (name, type_, value) = self.parse_const_or_static()?;
                Ok(Item::Static {
                    name,
                    mutable,
                    type_,
                    value,
                    location: self.current_location(),
                })
            }
            Token::Use => {
                self.advance(); // consume 'use'
                let (path, alias) = self.parse_use()?;
                Ok(Item::Use {
                    path,
                    alias,
                    location: self.current_location(),
                })
            }
            Token::Mod => {
                self.advance(); // consume 'mod'
                let (name, items, _) = self.parse_mod()?;
                Ok(Item::Mod {
                    name,
                    items,
                    is_public: is_pub,
                    location: self.current_location(),
                })
            }
            Token::Bound => {
                self.advance(); // consume 'bound'
                self.parse_bound_alias()
            }
            _ => Err(format!(
                "Unexpected token: {:?} (at token position {})",
                self.current_token(),
                self.position
            )),
        }
    }

    fn parse_bound_alias(&mut self) -> Result<Item, String> {
        // bound Name = Trait + Trait + ...
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected bound alias name".to_string());
        };

        self.expect(Token::Assign)?;

        // Parse trait list: Trait + Trait + ...
        let mut traits = Vec::new();
        loop {
            if let Token::Ident(trait_name) = self.current_token() {
                traits.push(trait_name.clone());
                self.advance();
            } else {
                return Err("Expected trait name in bound alias".to_string());
            }

            if self.current_token() == &Token::Plus {
                self.advance(); // consume +
            } else {
                break;
            }
        }

        Ok(Item::BoundAlias {
            name,
            traits,
            location: self.current_location(),
        })
    }

    pub(crate) fn parse_const_or_static(&mut self) -> Result<(String, Type, Expression), String> {
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected const/static name".to_string());
        };

        self.expect(Token::Colon)?;
        let type_ = self.parse_type()?;

        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;

        Ok((name, type_, value))
    }

    // Helper: Extract a name from a pattern for backward compatibility

    // Public wrapper methods for component compiler
    pub fn parse_expression_public(&mut self) -> Result<Expression, String> {
        self.parse_expression()
    }

    pub fn parse_function_public(&mut self) -> Result<FunctionDecl, String> {
        self.parse_function()
    }
}
