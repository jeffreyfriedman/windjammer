// Function and parameter parsing — extracted from item_parser.rs

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_function(&mut self) -> Result<FunctionDecl<'static>, String> {
        // Note: Token::Fn already consumed in parse_item

        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected function name".to_string());
        };

        // Parse type parameters: fn foo<T, U>(...)
        let type_params = self.parse_type_params()?;

        self.expect(Token::LParen)?;
        let parameters = self.parse_parameters()?;
        self.expect(Token::RParen)?;

        // Parse return type with optional decorators: -> @location(0) vec4<float>
        let (return_type, return_decorators) = if self.current_token() == &Token::Arrow {
            self.advance();

            // Collect decorators on return type
            let mut ret_decorators = Vec::new();
            while matches!(self.current_token(), Token::At | Token::Decorator(_)) {
                ret_decorators.push(self.parse_decorator()?);
            }

            (Some(self.parse_type()?), ret_decorators)
        } else {
            (None, Vec::new())
        };

        // Parse where clause (optional): where T: Display, U: Debug
        let where_clause = self.parse_where_clause()?;

        // Parse body (or semicolon for extern functions)
        // Semicolons are optional (Windjammer philosophy)
        let body = if self.current_token() == &Token::Semicolon {
            self.advance();
            Vec::new() // Empty body for extern functions
        } else if self.current_token() == &Token::LBrace {
            self.expect(Token::LBrace)?;
            let statements = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;
            statements
        } else {
            // No semicolon and no body - assume extern function
            Vec::new()
        };

        Ok(FunctionDecl {
            name,
            is_pub: false,          // Set by parse_item if pub keyword present
            is_extern: false,       // Set by parse_item if extern keyword present
            type_params,            // Parsed generic type parameters
            where_clause,           // Parsed where clause
            decorators: Vec::new(), // Set by parse_item
            is_async: false,        // Set by parse_item
            parameters,
            return_type,
            return_decorators, // Decorators on return type
            body,
            parent_type: None, // Set by parse_impl for methods
            impl_trait: None,
            doc_comment: None, // Set by parse_item if doc comments present
        })
    }

    pub(in crate::parser) fn parse_parameters(
        &mut self,
    ) -> Result<Vec<Parameter<'static>>, String> {
        let mut params = Vec::new();

        while self.current_token() != &Token::RParen {
            // Parse parameter decorators (@builtin, etc.)
            let mut decorators = Vec::new();
            while let Token::Decorator(_) = self.current_token() {
                decorators.push(self.parse_decorator()?);
            }

            // Check for self parameters
            if self.current_token() == &Token::Ampersand {
                self.advance();
                if self.current_token() == &Token::Mut {
                    self.advance();
                    self.expect(Token::Self_)?;
                    params.push(Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Mut,
                        is_mutable: false,
                        decorators: decorators.clone(),
                    });
                } else {
                    self.expect(Token::Self_)?;
                    params.push(Parameter {
                        name: "self".to_string(),
                        pattern: None,
                        type_: Type::Custom("Self".to_string()),
                        ownership: OwnershipHint::Ref,
                        is_mutable: false,
                        decorators: decorators.clone(),
                    });
                }
            } else if self.current_token() == &Token::Self_ {
                self.advance();
                params.push(Parameter {
                    name: "self".to_string(),
                    pattern: None,
                    type_: Type::Custom("Self".to_string()),
                    // SMART OWNERSHIP FIX: Let analyzer infer &self, &mut self, or self
                    // based on whether the method reads or writes fields!
                    ownership: OwnershipHint::Inferred,
                    is_mutable: false,
                    decorators: decorators.clone(),
                });
            } else if self.current_token() == &Token::Mut && self.peek(1) == Some(&Token::Self_) {
                // WINDJAMMER PHILOSOPHY: Reject `mut self` - ownership is inferred automatically
                let (file, line, col) = self
                    .current_location()
                    .map(|loc| (format!("{}", loc.file.display()), loc.line, loc.column))
                    .unwrap_or(("unknown".to_string(), 0, 0));
                return Err(format!(
                    "error: `mut` is not needed for method parameters\n \
                     --> {}:{}:{}\n  |\n \
                     {} | fn ...(mut self) {{\n  |           ^^^ help: remove `mut`, ownership is inferred automatically\n  |\n \
                     = note: Windjammer infers `&self`, `&mut self`, or owned based on usage\n \
                     = note: Use `let mut x` for local variable mutability",
                    file, line, col, line
                ));
            } else {
                // Regular parameter - could be a simple name or a pattern
                // Check if this is a pattern parameter (starts with '(')
                if self.current_token() == &Token::LParen {
                    // Parse tuple pattern
                    let pattern = self.parse_pattern()?;
                    self.expect(Token::Colon)?;
                    let type_ = self.parse_type()?;

                    // Extract a name from the pattern for backward compatibility
                    let name = Self::pattern_to_name(&pattern);

                    // CRITICAL FIX: Determine ownership from the type annotation
                    // If the type is explicitly &T or &mut T, use that.
                    // Otherwise, treat it as owned (pass by value).
                    let ownership = match &type_ {
                        Type::Reference(_) => OwnershipHint::Ref,
                        Type::MutableReference(_) => OwnershipHint::Mut,
                        _ => OwnershipHint::Inferred, // Let analyzer infer ownership based on usage
                    };

                    params.push(Parameter {
                        name,
                        pattern: Some(pattern),
                        type_,
                        ownership,
                        is_mutable: false,
                        decorators: decorators.clone(),
                    });
                } else {
                    // Simple identifier parameter
                    // Check for 'mut' keyword and preserve it
                    let is_mutable = if self.current_token() == &Token::Mut {
                        self.advance();
                        true
                    } else {
                        false
                    };

                    let name = if let Token::Ident(n) = self.current_token() {
                        let name = n.clone();
                        self.advance();
                        name
                    } else {
                        return Err(format!(
                            "Expected parameter name (at token position {})",
                            self.position
                        ));
                    };

                    self.expect(Token::Colon)?;
                    let type_ = self.parse_type()?;

                    // CRITICAL FIX: Determine ownership from the type annotation
                    // If the type is explicitly &T or &mut T, use that.
                    // Otherwise, let the analyzer infer based on usage.
                    // This allows the analyzer to automatically add &mut when parameters are mutated.
                    let ownership = match &type_ {
                        Type::Reference(_) => OwnershipHint::Ref,
                        Type::MutableReference(_) => OwnershipHint::Mut,
                        _ => OwnershipHint::Inferred, // Let analyzer infer ownership based on usage
                    };

                    params.push(Parameter {
                        name,
                        pattern: None,
                        type_,
                        ownership,
                        is_mutable,
                        decorators: decorators.clone(),
                    });
                }
            }

            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }
}
