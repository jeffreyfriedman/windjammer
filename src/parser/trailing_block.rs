// MODULE: trailing block arguments (ADR-0018 feature 4, spec area BLOCK).
//
//     with_file(path) { f -> f.read_all() }
//     button("Save") { save() }
//     pairs.each() { k, v -> println("${k}=${v}") }
//
// A call `f(args)` followed on the SAME line by `{` passes the block as one more positional argument, a closure
// (`{ params -> body }` or `{ body }`). Exactly one trailing block per call; parentheses are required.
// Not recognised in the header of `if` / `while` / `for` / `match` (where `{` opens the body): decided by scanning
// back from the `(` of the call to the start of the expression (`header_context`).
// Lowering: `Expression::Closure`, appended after the positional arguments (named ones keep their place).

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

type Args = Vec<(Option<String>, &'static Expression<'static>)>;

impl Parser {
    /// Is the token at `self.position` (a `{`) a trailing block of the call that just ended with `)`?
    fn at_trailing_block(&self) -> bool {
        if self.current_token() != &Token::LBrace || self.position == 0 {
            return false;
        }
        let (Some(close), Some(open)) = (
            self.tokens.get(self.position - 1),
            self.tokens.get(self.position),
        ) else {
            return false;
        };
        close.token == Token::RParen
            && close.line == open.line
            && !self.header_context(self.position - 1)
    }

    /// Scanning back from the `)` at `close`: is this expression part of an `if`/`while`/`for`/`match` header?
    fn header_context(&self, close: usize) -> bool {
        let mut depth: i32 = 0;
        let mut i = close;
        loop {
            let t = &self.tokens[i].token;
            match t {
                Token::RParen | Token::RBracket => depth += 1,
                Token::LParen | Token::LBracket => {
                    depth -= 1;
                    if depth < 0 {
                        return false; // inside an argument list or index: not a statement header
                    }
                }
                _ if depth > 0 => {}
                Token::If | Token::While | Token::For | Token::Match | Token::In => return true,
                Token::Let => {
                    // `while let P = e {` / `if let P = e {`
                    return i > 0 && matches!(self.tokens[i - 1].token, Token::If | Token::While);
                }
                // `if let Point { x, y } = f(a) {`: the braces of a struct pattern are part of the header.
                Token::RBrace => match self.struct_pattern_open_brace(i) {
                    Some(open) => i = open,
                    None => return false,
                },
                Token::LBrace
                | Token::Semicolon
                | Token::FatArrow
                | Token::Return
                | Token::Comma => return false,
                _ => {}
            }
            if i == 0 {
                return false;
            }
            i -= 1;
        }
    }

    /// For a `}` at `close` that ends a struct pattern or literal (`Type { .. }`, the `{` preceded by an uppercase
    /// name), the index of its `{`; `None` for any other `}` (a block end, a statement boundary).
    fn struct_pattern_open_brace(&self, close: usize) -> Option<usize> {
        let mut depth = 0i32;
        let mut j = close;
        loop {
            match self.tokens[j].token {
                Token::RBrace => depth += 1,
                Token::LBrace => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            if j == 0 {
                return None;
            }
            j -= 1;
        }
        match j.checked_sub(1).map(|k| &self.tokens[k].token) {
            Some(Token::Ident(n)) if n.chars().next().is_some_and(char::is_uppercase) => Some(j),
            _ => None,
        }
    }

    /// `{ a, b: int -> ` at the current `{`: the parameter list, consuming it; `None` (nothing consumed) otherwise.
    fn parse_block_params(&mut self) -> Option<Vec<String>> {
        let saved = self.position;
        self.advance(); // `{`
        let mut params = Vec::new();
        loop {
            let Token::Ident(name) = self.current_token().clone() else {
                break;
            };
            self.advance();
            let mut p = name;
            if self.current_token() == &Token::Colon {
                self.advance();
                match self.parse_type() {
                    Ok(ty) => p = format!("{p}: {}", self.type_to_string(&ty)),
                    Err(_) => break,
                }
            }
            params.push(p);
            match self.current_token() {
                Token::Comma => self.advance(),
                Token::Arrow => {
                    self.advance();
                    return Some(params);
                }
                _ => break,
            }
        }
        self.position = saved;
        None
    }

    /// After `f(args)`: append a trailing block closure to `args` when one follows.
    pub(in crate::parser) fn append_trailing_block(
        &mut self,
        mut args: Args,
    ) -> Result<Args, String> {
        if !self.at_trailing_block() {
            return Ok(args);
        }
        let location = self.current_location();
        let parameters = match self.parse_block_params() {
            Some(p) => p,
            None => {
                self.advance(); // `{`
                Vec::new()
            }
        };
        let statements = self.with_explicit_closure(|p| p.parse_block_statements())?;
        self.expect(Token::RBrace)?;
        let body = self.alloc_expr(Expression::Block {
            statements,
            is_unsafe: false,
            location: location.clone(),
        });
        let closure = self.alloc_expr(Expression::Closure {
            parameters,
            body,
            location,
        });
        let at = args
            .iter()
            .position(|(l, _)| l.is_some())
            .unwrap_or(args.len());
        args.insert(at, (None, closure));
        if self.current_token() == &Token::LBrace && self.at_trailing_block_after_block() {
            return Err(format!(
                "only one trailing block per call (at {}); pass further callbacks as named arguments",
                self.pos_desc()
            ));
        }
        Ok(args)
    }

    /// A second `{` right after a trailing block's `}` on the same line.
    fn at_trailing_block_after_block(&self) -> bool {
        match (
            self.tokens.get(self.position.wrapping_sub(1)),
            self.tokens.get(self.position),
        ) {
            (Some(a), Some(b)) => a.token == Token::RBrace && a.line == b.line,
            _ => false,
        }
    }
}
