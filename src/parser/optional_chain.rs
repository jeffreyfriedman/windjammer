// MODULE: optional chaining `a?.b` / `a?.m(args)` (ADR-0018 feature 5, spec area OPT).
//
// `a?.b` is `a.__opt_chain(|_oc| _oc.b)`: when `a: T?` is `Some(v)` the result is `v.b` lifted into an option
// (flattened when `.b` is itself an option, so `a?.b?.c` is one optional); `None` short-circuits to `None`.
// Each `?.` covers exactly one member; chain with another `?.` (`a?.b?.c`).
//
// The parser emits the plain method call above, so the analyzer, IR and linters see an ordinary closure call.
// Backends give `__opt_chain` its meaning: the interpreter and the portable (Go/JS/TS) lowering implement it;
// the Rust backend reports an explicit error until the IR solver supplies the receiver type (ADR-0018 status).

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

/// Method name of the desugared form; backends and `wj check` recognise it.
pub const OPT_CHAIN: &str = "__opt_chain";
/// The closure parameter standing for the unwrapped receiver.
pub const OPT_CHAIN_PARAM: &str = "_oc";

impl Parser {
    /// Is the current `?` directly followed by `.` on the same line (`a?.b`, not `a?` + a continued line)?
    pub(in crate::parser) fn at_optional_chain(&self) -> bool {
        if self.current_token() != &Token::Question || self.peek(1) != Some(&Token::Dot) {
            return false;
        }
        match (self.tokens.get(self.position), self.tokens.get(self.position + 1)) {
            (Some(q), Some(d)) => q.line == d.line && d.column == q.column + 1,
            _ => false,
        }
    }

    /// At `?` of `?.member`: parse the member and return the desugared call on `object`.
    pub(in crate::parser) fn parse_optional_chain(
        &mut self,
        object: &'static Expression<'static>,
    ) -> Result<&'static Expression<'static>, String> {
        let location = self.current_location();
        self.advance(); // ?
        self.advance(); // .
        let Token::Ident(member) = self.current_token().clone() else {
            return Err(format!("expected a field or method name after `?.` (at {})", self.pos_desc()));
        };
        self.advance();
        let receiver = self.alloc_expr(Expression::Identifier { name: OPT_CHAIN_PARAM.to_string(), location: location.clone() });
        let body = if self.current_token() == &Token::LParen && !self.had_newline_before_current() {
            self.advance();
            let arguments = self.parse_call_rest(Some(format!(".{member}")))?;
            self.alloc_expr(Expression::MethodCall { object: receiver, method: member, type_args: None, arguments, location: location.clone() })
        } else {
            self.alloc_expr(Expression::FieldAccess { object: receiver, field: member, location: location.clone() })
        };
        let closure = self.alloc_expr(Expression::Closure { parameters: vec![OPT_CHAIN_PARAM.to_string()], body, location: location.clone() });
        Ok(self.alloc_expr(Expression::MethodCall {
            object,
            method: OPT_CHAIN.to_string(),
            type_args: None,
            arguments: vec![(None, closure)],
            location,
        }))
    }
}
