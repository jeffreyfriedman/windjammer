// ADR-0004 concurrency prefixes (SPEC-CONC-001..003):
//   spawn f(x) / spawn { ... }   -> Expression::SpawnCall
//   async f(x)                   -> Expression::AsyncCall      (`async { }` keeps its block form)
//   scope { ... }                -> call of the scope runtime with a closure over the block
// `spawn` and `scope` are contextual: only followed by a name or `{` on the same line.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

/// Name of the runtime entry that runs a `scope { }` body and waits for its tasks.
pub const SCOPE_FN: &str = "__wj_scope";

impl Parser {
    /// A primary expression, or a `spawn`/`async`/`scope` task form.
    pub(in crate::parser) fn parse_prefixed_or_primary(&mut self) -> Result<&'static Expression<'static>, String> {
        match self.try_parse_task_prefix()? {
            Some(e) => Ok(e),
            None => self.parse_primary_expression(),
        }
    }

    fn next_on_same_line(&self) -> bool {
        match (self.tokens.get(self.position), self.tokens.get(self.position + 1)) {
            (Some(a), Some(b)) => a.line == b.line,
            _ => false,
        }
    }

    /// Called with the current token on `spawn`/`scope`/`async`; `None` when it is not a prefix.
    pub(in crate::parser) fn try_parse_task_prefix(&mut self) -> Result<Option<&'static Expression<'static>>, String> {
        let kw = match self.current_token() {
            Token::Ident(n) if n == "spawn" || n == "scope" => n.clone(),
            Token::Async => "async".to_string(),
            _ => return Ok(None),
        };
        let next = self.peek(1).cloned();
        let starts_operand = match (&next, kw.as_str()) {
            (Some(Token::LBrace), "spawn" | "scope") => true,
            (Some(Token::Ident(_)), "spawn" | "async") => true,
            _ => false,
        };
        if !starts_operand || !self.next_on_same_line() {
            return Ok(None);
        }
        let location = self.current_location();
        self.advance(); // the prefix
        let operand = if self.current_token() == &Token::LBrace {
            self.advance();
            let statements = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;
            self.alloc_expr(Expression::Block { statements, is_unsafe: false, location: location.clone() })
        } else {
            self.parse_binary_expression(100)?
        };
        Ok(Some(match kw.as_str() {
            "spawn" => self.alloc_expr(Expression::SpawnCall { expr: operand, location }),
            "async" => self.alloc_expr(Expression::AsyncCall { expr: operand, location }),
            _ => {
                let function = self.alloc_expr(Expression::Identifier { name: SCOPE_FN.to_string(), location: location.clone() });
                let body = self.alloc_expr(Expression::Closure { parameters: vec![], body: operand, location: location.clone() });
                self.alloc_expr(Expression::Call { function, arguments: vec![(None, body)], location })
            }
        }))
    }
}
