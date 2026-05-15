// Match-value expression parsing (patterns / match arms), without struct-literal path.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn parse_match_value(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        // Parse a non-struct-literal expression for match values
        // This is basically parse_binary_expression but without struct literal support
        let mut left = match self.current_token() {
            Token::LParen => {
                self.advance();

                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    return Ok(self.alloc_expr(Expression::Tuple {
                        elements: vec![],
                        location: self.current_location(),
                    }));
                }

                // Parse the first expression inside parentheses
                // Use parse_match_value recursively to avoid parsing assignment operators
                let first_expr = self.parse_match_value()?;

                // Check if it's a tuple (has comma) or just a parenthesized expression
                if self.current_token() == &Token::Comma {
                    let mut elements = vec![first_expr];

                    while self.current_token() == &Token::Comma {
                        self.advance(); // consume comma

                        // Allow trailing comma
                        if self.current_token() == &Token::RParen {
                            break;
                        }

                        elements.push(self.parse_match_value()?);
                    }

                    self.expect(Token::RParen)?;
                    self.alloc_expr(Expression::Tuple {
                        elements,
                        location: self.current_location(),
                    })
                } else {
                    // Just a parenthesized expression
                    self.expect(Token::RParen)?;
                    first_expr
                }
            }
            Token::LBracket => {
                // Array literal: [a, b, c] or array repeat: [value; count]
                self.advance();

                // Check for empty array []
                if self.current_token() == &Token::RBracket {
                    self.advance();
                    return Ok(self.alloc_expr(Expression::Array {
                        elements: vec![],
                        location: self.current_location(),
                    }));
                }

                let first_element = self.parse_expression()?;

                // Check for array repeat syntax: [value; count]
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                    let count = self.parse_expression()?;
                    self.expect(Token::RBracket)?;

                    // Represent as a macro invocation: vec![value; count]
                    return Ok(self.alloc_expr(Expression::MacroInvocation {
                        name: "vec".to_string(),
                        args: vec![first_element, count],
                        delimiter: MacroDelimiter::Brackets,
                        is_repeat: true, // This is vec![x; n] repeat syntax
                        location: self.current_location(),
                    }));
                }

                // Regular array literal
                let mut elements = vec![first_element];

                while self.current_token() == &Token::Comma {
                    self.advance(); // consume comma

                    // Allow trailing comma
                    if self.current_token() == &Token::RBracket {
                        break;
                    }

                    elements.push(self.parse_expression()?);
                }

                self.expect(Token::RBracket)?;
                self.alloc_expr(Expression::Array {
                    elements,
                    location: self.current_location(),
                })
            }
            Token::Ampersand => {
                // Handle & and &mut unary operators
                self.advance();
                let is_mut = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = self.parse_match_value()?;
                self.alloc_expr(Expression::Unary {
                    op: if is_mut {
                        UnaryOp::MutRef
                    } else {
                        UnaryOp::Ref
                    },
                    operand: inner,
                    location: self.current_location(),
                })
            }
            Token::Star => {
                // Handle * dereference operator
                self.advance();
                let inner = self.parse_match_value()?;
                self.alloc_expr(Expression::Unary {
                    op: UnaryOp::Deref,
                    operand: inner,
                    location: self.current_location(),
                })
            }
            Token::Minus => {
                // Handle - negation operator
                self.advance();
                let inner = self.parse_match_value()?;
                self.alloc_expr(Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: inner,
                    location: self.current_location(),
                })
            }
            Token::Bang => {
                // Handle ! not operator
                self.advance();
                let inner = self.parse_match_value()?;
                self.alloc_expr(Expression::Unary {
                    op: UnaryOp::Not,
                    operand: inner,
                    location: self.current_location(),
                })
            }
            Token::Ident(name) => {
                let mut qualified_name = name.clone();
                self.advance();

                // Handle qualified paths with :: (e.g., std::fs::read)
                while self.current_token() == &Token::ColonColon {
                    // Look ahead to see if there's an identifier after ::
                    if self.position + 1 < self.tokens.len() {
                        if let Token::Ident(next_name) = &self.tokens[self.position + 1].token {
                            // This is a qualified path segment
                            qualified_name.push_str("::");
                            qualified_name.push_str(next_name);
                            self.advance(); // consume ::
                            self.advance(); // consume identifier
                        } else if let Token::Lt = &self.tokens[self.position + 1].token {
                            // This is turbofish (e.g., Type::<T>), stop here
                            break;
                        } else {
                            // Unknown token after ::, stop here
                            break;
                        }
                    } else {
                        // No more tokens, stop
                        break;
                    }
                }

                // Don't check for { here - just create the identifier
                // and continue to postfix operators
                self.alloc_expr(Expression::Identifier {
                    name: qualified_name,
                    location: self.current_location(),
                })
            }
            _ => self.parse_primary_expression()?,
        };

        // Handle postfix operators (., [, etc.) before binary operators
        loop {
            match self.current_token() {
                Token::Dot => {
                    // Check for .await
                    if self.peek(1) == Some(&Token::Await) {
                        self.advance(); // consume '.'
                        self.advance(); // consume 'await'
                        left = self.alloc_expr(Expression::Await {
                            expr: left,
                            location: self.current_location(),
                        });
                    } else {
                        self.advance();
                        let field = match self.current_token() {
                            Token::Ident(name) => {
                                let name = name.clone();
                                self.advance();
                                name
                            }
                            Token::IntLiteral(n) | Token::IntLiteralSuffixed(n, _) => {
                                let field_name = n.to_string();
                                self.advance();
                                field_name
                            }
                            _ => {
                                return Err("Expected field or method name after .".to_string());
                            }
                        };
                        left = self.alloc_expr(Expression::FieldAccess {
                            object: left,
                            field,
                            location: self.current_location(),
                        });
                    }
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
                        // We need to compute end_expr without holding onto left
                        let len_call = self.alloc_expr(Expression::MethodCall {
                            object: left,
                            method: "len".to_string(),
                            type_args: None,
                            arguments: vec![],
                            location: self.current_location(),
                        });
                        let end_expr = end.unwrap_or(len_call);

                        let zero_lit = self.alloc_expr(Expression::Literal {
                            value: Literal::Int(0),
                            location: self.current_location(),
                        });

                        left = self.alloc_expr(Expression::MethodCall {
                            object: left,
                            method: "slice".to_string(),
                            type_args: None,
                            arguments: vec![(None, zero_lit), (None, end_expr)],
                            location: self.current_location(),
                        });
                    } else {
                        let start_or_index = self.parse_expression()?;

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
                                    object: left,
                                    method: "len".to_string(),
                                    type_args: None,
                                    arguments: vec![],
                                    location: self.current_location(),
                                })
                            });

                            left = self.alloc_expr(Expression::MethodCall {
                                object: left,
                                method: "slice".to_string(),
                                type_args: None,
                                arguments: vec![(None, start_or_index), (None, end_expr)],
                                location: self.current_location(),
                            });
                        } else {
                            // Regular index: [i]
                            self.expect(Token::RBracket)?;
                            left = self.alloc_expr(Expression::Index {
                                object: left,
                                index: start_or_index,
                                location: self.current_location(),
                            });
                        }
                    }
                }
                Token::ColonColon => {
                    // Handle turbofish and static method calls in match values
                    self.advance(); // consume ::

                    if self.current_token() == &Token::Lt {
                        // Turbofish: expr::<Type>
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

                        // Expect function call after turbofish
                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            left = self.alloc_expr(Expression::MethodCall {
                                object: left,
                                method: String::new(), // Empty method name signals turbofish call
                                type_args: Some(types),
                                arguments,
                                location: self.current_location(),
                            });
                        } else {
                            return Err("Expected '(' after turbofish".to_string());
                        }
                    } else if let Token::Ident(method) = self.current_token() {
                        // Static method or path continuation
                        let method = method.clone();
                        self.advance();

                        // Check for turbofish on this method
                        let type_args = if self.current_token() == &Token::ColonColon {
                            // Peek ahead to see if this is turbofish or path continuation
                            if self.peek(1) == Some(&Token::Lt) {
                                // Turbofish: Type::<T>
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
                                // Not turbofish - don't consume ::, let the loop handle it
                                None
                            }
                        } else {
                            None
                        };

                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            left = self.alloc_expr(Expression::MethodCall {
                                object: left,
                                method,
                                type_args,
                                arguments,
                                location: self.current_location(),
                            });
                        } else {
                            // Just a path, treat as field access
                            left = self.alloc_expr(Expression::FieldAccess {
                                object: left,
                                field: method,
                                location: self.current_location(),
                            });
                        }
                    } else {
                        return Err("Expected '<' or identifier after '::'".to_string());
                    }
                }
                Token::LParen => {
                    // Function call
                    self.advance();
                    let mut arguments = Vec::new();
                    while self.current_token() != &Token::RParen {
                        let arg = self.parse_expression()?;
                        arguments.push((None, arg));
                        if self.current_token() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    left = self.alloc_expr(Expression::Call {
                        function: left,
                        arguments,
                        location: self.current_location(),
                    });
                }
                _ => break,
            }
        }

        // Handle binary operators
        while let Some((op, precedence)) = self.get_binary_op() {
            self.advance();
            let right = self.parse_binary_expression(precedence + 1)?;
            left = self.alloc_expr(Expression::Binary {
                left,
                op,
                right,
                location: self.current_location(),
            });
        }

        Ok(left)
    }
}
