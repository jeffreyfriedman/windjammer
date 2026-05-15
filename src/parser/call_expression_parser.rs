// Postfix operators: calls, method calls, indexing, slicing, casts, macros.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn parse_postfix_expression(
        &mut self,
        mut expr: &'static Expression<'static>,
    ) -> Result<&'static Expression<'static>, String> {
        loop {
            expr = match self.current_token() {
                Token::Dot => {
                    // Peek ahead to check for .await
                    if self.peek(1) == Some(&Token::Await) {
                        self.advance(); // consume '.'
                        self.advance(); // consume 'await'
                        self.alloc_expr(Expression::Await {
                            expr,
                            location: self.current_location(),
                        })
                    } else {
                        self.advance();
                        // Allow keywords and numeric indices as field names (e.g., std.thread, tuple.0)
                        let field_opt = match self.current_token() {
                            Token::Ident(f) => Some(f.clone()),
                            Token::Thread => Some("thread".to_string()),
                            Token::Async => Some("async".to_string()),
                            Token::IntLiteral(n) | Token::IntLiteralSuffixed(n, _) => {
                                Some(n.to_string())
                            }
                            _ => None,
                        };
                        if let Some(field) = field_opt {
                            self.advance();

                            // Check for turbofish ::<Type>
                            let type_args = if self.current_token() == &Token::ColonColon {
                                // Peek ahead to see if this is turbofish
                                if self.peek(1) == Some(&Token::Lt) {
                                    self.advance(); // consume ::
                                    self.advance(); // consume <
                                    let mut types = vec![self.parse_type()?];
                                    while self.current_token() == &Token::Comma {
                                        self.advance();
                                        if self.current_token() != &Token::Gt
                                            && self.current_token() != &Token::Shr
                                        {
                                            types.push(self.parse_type()?);
                                        }
                                    }
                                    self.expect_gt_or_split_shr()?; // Handle nested generics
                                    Some(types)
                                } else {
                                    // Not turbofish - don't consume ::
                                    None
                                }
                            } else {
                                None
                            };

                            if self.current_token() == &Token::LParen {
                                // Check for newline before LParen (ASI)
                                if self.had_newline_before_current() {
                                    // ASI: This LParen starts a new statement, not a method call
                                    // Create a field access and break
                                    self.alloc_expr(Expression::FieldAccess {
                                        object: expr,
                                        field,
                                        location: self.current_location(),
                                    })
                                } else {
                                    // Method call (possibly with turbofish)
                                    self.advance();
                                    let arguments = self.parse_arguments()?;
                                    self.expect(Token::RParen)?;
                                    self.alloc_expr(Expression::MethodCall {
                                        object: expr,
                                        method: field,
                                        type_args,
                                        arguments,
                                        location: self.current_location(),
                                    })
                                }
                            } else if type_args.is_some() {
                                return Err(
                                    "Turbofish syntax only allowed on method calls".to_string()
                                );
                            } else {
                                // Field access
                                self.alloc_expr(Expression::FieldAccess {
                                    object: expr,
                                    field,
                                    location: self.current_location(),
                                })
                            }
                        } else {
                            return Err(format!(
                                "Expected field or method name (at token position {})",
                                self.position
                            ));
                        }
                    }
                }
                Token::ColonColon => {
                    // Turbofish on function/static method: func::<Type>() or Type::method::<T>()
                    self.advance();
                    if self.current_token() == &Token::Lt {
                        self.advance();
                        let mut types = vec![self.parse_type()?];
                        while self.current_token() == &Token::Comma {
                            self.advance();
                            if self.current_token() != &Token::Gt
                                && self.current_token() != &Token::Shr
                            {
                                types.push(self.parse_type()?);
                            }
                        }
                        self.expect_gt_or_split_shr()?; // Handle nested generics

                        // Now expect either () for call, :: for path continuation, or identifier
                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            // Convert to method call with turbofish
                            // For func::<T>(), treat as a special method call on the function
                            self.alloc_expr(Expression::MethodCall {
                                object: expr,
                                method: String::new(), // Empty method name signals turbofish call
                                type_args: Some(types),
                                arguments,
                                location: self.current_location(),
                            })
                        } else if self.current_token() == &Token::ColonColon {
                            // Vec::<int>::new() - another :: after turbofish
                            // Continue parsing in the loop, the :: will be handled on next iteration
                            // We need to represent this as a turbofish-qualified path
                            // For now, convert the types to a string and append to the identifier
                            let mut type_str = String::new();
                            type_str.push_str("::<");
                            for (i, ty) in types.iter().enumerate() {
                                if i > 0 {
                                    type_str.push_str(", ");
                                }
                                type_str.push_str(&self.type_to_string(ty));
                            }
                            type_str.push('>');

                            // Update the expression to include the turbofish
                            if let Expression::Identifier { name, .. } = expr {
                                expr = self.alloc_expr(Expression::Identifier {
                                    name: format!("{}{}", name, type_str),
                                    location: self.current_location(),
                                });
                            } else {
                                return Err(
                                    "Turbofish can only be applied to identifiers".to_string()
                                );
                            }
                            // Continue the loop to handle the next ::
                            continue;
                        } else if let Token::Ident(method) = self.current_token() {
                            // Type::method or module::function continuation
                            let method = method.clone();
                            self.advance();
                            self.alloc_expr(Expression::MethodCall {
                                object: expr,
                                method,
                                type_args: None,
                                arguments: vec![],
                                location: self.current_location(),
                            })
                        } else {
                            return Err(format!(
                                "Expected '(', '::', or identifier after '::<Type>', got {:?}",
                                self.current_token()
                            ));
                        }
                    } else {
                        // Allow keywords as identifiers after :: (e.g., std::thread, std::async)
                        let method = match self.current_token() {
                            Token::Ident(n) => n.clone(),
                            Token::Thread => "thread".to_string(),
                            Token::Async => "async".to_string(),
                            Token::Await => "await".to_string(),
                            _ => {
                                return Err(format!(
                                    "Expected '<' or identifier after '::', got {:?}",
                                    self.current_token()
                                ));
                            }
                        };
                        self.advance();

                        // Check for turbofish on this method
                        let type_args = if self.current_token() == &Token::ColonColon {
                            // Peek ahead to see if this is turbofish
                            if self.peek(1) == Some(&Token::Lt) {
                                self.advance(); // consume ::
                                self.advance(); // consume <
                                let mut types = vec![self.parse_type()?];
                                while self.current_token() == &Token::Comma {
                                    self.advance();
                                    if self.current_token() != &Token::Gt
                                        && self.current_token() != &Token::Shr
                                    {
                                        types.push(self.parse_type()?);
                                    }
                                }
                                self.expect_gt_or_split_shr()?; // Handle nested generics
                                Some(types)
                            } else {
                                // Not turbofish - don't consume ::
                                None
                            }
                        } else {
                            None
                        };

                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            self.alloc_expr(Expression::MethodCall {
                                object: expr,
                                method,
                                type_args,
                                arguments,
                                location: self.current_location(),
                            })
                        } else {
                            // Just a path, treat as field access
                            self.alloc_expr(Expression::FieldAccess {
                                object: expr,
                                field: method,
                                location: self.current_location(),
                            })
                        }
                    }
                }
                Token::LParen => {
                    // Check for newline before LParen (automatic semicolon insertion)
                    // If there was a newline, this might be a new statement, not a function call
                    if self.had_newline_before_current() {
                        // ASI: Treat newline as statement terminator
                        // Don't consume the LParen - it belongs to the next statement
                        break;
                    }

                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect(Token::RParen)?;
                    self.alloc_expr(Expression::Call {
                        function: expr,
                        arguments,
                        location: self.current_location(),
                    })
                }
                Token::Question => {
                    // TryOp: expr?
                    // No ambiguity since we removed ternary operator
                    self.advance();
                    self.alloc_expr(Expression::TryOp {
                        expr,
                        location: self.current_location(),
                    })
                }
                Token::LBracket => {
                    self.advance();

                    // Check for slice syntax: [start..end], [start..], [..end]
                    if self.current_token() == &Token::DotDot {
                        // [..end] - slice from beginning
                        self.advance(); // consume '..'
                        let end = if self.current_token() != &Token::RBracket {
                            Some(self.parse_expression()?)
                        } else {
                            None
                        };
                        self.expect(Token::RBracket)?;

                        // Desugar [..end] to .slice(0, end)
                        let end_expr = end.unwrap_or_else(|| {
                            self.alloc_expr(Expression::MethodCall {
                                object: self.alloc_expr(expr.clone()),
                                method: "len".to_string(),
                                type_args: None,
                                arguments: vec![],
                                location: self.current_location(),
                            })
                        });

                        self.alloc_expr(Expression::MethodCall {
                            object: expr,
                            method: "slice".to_string(),
                            type_args: None,
                            arguments: vec![
                                (
                                    None,
                                    self.alloc_expr(Expression::Literal {
                                        value: Literal::Int(0),
                                        location: self.current_location(),
                                    }),
                                ),
                                (None, end_expr),
                            ],
                            location: self.current_location(),
                        })
                    } else {
                        // Parse the first expression
                        // We need to parse without consuming .. as a range operator
                        // So we manually parse a binary expression that stops at ..
                        let start_or_index = self.parse_binary_expression(0)?;

                        // Check if this is a slice or regular index
                        if self.current_token() == &Token::DotDot {
                            // [start..] or [start..end] - slice syntax
                            self.advance(); // consume '..'
                            let end = if self.current_token() != &Token::RBracket {
                                Some(self.parse_expression()?)
                            } else {
                                None
                            };
                            self.expect(Token::RBracket)?;

                            // Desugar [start..end] to .slice(start, end)
                            let end_expr = end.unwrap_or_else(|| {
                                self.alloc_expr(Expression::MethodCall {
                                    object: expr,
                                    method: "len".to_string(),
                                    type_args: None,
                                    arguments: vec![],
                                    location: self.current_location(),
                                })
                            });

                            self.alloc_expr(Expression::MethodCall {
                                object: expr,
                                method: "slice".to_string(),
                                type_args: None,
                                arguments: vec![(None, start_or_index), (None, end_expr)],
                                location: self.current_location(),
                            })
                        } else {
                            // Regular index: [i]
                            self.expect(Token::RBracket)?;
                            self.alloc_expr(Expression::Index {
                                object: expr,
                                index: start_or_index,
                                location: self.current_location(),
                            })
                        }
                    }
                }
                Token::DotDot | Token::DotDotEq => {
                    // Don't parse as range if followed by ] (that's slice syntax)
                    if let Some(next_tok) = self.peek(1) {
                        if next_tok == &Token::RBracket {
                            // This is slice syntax like [1..], not a range
                            // Let the LBracket handler deal with it
                            break;
                        }
                    }

                    let inclusive = self.current_token() == &Token::DotDotEq;
                    self.advance();
                    let end = self.parse_primary_expression()?;
                    self.alloc_expr(Expression::Range {
                        start: expr,
                        end,
                        inclusive,
                        location: self.current_location(),
                    })
                }
                Token::As => {
                    self.advance();
                    let type_ = self.parse_type()?;
                    self.alloc_expr(Expression::Cast {
                        expr,
                        type_,
                        location: self.current_location(),
                    })
                }
                Token::Bang => {
                    // Macro invocation: name!(...) or name![...] or name!{...}
                    if let Expression::Identifier { name, .. } = expr {
                        self.advance(); // consume '!'

                        let (delimiter, end_token) = match self.current_token() {
                            Token::LParen => (MacroDelimiter::Parens, Token::RParen),
                            Token::LBracket => (MacroDelimiter::Brackets, Token::RBracket),
                            Token::LBrace => (MacroDelimiter::Braces, Token::RBrace),
                            _ => return Err("Expected (, [, or { after macro name!".to_string()),
                        };

                        self.advance(); // consume opening delimiter

                        let mut args = Vec::new();

                        // Check for empty macro: vec![], println!()
                        let mut is_repeat_flag = false;
                        if self.current_token() != &end_token {
                            // Parse first argument
                            args.push(self.parse_expression()?);

                            // Check for vec![item; count] repetition syntax
                            if delimiter == MacroDelimiter::Brackets
                                && self.current_token() == &Token::Semicolon
                            {
                                self.advance(); // consume semicolon
                                args.push(self.parse_expression()?);
                                is_repeat_flag = true; // This is vec![x; n] repeat syntax
                            } else {
                                // Parse remaining comma-separated arguments
                                while self.current_token() == &Token::Comma {
                                    self.advance();

                                    // Allow trailing comma
                                    if self.current_token() == &end_token {
                                        break;
                                    }

                                    args.push(self.parse_expression()?);
                                }
                            }
                        }

                        self.expect(end_token)?;

                        if name == "format" {
                            let (file, line) = if let Some(loc) = self.current_location() {
                                (Some(loc.file.display().to_string()), Some(loc.line))
                            } else {
                                (None, None)
                            };
                            self.emit_warning(
                                "format!() is Rust syntax. Use string interpolation instead: \"text ${expr}\"".to_string(),
                                file,
                                line,
                                None,
                            );
                        }

                        self.alloc_expr(Expression::MacroInvocation {
                            name: name.clone(),
                            args,
                            delimiter,
                            is_repeat: is_repeat_flag,
                            location: self.current_location(),
                        })
                    } else {
                        // Not a macro invocation, break out of postfix loop
                        break;
                    }
                }
                _ => break,
            };
        }

        Ok(expr)
    }
}
