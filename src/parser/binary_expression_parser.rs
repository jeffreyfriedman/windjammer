// Binary expressions, channel send, and precedence helpers.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn parse_binary_expression(
        &mut self,
        min_precedence: u8,
    ) -> Result<&'static Expression<'static>, String> {
        // A (sub)expression parsed here is never itself a prefix operator's operand, whatever
        // context it is nested in (parentheses, index, arguments): `-(a as u8)` keeps its cast.
        let saved = std::mem::replace(&mut self.in_unary_operand, false);
        let r = self.parse_binary_expression_inner(min_precedence);
        self.in_unary_operand = saved;
        r
    }

    fn parse_binary_expression_inner(
        &mut self,
        min_precedence: u8,
    ) -> Result<&'static Expression<'static>, String> {
        let mut left = self.parse_prefixed_or_primary()?;

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

            // ADR-0018 `x ?? d`: right-associative, between comparison (7) and additive (8).
            if self.current_token() == &Token::QuestionQuestion && min_precedence <= 7 {
                self.advance();
                let right = self.parse_coalesce_rhs()?;
                left = self.option_default(left, right);
                continue;
            }

            if let Some((op, precedence)) = self.get_binary_op() {
                if precedence < min_precedence {
                    break;
                }
                // Statement-boundary rule (docs/design/statement-boundaries.md): a `*` that
                // starts a new line and begins a deref-assignment (`*p = v`, `*p += 1`) is a
                // new statement, not multiplication continuing the previous line.
                if self.current_token() == &Token::Star
                    && self.had_newline_before_current()
                    && self.line_is_deref_assignment()
                {
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

    /// True when the tokens from the current `*` to the end of its line contain an assignment
    /// operator at bracket depth 0. A binary-operator continuation can never contain one, so
    /// the test cannot misclassify a genuine `a\n  * b` continuation.
    fn line_is_deref_assignment(&self) -> bool {
        let Some(first) = self.tokens.get(self.position) else {
            return false;
        };
        let line = first.line;
        let mut depth: i32 = 0;
        for t in self.tokens[self.position..].iter().take_while(|t| t.line == line) {
            match t.token {
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => depth -= 1,
                Token::Assign
                | Token::PlusAssign
                | Token::MinusAssign
                | Token::StarAssign
                | Token::SlashAssign
                | Token::PercentAssign
                | Token::AndAssign
                | Token::OrAssign
                | Token::XorAssign
                | Token::ShlAssign
                | Token::ShrAssign
                    if depth == 0 =>
                {
                    return true;
                }
                _ => {}
            }
            if depth < 0 {
                return false;
            }
        }
        false
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
            Token::Star => Some((BinaryOp::Mul, 10)),
            Token::Slash => Some((BinaryOp::Div, 10)),
            Token::Percent => Some((BinaryOp::Mod, 10)),
            _ => None,
        }
    }

    /// The right operand of `??`: an additive-level expression, itself possibly `b ?? c` (right-associative).
    fn parse_coalesce_rhs(&mut self) -> Result<&'static Expression<'static>, String> {
        let right = self.parse_binary_expression(8)?;
        if self.current_token() == &Token::QuestionQuestion {
            self.advance();
            let rest = self.parse_coalesce_rhs()?;
            return Ok(self.option_default(right, rest));
        }
        Ok(right)
    }

    /// `x ?? d` is `x.unwrap_or_else(|| d)`: `d` is evaluated only when `x` is `None`.
    fn option_default(
        &mut self,
        value: &'static Expression<'static>,
        default: &'static Expression<'static>,
    ) -> &'static Expression<'static> {
        let location = value.location();
        // A literal or a variable cannot have effects, so the eager `unwrap_or` is equivalent (and gets the
        // literal coercions every backend already applies to `unwrap_or`); anything else stays lazy.
        let trivial = matches!(default, Expression::Literal { .. } | Expression::Identifier { .. });
        let (method, arg) = if trivial {
            ("unwrap_or", default)
        } else {
            let closure =
                self.alloc_expr(Expression::Closure { parameters: vec![], body: default, location: location.clone() });
            ("unwrap_or_else", closure)
        };
        self.alloc_expr(Expression::MethodCall {
            object: value,
            method: method.to_string(),
            type_args: None,
            arguments: vec![(None, arg)],
            location,
        })
    }
}
