// MODULE: guard-only `match` (ADR-0018 feature 7, spec area MATCH; Elixir `cond`).
//
//     match {
//         x < 0 => "negative",
//         x == 0 => "zero",
//         _ => "positive",
//     }
//
// A `match` with no scrutinee: arms are boolean conditions tried in order; the last arm must be `_`.
// Lowering (here, at parse time): `match true { _ if c1 => v1, _ if c2 => v2, _ => v3 }`, so every backend and
// the solver see an ordinary match with guards (exhaustive thanks to the final wildcard).
// Entry: `parse_guard_only_match`, called by both the statement and the expression `match` parsers when the
// token after `match` is `{`.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    /// Is the `match` keyword at `self.position` followed directly by `{` (a guard-only match)?
    pub(in crate::parser) fn at_guard_only_match(&self) -> bool {
        self.current_token() == &Token::Match && self.peek(1) == Some(&Token::LBrace)
    }

    /// Parse `match { cond => value, ..., _ => value }` (the `match` token is current) into a `Statement::Match`.
    pub(in crate::parser) fn parse_guard_only_match(
        &mut self,
    ) -> Result<&'static Statement<'static>, String> {
        let start = self.position;
        let location = self.current_location();
        self.expect(Token::Match)?;
        self.expect(Token::LBrace)?;
        let mut arms = Vec::new();
        let mut default_seen: Option<usize> = None;
        while self.current_token() != &Token::RBrace {
            let arm_pos = self.position;
            if default_seen.is_some() {
                return Err(self.pipe_error(
                    arm_pos,
                    "unreachable arm: `_ =>` must be the last arm of a guard-only `match`",
                ));
            }
            let guard = if self.current_token() == &Token::Underscore
                && self.peek(1) == Some(&Token::FatArrow)
            {
                self.advance();
                default_seen = Some(arm_pos);
                None
            } else {
                Some(self.parse_expression()?)
            };
            self.expect(Token::FatArrow)?;
            let (body, is_block) = self.parse_match_arm_body()?;
            arms.push(MatchArm {
                pattern: Pattern::Wildcard,
                guard,
                body,
            });
            if self.current_token() == &Token::Comma {
                self.advance();
            } else if self.current_token() != &Token::RBrace && !is_block {
                return Err(format!(
                    "Expected ',' or '}}' after match arm, got {:?}",
                    self.current_token()
                ));
            }
        }
        if default_seen.is_none() {
            return Err(self.pipe_error(
                start,
                "guard-only `match` needs a final `_ => ..` arm (the conditions are not known to be exhaustive)",
            ));
        }
        self.expect(Token::RBrace)?;
        let value = self.alloc_expr(Expression::Literal {
            value: Literal::Bool(true),
            location: location.clone(),
        });
        Ok(self.alloc_stmt(Statement::Match {
            value,
            arms,
            location,
        }))
    }
}
