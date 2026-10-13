// Expression parsing entry points and shared helpers (`parse_arguments`, `peek`).
//
// The `impl Parser` expression logic is split across sibling modules (all `impl Parser`):
// - `binary_expression_parser` — precedence / binary and postfix chain bootstrap
// - `call_expression_parser` — postfix chain: calls, field/method access, index, slice, `?`, `as`, macros
// - `compound_primary_expression_parser` — `match`, closures, `if` / `if let`, `unsafe` as primaries
// - `primary_expression_parser` — literals, paths, tuples/arrays, blocks, remaining primaries
// - `interpolated_string_expression_parser` — string interpolation
// - `match_value_expression_parser` — match-arm values (no struct literals / assignment)

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(crate) fn parse_expression(&mut self) -> Result<&'static Expression<'static>, String> {
        // A full expression (parenthesised, argument, index...) is never itself a unary operand.
        let saved = std::mem::replace(&mut self.in_unary_operand, false);
        let r = self.parse_ternary_expression();
        self.in_unary_operand = saved;
        r
    }

    /// Operand of a prefix operator (`-`, `!`, `*`, `&`). Postfix operators bind tighter than
    /// prefix ones except `as`, which binds looser (Rust): `-1 as u8` is `(-1) as u8`,
    /// `-x.abs()` is `-(x.abs())`.
    pub(in crate::parser) fn parse_unary_operand(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        self.with_unary_operand(|p| p.parse_primary_expression())
    }

    pub(in crate::parser) fn with_unary_operand<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        let saved = std::mem::replace(&mut self.in_unary_operand, true);
        let r = f(self);
        self.in_unary_operand = saved;
        r
    }

    pub(in crate::parser) fn parse_ternary_expression(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        // Ternary operator removed - use if/else expressions instead
        // This simplifies the parser and eliminates ambiguity with TryOp (?)
        self.parse_pipe_expression()
    }

    pub(crate) fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset).map(|t| &t.token)
    }

    pub(in crate::parser) fn parse_arguments(
        &mut self,
    ) -> Result<Vec<(Option<String>, &'static Expression<'static>)>, String> {
        let mut args = Vec::new();

        while self.current_token() != &Token::RParen {
            // Check for labeled argument: name: expr
            let label = if let Token::Ident(name) = self.current_token() {
                if self.peek(1) == Some(&Token::Colon) {
                    let label = name.clone();
                    self.saw_label = true;
                    self.advance(); // consume identifier
                    self.advance(); // consume colon
                    Some(label)
                } else {
                    None
                }
            } else {
                None
            };

            let expr = self.parse_argument_with_shorthand()?;
            args.push((label, expr));

            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }
}
