// Match-value expression parsing (patterns / match arms), without struct-literal path.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    /// Prefix + postfix atom for match/if-expr conditions — no `&&` / `||` absorption.
    ///
    /// `!row.flag && row.n != 0` must parse as `(!row.flag) && (row.n != 0)`, not
    /// `!(row.flag && row.n != 0)`. Unary `!`/`-`/`*` operands therefore stop before
    /// the binary-operator loop (same binding as `parse_primary_expression` + postfix).
    pub(in crate::parser) fn parse_match_value_atom(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        let mut left = if self.current_token() == &Token::Bang {
            self.advance();
            let inner = self.parse_match_value_atom()?;
            self.alloc_expr(Expression::Unary {
                op: UnaryOp::Not,
                operand: inner,
                location: self.current_location(),
            })
        } else if self.current_token() == &Token::Minus {
            self.advance();
            let inner = self.parse_match_value_atom()?;
            self.alloc_expr(Expression::Unary {
                op: UnaryOp::Neg,
                operand: inner,
                location: self.current_location(),
            })
        } else if self.current_token() == &Token::Star {
            self.advance();
            let inner = self.parse_match_value_atom()?;
            self.alloc_expr(Expression::Unary {
                op: UnaryOp::Deref,
                operand: inner,
                location: self.current_location(),
            })
        } else {
            self.parse_match_value_atom_base()?
        };

        // Postfix operators (., [, ::, () — same as full match-value parser).
        loop {
            match self.current_token() {
                Token::Dot => {
                    if self.peek(1) == Some(&Token::Await) {
                        self.advance();
                        self.advance();
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
                    if self.current_token() == &Token::DotDot {
                        self.advance();
                        let end = if self.current_token() != &Token::RBracket {
                            Some(self.parse_expression()?)
                        } else {
                            None
                        };
                        self.expect(Token::RBracket)?;
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
                        if self.current_token() == &Token::DotDot {
                            self.advance();
                            let end = if self.current_token() != &Token::RBracket {
                                Some(self.parse_expression()?)
                            } else {
                                None
                            };
                            self.expect(Token::RBracket)?;
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
                        self.expect_gt_or_split_shr()?;
                        if self.current_token() == &Token::LParen {
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            left = self.alloc_expr(Expression::MethodCall {
                                object: left,
                                method: String::new(),
                                type_args: Some(types),
                                arguments,
                                location: self.current_location(),
                            });
                        } else {
                            return Err("Expected '(' after turbofish".to_string());
                        }
                    } else if let Token::Ident(method) = self.current_token() {
                        let method = method.clone();
                        self.advance();
                        let type_args = if self.current_token() == &Token::ColonColon {
                            if self.peek(1) == Some(&Token::Lt) {
                                self.advance();
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
                                self.expect_gt_or_split_shr()?;
                                Some(types)
                            } else {
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

        Ok(left)
    }

    fn parse_match_value_atom_base(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        match self.current_token() {
            Token::LParen => {
                self.advance();
                if self.current_token() == &Token::RParen {
                    self.advance();
                    return Ok(self.alloc_expr(Expression::Tuple {
                        elements: vec![],
                        location: self.current_location(),
                    }));
                }
                let first_expr = self.parse_match_value()?;
                if self.current_token() == &Token::Comma {
                    let mut elements = vec![first_expr];
                    while self.current_token() == &Token::Comma {
                        self.advance();
                        if self.current_token() == &Token::RParen {
                            break;
                        }
                        elements.push(self.parse_match_value()?);
                    }
                    self.expect(Token::RParen)?;
                    Ok(self.alloc_expr(Expression::Tuple {
                        elements,
                        location: self.current_location(),
                    }))
                } else {
                    self.expect(Token::RParen)?;
                    Ok(first_expr)
                }
            }
            Token::LBracket => {
                self.advance();
                if self.current_token() == &Token::RBracket {
                    self.advance();
                    return Ok(self.alloc_expr(Expression::Array {
                        elements: vec![],
                        location: self.current_location(),
                    }));
                }
                let first_element = self.parse_expression()?;
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                    let count = self.parse_expression()?;
                    self.expect(Token::RBracket)?;
                    return Ok(self.alloc_expr(Expression::MacroInvocation {
                        name: "vec".to_string(),
                        args: vec![first_element, count],
                        delimiter: MacroDelimiter::Brackets,
                        is_repeat: true,
                        location: self.current_location(),
                    }));
                }
                let mut elements = vec![first_element];
                while self.current_token() == &Token::Comma {
                    self.advance();
                    if self.current_token() == &Token::RBracket {
                        break;
                    }
                    elements.push(self.parse_expression()?);
                }
                self.expect(Token::RBracket)?;
                Ok(self.alloc_expr(Expression::Array {
                    elements,
                    location: self.current_location(),
                }))
            }
            Token::Ampersand => {
                self.advance();
                let is_mut = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let inner = self.parse_match_value_atom()?;
                Ok(self.alloc_expr(Expression::Unary {
                    op: if is_mut {
                        UnaryOp::MutRef
                    } else {
                        UnaryOp::Ref
                    },
                    operand: inner,
                    location: self.current_location(),
                }))
            }
            Token::Ident(name) => {
                let mut qualified_name = name.clone();
                self.advance();
                while self.current_token() == &Token::ColonColon {
                    if self.position + 1 < self.tokens.len() {
                        if let Token::Ident(next_name) = &self.tokens[self.position + 1].token {
                            qualified_name.push_str("::");
                            qualified_name.push_str(next_name);
                            self.advance();
                            self.advance();
                        } else if let Token::Lt = &self.tokens[self.position + 1].token {
                            break;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                Ok(self.alloc_expr(Expression::Identifier {
                    name: qualified_name,
                    location: self.current_location(),
                }))
            }
            _ => self.parse_primary_expression(),
        }
    }

    pub(in crate::parser) fn parse_match_value(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        let mut left = self.parse_match_value_atom()?;

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
