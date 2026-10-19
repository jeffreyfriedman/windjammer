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
        let left = if self.current_token() == &Token::Bang {
            self.advance();
            let inner = self.with_unary_operand(|p| p.parse_match_value_atom())?;
            self.alloc_expr(Expression::Unary {
                op: UnaryOp::Not,
                operand: inner,
                location: self.current_location(),
            })
        } else if self.current_token() == &Token::Minus {
            self.advance();
            let inner = self.with_unary_operand(|p| p.parse_match_value_atom())?;
            self.alloc_expr(Expression::Unary {
                op: UnaryOp::Neg,
                operand: inner,
                location: self.current_location(),
            })
        } else if self.current_token() == &Token::Star {
            self.advance();
            let inner = self.with_unary_operand(|p| p.parse_match_value_atom())?;
            self.alloc_expr(Expression::Unary {
                op: UnaryOp::Deref,
                operand: inner,
                location: self.current_location(),
            })
        } else {
            self.parse_match_value_atom_base()?
        };

        // Postfix operators: one implementation shared with ordinary expressions, so method
        // calls, turbofish (`s.parse::<int>()`), `?`, `as` and indexing cannot drift apart.
        // Only the *primary* differs here: an identifier never starts a struct literal.
        self.parse_postfix_expression(left)
    }

    fn parse_match_value_atom_base(&mut self) -> Result<&'static Expression<'static>, String> {
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
                let inner = self.with_unary_operand(|p| p.parse_match_value_atom())?;
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
        // Like `parse_binary_expression`: a full match value (e.g. inside parentheses) is never a
        // prefix operator's operand, so its own `as` casts stay inside.
        let saved = std::mem::replace(&mut self.in_unary_operand, false);
        let r = self.parse_match_value_inner();
        self.in_unary_operand = saved;
        r
    }

    fn parse_match_value_inner(&mut self) -> Result<&'static Expression<'static>, String> {
        let left = self.parse_match_value_prec(0)?;
        self.parse_pipe_stages(left)
    }

    /// Binary chain of a `match` scrutinee. Every operand is a scrutinee atom, so a name followed by `{`
    /// (`match a == b { .. }`) opens the match body, never a struct literal (as in Rust).
    fn parse_match_value_prec(&mut self, min_precedence: u8) -> Result<&'static Expression<'static>, String> {
        let mut left = self.parse_match_value_atom()?;
        loop {
            // ADR-0018 `x ?? d`: between comparison (7) and additive (8), right-associative.
            if self.current_token() == &Token::QuestionQuestion && min_precedence <= 7 {
                self.advance();
                let right = self.parse_match_coalesce_rhs()?;
                left = self.option_default(left, right);
                continue;
            }
            let Some((op, precedence)) = self.get_binary_op() else { break };
            if precedence < min_precedence {
                break;
            }
            self.advance();
            let right = self.parse_match_value_prec(precedence + 1)?;
            left = self.alloc_expr(Expression::Binary { left, op, right, location: self.current_location() });
        }
        Ok(left)
    }

    fn parse_match_coalesce_rhs(&mut self) -> Result<&'static Expression<'static>, String> {
        let right = self.parse_match_value_prec(8)?;
        if self.current_token() == &Token::QuestionQuestion {
            self.advance();
            let rest = self.parse_match_coalesce_rhs()?;
            return Ok(self.option_default(right, rest));
        }
        Ok(right)
    }
}
