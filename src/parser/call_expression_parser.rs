// Postfix operators: calls, method calls, indexing, slicing, casts, macros.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    /// Where the postfix chain must stop (docs/design/statement-boundaries.md): `(` or `[` that
    /// start a new line begin a new statement; `as` after a prefix operator's operand applies to
    /// the whole prefix expression (`-1 as u8` is `(-1) as u8`, as in Rust).
    fn postfix_stops_here(&self, stop_at_as: bool) -> bool {
        match self.current_token() {
            Token::LParen | Token::LBracket => self.had_newline_before_current(),
            Token::As => stop_at_as,
            _ => false,
        }
    }

    /// Postfix chain. Whether `as` ends it is decided by *this* chain's context only: expressions
    /// nested inside it (index, arguments, closure bodies) start with the flag cleared.
    pub(in crate::parser) fn parse_postfix_expression(
        &mut self,
        expr: &'static Expression<'static>,
    ) -> Result<&'static Expression<'static>, String> {
        let stop_at_as = std::mem::replace(&mut self.in_unary_operand, false);
        let r = self.parse_postfix_chain(expr, stop_at_as);
        self.in_unary_operand = stop_at_as;
        r
    }

    fn parse_postfix_chain(
        &mut self,
        mut expr: &'static Expression<'static>,
        stop_at_as: bool,
    ) -> Result<&'static Expression<'static>, String> {
        loop {
            if self.postfix_stops_here(stop_at_as) {
                break;
            }
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
                            Token::String => Some("string".to_string()),
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
                                    // ASI: `(` on a new line starts a statement; keep the field access
                                    self.alloc_expr(Expression::FieldAccess {
                                        object: expr,
                                        field,
                                        location: self.current_location(),
                                    })
                                } else {
                                    // Method call (possibly with turbofish)
                                    self.advance();
                                    let arguments = self.parse_call_rest(Some(format!(".{field}")))?;
                                    if let Some(lowered) =
                                        self.lower_stream_method_call(expr, &field, &arguments)
                                    {
                                        lowered
                                    } else {
                                        self.alloc_expr(Expression::MethodCall {
                                            object: expr,
                                            method: field,
                                            type_args,
                                            arguments,
                                            location: self.current_location(),
                                        })
                                    }
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
                                "Expected field or method name (at {})",
                                self.pos_desc()
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
                            let key = crate::parser::named_args::call_key(expr).map(|t| format!("{t}::{method}"));
                            let arguments = self.parse_call_rest(key)?;
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
                    self.advance();
                    let arguments = self.parse_call_rest(crate::parser::named_args::call_key(expr))?;
                    self.alloc_expr(Expression::Call {
                        function: expr,
                        arguments,
                        location: self.current_location(),
                    })
                }
                Token::QuestionQuestion if self.double_try_follows() => self.double_try(expr),
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
                    if let Some(open) = self.try_parse_range_from(expr) {
                        expr = open;
                        continue;
                    }
                    self.advance();
                    // P3.280: range end must be a full expression so
                    // `(cx - 16)..(cx + 16)` and `a..b + c` parse correctly.
                    // Previously only `parse_primary_expression` ran, dropping
                    // binary ops on the end bound unless parenthesized.
                    let end = self.parse_binary_expression(0)?;
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
                    // Do not treat `ident !expr` across statement boundaries as a macro
                    // (`let _ = name` / `!false` must leave Bang for unary-not).
                    if let Expression::Identifier { name, .. } = expr {
                        let is_macro_call = matches!(
                            self.peek(1),
                            Some(Token::LParen | Token::LBracket | Token::LBrace)
                        );
                        if !is_macro_call {
                            break;
                        }
                        self.advance(); // consume '!'

                        let (delimiter, end_token) = match self.current_token() {
                            Token::LParen => (MacroDelimiter::Parens, Token::RParen),
                            Token::LBracket => (MacroDelimiter::Brackets, Token::RBracket),
                            Token::LBrace => (MacroDelimiter::Braces, Token::RBrace),
                            _ => return Err("Expected (, [, or { after macro name!".to_string()),
                        };

                        self.advance(); // consume opening delimiter

                        if name == "matches" && delimiter == MacroDelimiter::Parens {
                            expr = self.parse_matches_macro()?;
                            continue;
                        }

                        let mut args = Vec::new();

                        // Check for empty macro: vec![], println!()
                        let mut is_repeat_flag = false;
                        if self.current_token() != &end_token {
                            if name == "thread_local" && delimiter == MacroDelimiter::Braces {
                                let mut statements = Vec::new();
                                while self.current_token() != &end_token {
                                    statements.push(self.parse_statement()?);
                                }
                                args.push(self.alloc_expr(Expression::Block {
                                    statements,
                                    is_unsafe: false,
                                    location: self.current_location(),
                                }));
                            } else {
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
                        }

                        self.expect(end_token)?;

                        self.warn_if_format_macro(name);

                        if let Some(lowered) = self.lower_intrinsic_macro(name, &args) {
                            lowered
                        } else {
                            self.alloc_expr(Expression::MacroInvocation {
                                name: name.clone(),
                                args,
                                delimiter,
                                is_repeat: is_repeat_flag,
                                location: self.current_location(),
                            })
                        }
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
