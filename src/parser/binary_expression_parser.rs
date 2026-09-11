// Binary expressions, pipe (`|>`), channel send, and precedence helpers.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn parse_binary_expression(
        &mut self,
        min_precedence: u8,
    ) -> Result<&'static Expression<'static>, String> {
        let mut left = self.parse_primary_expression()?;

        loop {
            // Check for pipe operator: value |> func  or  value |> f(a, b) -> f(value, a, b)
            if self.current_token() == &Token::PipeOp {
                self.advance();

                // Parse the right side (function or call expression)
                let func = self.parse_primary_expression()?;

                left = match func {
                    Expression::Call {
                        function,
                        arguments,
                        location,
                    } => {
                        let mut new_args = arguments.clone();
                        new_args.insert(0, (None, left));
                        self.alloc_expr(Expression::Call {
                            function,
                            arguments: new_args,
                            location: location.clone(),
                        })
                    }
                    _ => self.alloc_expr(Expression::Call {
                        function: func,
                        arguments: vec![(None, left)],
                        location: self.current_location(),
                    }),
                };
                continue;
            }

            // Check for channel send: ch <- value
            if self.current_token() == &Token::LeftArrow {
                self.advance();
                let value = self.parse_expression()?;
                left = self.alloc_expr(Expression::ChannelSend {
                    channel: left,
                    value,
                    location: self.current_location(),
                });
                continue;
            }

            if let Some((op, precedence)) = self.get_binary_op() {
                if precedence < min_precedence {
                    break;
                }

                self.advance();
                let right = self.parse_binary_expression(precedence + 1)?;

                left = self.alloc_expr(Expression::Binary {
                    left,
                    op,
                    right,
                    location: self.current_location(),
                });
            } else {
                break;
            }
        }

        Ok(left)
    }

    pub(in crate::parser) fn get_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.current_token() {
            Token::Or => Some((BinaryOp::Or, 1)),            // Logical OR
            Token::And => Some((BinaryOp::And, 2)),          // Logical AND
            Token::Pipe => Some((BinaryOp::BitOr, 3)),       // Bitwise OR
            Token::Caret => Some((BinaryOp::BitXor, 4)),     // Bitwise XOR
            Token::Ampersand => Some((BinaryOp::BitAnd, 5)), // Bitwise AND
            Token::Eq => Some((BinaryOp::Eq, 6)),
            Token::Ne => Some((BinaryOp::Ne, 6)),
            Token::Lt => Some((BinaryOp::Lt, 7)),
            Token::Le => Some((BinaryOp::Le, 7)),
            Token::Gt => Some((BinaryOp::Gt, 7)),
            Token::Ge => Some((BinaryOp::Ge, 7)),
            // WDB-153: Windjammer binds shifts tighter than additive (unlike Rust/C),
            // so `a + b << c` means `a + (b << c)`. Rust codegen must parenthesize.
            Token::Plus => Some((BinaryOp::Add, 8)),
            Token::Minus => Some((BinaryOp::Sub, 8)),
            Token::Shl => Some((BinaryOp::Shl, 9)), // Shift left
            Token::Shr => Some((BinaryOp::Shr, 9)), // Shift right
            Token::Star => Some((BinaryOp::Mul, 10)),
            Token::Slash => Some((BinaryOp::Div, 10)),
            Token::Percent => Some((BinaryOp::Mod, 10)),
            _ => None,
        }
    }
}
