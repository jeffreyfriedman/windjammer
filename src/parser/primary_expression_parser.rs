// Primary expressions: literals, calls, closures, macros, indexing, structs, etc.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn parse_primary_expression(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        let expr = match self.current_token() {
            Token::Thread => {
                // Check if this is a thread block or a module path
                if self.peek(1) == Some(&Token::LBrace) {
                    // Thread block: thread { ... }
                    self.advance();
                    self.expect(Token::LBrace)?;
                    let body = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    // Wrap in a statement expression
                    let thread_stmt = self.alloc_stmt(Statement::Thread {
                        body,
                        location: self.current_location(),
                    });
                    self.alloc_expr(Expression::Block {
                        statements: vec![thread_stmt],
                        is_unsafe: false,
                        location: self.current_location(),
                    })
                } else {
                    // Module path like thread::sleep_seconds
                    // Parse as identifier and let postfix operators handle ::
                    let name = "thread".to_string();
                    self.advance();
                    self.alloc_expr(Expression::Identifier {
                        name,
                        location: self.current_location(),
                    })
                }
            }
            Token::Async => {
                // Check if this is an async block or a module path
                if self.peek(1) == Some(&Token::LBrace) {
                    // Async block: async { ... }
                    self.advance();
                    self.expect(Token::LBrace)?;
                    let body = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    // Wrap in a statement expression
                    let async_stmt = self.alloc_stmt(Statement::Async {
                        body,
                        location: self.current_location(),
                    });
                    self.alloc_expr(Expression::Block {
                        statements: vec![async_stmt],
                        is_unsafe: false,
                        location: self.current_location(),
                    })
                } else {
                    // Module path like async::something
                    let name = "async".to_string();
                    self.advance();
                    self.alloc_expr(Expression::Identifier {
                        name,
                        location: self.current_location(),
                    })
                }
            }
            Token::LeftArrow => {
                // Channel receive: <-ch
                self.advance();
                let channel = self.parse_primary_expression()?;
                self.alloc_expr(Expression::ChannelRecv {
                    channel,
                    location: self.current_location(),
                })
            }
            Token::Ampersand => {
                // Reference: &expr or &mut expr
                self.advance();
                let is_mut = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let operand = self.parse_primary_expression()?;
                self.alloc_expr(Expression::Unary {
                    op: if is_mut {
                        UnaryOp::MutRef
                    } else {
                        UnaryOp::Ref
                    },
                    operand,
                    location: self.current_location(),
                })
            }
            Token::Star => {
                // Dereference: *expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                self.alloc_expr(Expression::Unary {
                    op: UnaryOp::Deref,
                    operand,
                    location: self.current_location(),
                })
            }
            Token::Minus => {
                // Negation: -expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                self.alloc_expr(Expression::Unary {
                    op: UnaryOp::Neg,
                    operand,
                    location: self.current_location(),
                })
            }
            Token::Bang => {
                // Logical not: !expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                self.alloc_expr(Expression::Unary {
                    op: UnaryOp::Not,
                    operand,
                    location: self.current_location(),
                })
            }
            Token::Self_ => {
                // self keyword used in expressions
                self.advance();
                self.alloc_expr(Expression::Identifier {
                    name: "self".to_string(),
                    location: self.current_location(),
                })
            }
            Token::IntLiteral(n) => {
                let n = *n;
                let loc = self.current_location();
                self.advance();
                self.alloc_expr(Expression::Literal {
                    value: Literal::Int(n),
                    location: loc,
                })
            }
            Token::IntLiteralSuffixed(n, ref suffix) => {
                let n = *n;
                let suffix = suffix.clone();
                let loc = self.current_location();
                self.advance();
                self.alloc_expr(Expression::Literal {
                    value: Literal::IntSuffixed(n, suffix),
                    location: loc,
                })
            }
            Token::FloatLiteral(f) => {
                let f = *f;
                let loc = self.current_location(); // Capture BEFORE advance - critical for float type inference
                self.advance();
                self.alloc_expr(Expression::Literal {
                    value: Literal::Float(f),
                    location: loc,
                })
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                let loc = self.current_location();
                self.advance();
                self.alloc_expr(Expression::Literal {
                    value: Literal::String(s),
                    location: loc,
                })
            }
            Token::CharLiteral(c) => {
                let c = *c;
                let loc = self.current_location();
                self.advance();
                self.alloc_expr(Expression::Literal {
                    value: Literal::Char(c),
                    location: loc,
                })
            }
            Token::InterpolatedString(parts) => {
                let parts = parts.clone();
                self.advance();
                self.finish_interpolated_string(parts)?
            }
            Token::BoolLiteral(b) => {
                let b = *b;
                self.advance();
                self.alloc_expr(Expression::Literal {
                    value: Literal::Bool(b),
                    location: self.current_location(),
                })
            }
            Token::Ident(name) => {
                let mut qualified_name = name.clone();
                self.advance();

                // Handle qualified paths with :: (e.g., sqlx::SqlitePool, std::fs::File)
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

                // Check for struct literal
                // Only parse as struct literal if the name looks like a type (starts with uppercase)
                // AND the next tokens look like struct literal syntax (field: value or field,)
                // This avoids ambiguity in contexts like "for item in items { ... }"
                // For qualified names (e.g., "ffi::GpuVertex"), check the LAST component
                let last_component = qualified_name.split("::").last().unwrap_or(&qualified_name);
                let looks_like_type = last_component
                    .chars()
                    .next()
                    .is_some_and(|c: char| c.is_uppercase());

                let looks_like_struct_literal =
                    if looks_like_type && self.current_token() == &Token::LBrace {
                        // Lookahead: check if the first token after { looks like a field name
                        // followed by : or , or }
                        if self.position + 1 < self.tokens.len() {
                            match &self.tokens[self.position + 1].token {
                                Token::Ident(_) | Token::RBrace => {
                                    // Could be struct literal: { field: ... } or { field, ... } or { }
                                    if self.position + 2 < self.tokens.len() {
                                        matches!(
                                            &self.tokens[self.position + 2].token,
                                            Token::Colon | Token::Comma | Token::RBrace
                                        )
                                    } else {
                                        true
                                    }
                                }
                                _ => false,
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                if looks_like_struct_literal {
                    self.advance();
                    let mut fields = Vec::new();

                    while self.current_token() != &Token::RBrace {
                        // Allow identifiers or keywords as field names
                        let field_name = match self.current_token() {
                            Token::Ident(name) => name.clone(),
                            Token::Async => "async".to_string(),
                            Token::Thread => "thread".to_string(),
                            Token::Type => "type".to_string(),
                            Token::Self_ => "self".to_string(),
                            tok => {
                                return Err(format!("Expected field name in struct literal, got {:?} at position {}", tok, self.position));
                            }
                        };
                        self.advance();

                        let field_value = if self.current_token() == &Token::Colon {
                            // Regular syntax: field: value
                            self.advance();
                            self.parse_expression()?
                        } else {
                            // Shorthand syntax: field (implicitly field: field)
                            self.alloc_expr(Expression::Identifier {
                                name: field_name.clone(),
                                location: self.current_location(),
                            })
                        };

                        fields.push((field_name, field_value));

                        if self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RBrace {
                                break;
                            }
                        } else if self.current_token() != &Token::RBrace {
                            return Err(
                                    format!("Expected comma or closing brace in struct literal, got {:?} at position {}", self.current_token(), self.position)
                                );
                        }
                    }

                    self.expect(Token::RBrace)?;
                    self.alloc_expr(Expression::StructLiteral {
                        name: qualified_name,
                        fields,
                        location: self.current_location(),
                    })
                } else {
                    self.alloc_expr(Expression::Identifier {
                        name: qualified_name,
                        location: self.current_location(),
                    })
                }
            }
            Token::LParen => {
                self.advance();

                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    self.alloc_expr(Expression::Tuple {
                        elements: vec![],
                        location: self.current_location(),
                    })
                } else {
                    let first_expr = self.parse_expression()?;

                    // Check if this is a tuple or just a parenthesized expression
                    if self.current_token() == &Token::Comma {
                        // It's a tuple
                        let mut exprs = vec![first_expr];

                        while self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RParen {
                                break;
                            }
                            exprs.push(self.parse_expression()?);
                        }

                        self.expect(Token::RParen)?;
                        self.alloc_expr(Expression::Tuple {
                            elements: exprs,
                            location: self.current_location(),
                        })
                    } else {
                        // Just a parenthesized expression
                        self.expect(Token::RParen)?;
                        first_expr
                    }
                }
            }
            Token::LBracket => {
                // Array literal: [a, b, c] or array repeat: [value; count]
                self.advance();

                // Check for empty array []
                if self.current_token() == &Token::RBracket {
                    self.advance();
                    self.alloc_expr(Expression::Array {
                        elements: vec![],
                        location: self.current_location(),
                    })
                } else {
                    let first_element = self.parse_expression()?;

                    // Check for array repeat syntax: [value; count]
                    if self.current_token() == &Token::Semicolon {
                        self.advance();
                        let count = self.parse_expression()?;
                        self.expect(Token::RBracket)?;

                        // Represent as a macro invocation: vec![value; count]
                        self.alloc_expr(Expression::MacroInvocation {
                            name: "vec".to_string(),
                            args: vec![first_element, count],
                            delimiter: MacroDelimiter::Brackets,
                            is_repeat: true, // This is vec![x; n] repeat syntax
                            location: self.current_location(),
                        })
                    } else {
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
                }
            }
            Token::Match => self.parse_primary_match()?,
            Token::Pipe => self.parse_primary_closure_pipe()?,
            Token::Or => self.parse_primary_closure_or()?,
            Token::If => self.parse_primary_if()?,
            Token::Unsafe => self.parse_primary_unsafe_block()?,
            Token::LBrace => {
                // Could be block expression or map literal
                // Disambiguate by looking ahead:
                // - { key: value } → map literal
                // - { stmt; stmt } → block
                self.advance(); // consume '{'

                // Check for empty braces
                if self.current_token() == &Token::RBrace {
                    self.advance();
                    // Empty block (not empty map - use HashMap::new() or map{} for that)
                    return Ok(self.alloc_expr(Expression::Block {
                        statements: vec![],
                        is_unsafe: false,
                        location: self.current_location(),
                    }));
                }

                // Try to detect map literal by parsing first item
                // Save position in case we need to backtrack
                let checkpoint = self.position;

                // Try parsing as map literal first
                let is_map = if let Ok(_key) = self.parse_ternary_expression() {
                    // If followed by ':', it's a map literal
                    self.current_token() == &Token::Colon
                } else {
                    false
                };

                // Restore position
                self.position = checkpoint;

                if is_map {
                    // Parse as map literal
                    let mut pairs = vec![];

                    loop {
                        if self.current_token() == &Token::RBrace {
                            break;
                        }

                        let key = self.parse_ternary_expression()?;
                        self.expect(Token::Colon)?;
                        let value = self.parse_expression()?;

                        pairs.push((key, value));

                        if self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RBrace {
                                break;
                            }
                        } else if self.current_token() != &Token::RBrace {
                            return Err("Expected ',' or '}' in map literal".to_string());
                        }
                    }

                    self.expect(Token::RBrace)?;
                    self.alloc_expr(Expression::MapLiteral {
                        pairs,
                        location: self.current_location(),
                    })
                } else {
                    // Parse as block expression
                    let body = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    self.alloc_expr(Expression::Block {
                        statements: body,
                        is_unsafe: false,
                        location: self.current_location(),
                    })
                }
            }
            Token::Return => {
                // Return expression: return expr
                self.advance(); // consume 'return'
                let return_value = if matches!(
                    self.current_token(),
                    Token::Comma | Token::RBrace | Token::Semicolon
                ) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                // Wrap in a block with a return statement
                let return_stmt = self.alloc_stmt(Statement::Return {
                    value: return_value,
                    location: self.current_location(),
                });
                self.alloc_expr(Expression::Block {
                    statements: vec![return_stmt],
                    is_unsafe: false,
                    location: self.current_location(),
                })
            }
            // Allow certain keywords as identifiers in expression context (e.g., HTML attributes)
            Token::For => {
                self.advance();
                self.alloc_expr(Expression::Identifier {
                    name: "for".to_string(),
                    location: self.current_location(),
                })
            }
            Token::Type => {
                self.advance();
                self.alloc_expr(Expression::Identifier {
                    name: "type".to_string(),
                    location: self.current_location(),
                })
            }
            _ => {
                eprintln!(
                    "DEBUG: Unexpected token at position {}: {:?}",
                    self.position,
                    self.current_token()
                );
                if self.position > 0 {
                    eprintln!(
                        "DEBUG: Previous token: {:?}",
                        self.tokens.get(self.position - 1)
                    );
                }
                if self.position > 1 {
                    eprintln!(
                        "DEBUG: Token before that: {:?}",
                        self.tokens.get(self.position - 2)
                    );
                }
                return Err(format!(
                    "Unexpected token in expression: {:?} (at token position {})",
                    self.current_token(),
                    self.position
                ));
            }
        };

        self.parse_postfix_expression(expr)
    }
}
