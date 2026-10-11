// Binary expressions, channel send, and precedence helpers.

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

    /// A `*` that opens a line and whose line contains an assignment (`*g += 1`, `*r = x`) starts a
    /// new deref-assignment statement; it is not a multiplication continuing the previous line.
    fn star_starts_assignment_statement(&self) -> bool {
        let Some(cur) = self.tokens.get(self.position) else { return false };
        let Some(prev) = self.position.checked_sub(1).and_then(|i| self.tokens.get(i)) else { return false };
        if prev.line >= cur.line {
            return false;
        }
        self.tokens[self.position + 1..].iter().take_while(|t| t.line == cur.line).any(|t| {
            matches!(
                t.token,
                Token::Assign
                    | Token::PlusAssign
                    | Token::MinusAssign
                    | Token::StarAssign
                    | Token::SlashAssign
                    | Token::PercentAssign
            )
        })
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
            // Windjammer binds shifts tighter than additive (unlike Rust/C),
            // so `a + b << c` means `a + (b << c)`. Rust codegen must parenthesize.
            Token::Plus => Some((BinaryOp::Add, 8)),
            Token::Minus => Some((BinaryOp::Sub, 8)),
            Token::Shl => Some((BinaryOp::Shl, 9)), // Shift left
            Token::Shr => Some((BinaryOp::Shr, 9)), // Shift right
            Token::Star if self.star_starts_assignment_statement() => None,
            Token::Star => Some((BinaryOp::Mul, 10)),
            Token::Slash => Some((BinaryOp::Div, 10)),
            Token::Percent => Some((BinaryOp::Mod, 10)),
            _ => None,
        }
    }
}
