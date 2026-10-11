// `matches!(expr, pattern [if guard])` takes a *pattern* as its second operand, which the generic
// macro-argument parser (expressions only) cannot read. Lower it here to the equivalent
//   match expr { pattern [if guard] => true, _ => false }
// so every backend sees an ordinary match.

use crate::parser::ast::*;
use crate::parser_impl::Parser;
use crate::lexer::Token;

impl Parser {
    /// Called with the opening `(` already consumed; consumes through the closing `)`.
    pub(in crate::parser) fn parse_matches_macro(&mut self) -> Result<&'static Expression<'static>, String> {
        let value = self.parse_expression()?;
        self.expect(Token::Comma)?;
        let pattern = self.parse_pattern_with_or()?;
        let guard = if self.current_token() == &Token::If {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        if self.current_token() == &Token::Comma {
            self.advance();
        }
        self.expect(Token::RParen)?;
        let lit = |this: &Self, b: bool| {
            this.alloc_expr(Expression::Literal { value: Literal::Bool(b), location: this.current_location() })
        };
        let arms = vec![
            MatchArm { pattern, guard, body: lit(self, true) },
            MatchArm { pattern: Pattern::Wildcard, guard: None, body: lit(self, false) },
        ];
        let stmt = self.alloc_stmt(Statement::Match { value, arms, location: self.current_location() });
        Ok(self.alloc_expr(Expression::Block {
            statements: vec![stmt],
            is_unsafe: false,
            location: self.current_location(),
        }))
    }
}
