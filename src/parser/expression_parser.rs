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
        self.parse_ternary_expression()
    }

    pub(in crate::parser) fn parse_ternary_expression(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        // Ternary operator removed - use if/else expressions instead
        // This simplifies the parser and eliminates ambiguity with TryOp (?)
        self.parse_binary_expression(0)
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
                    self.advance(); // consume identifier
                    self.advance(); // consume colon
                    Some(label)
                } else {
                    None
                }
            } else {
                None
            };

            let expr = self.parse_expression()?;
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
