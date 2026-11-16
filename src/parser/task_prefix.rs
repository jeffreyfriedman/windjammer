// ADR-0004 concurrency surface (SPEC-CONC-001..003, docs/spec/concurrency.md):
//   spawn f(x) / spawn { ... }   -> Expression::SpawnCall   (a `Task<T>`)
//   async f(x) / async { ... }   -> Expression::AsyncCall   (a `Task<T>`)
//   scope { ... }                -> Expression::Scope       (waits for every task started inside)
// `spawn` and `scope` are contextual: only followed by a name, `self` or `{` on the same line.
//
// Removed forms (WJ0191, SPEC-CONC-007): `thread { }`, `ch <- v` and `<-ch`. The parser reports each with a
// machine-applicable edit (`RemovedForm`) that `wj check --fix` and `wj fix` apply; it never accepts them.

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

/// Diagnostic code of a removed concurrency form.
pub const REMOVED_FORM_CODE: &str = "WJ0191";

/// A removed concurrency form the parser met, with the edits that migrate it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedForm {
    /// Position of the removed token (`thread` or `<-`), 1-based.
    pub line: usize,
    pub column: usize,
    /// Byte range of the removed token.
    pub byte_start: usize,
    pub byte_end: usize,
    pub message: String,
    pub help: String,
    /// `(byte_start, byte_end, replacement)` edits that turn the old form into the new one; empty when the
    /// operand could not be delimited (then the fix is manual).
    pub edits: Vec<(usize, usize, String)>,
}

impl RemovedForm {
    /// The parse error text: code, message and position (`check/front.rs` reads the position back).
    pub fn error_text(&self) -> String {
        format!("{}: {} (at line {}, column {})", REMOVED_FORM_CODE, self.message, self.line, self.column)
    }
}

impl Parser {
    /// A primary expression (task forms are primaries: `parse_primary_expression` recognises them, so they also
    /// work as operands of unary operators and take postfix operators: `spawn { 1 }.wait()`).
    pub(in crate::parser) fn parse_prefixed_or_primary(&mut self) -> Result<&'static Expression<'static>, String> {
        self.parse_primary_expression()
    }

    /// The current token starts a task form: `spawn`/`async` before a name, `self` or `{`, `scope` before `{`, with
    /// the next token on the same line.
    pub(in crate::parser) fn task_prefix_follows(&self) -> bool {
        let kw = match self.current_token() {
            Token::Ident(n) if n == "spawn" || n == "scope" => n.as_str(),
            Token::Async => "async",
            _ => return false,
        };
        let starts_operand = match (self.peek(1), kw) {
            (Some(Token::LBrace), _) => true,
            (Some(Token::Ident(_) | Token::Self_), "spawn" | "async") => true,
            _ => false,
        };
        starts_operand && self.next_on_same_line()
    }

    fn next_on_same_line(&self) -> bool {
        match (self.tokens.get(self.position), self.tokens.get(self.position + 1)) {
            (Some(a), Some(b)) => a.line == b.line,
            _ => false,
        }
    }

    /// Called with the current token on `spawn`/`scope`/`async`; `None` when it is not a prefix.
    pub(in crate::parser) fn try_parse_task_prefix(&mut self) -> Result<Option<&'static Expression<'static>>, String> {
        if !self.task_prefix_follows() {
            return Ok(None);
        }
        let kw = match self.current_token() {
            Token::Ident(n) => n.clone(),
            _ => "async".to_string(),
        };
        // node spans (span log): the task starts at its keyword, a block operand at its `{`
        self.span_enter();
        let r = self.parse_task_form(&kw);
        self.span_exit();
        r.map(Some)
    }

    fn parse_task_form(&mut self, kw: &str) -> Result<&'static Expression<'static>, String> {
        let location = self.current_location();
        self.advance(); // the prefix
        let operand = if self.current_token() == &Token::LBrace {
            self.span_enter();
            let block = self.parse_task_block();
            self.span_exit();
            block?
        } else {
            self.parse_binary_expression(100)?
        };
        Ok(match kw {
            "spawn" => self.alloc_expr(Expression::SpawnCall { expr: operand, location }),
            "async" => self.alloc_expr(Expression::AsyncCall { expr: operand, location }),
            _ => self.alloc_expr(Expression::Scope { body: operand, location }),
        })
    }

    fn parse_task_block(&mut self) -> Result<&'static Expression<'static>, String> {
        let location = self.current_location();
        self.advance(); // `{`
        let statements = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        Ok(self.alloc_expr(Expression::Block { statements, is_unsafe: false, location }))
    }

    fn removed_form(&mut self, form: RemovedForm) -> String {
        let text = form.error_text();
        if !self.removed_forms.iter().any(|f| f.byte_start == form.byte_start) {
            self.removed_forms.push(form);
        }
        text
    }

    /// (line, column, byte_start, byte_end) of token `i` (the last token when past the end).
    fn token_at(&self, i: usize) -> (usize, usize, usize, usize) {
        let t = &self.tokens[i.min(self.tokens.len().saturating_sub(1))];
        (t.src_line, t.src_column, t.byte_start, t.byte_end)
    }

    /// `thread { ... }` (current token `thread`, next `{`): WJ0191, fixed to `spawn { ... }`.
    pub(in crate::parser) fn removed_thread_block(&mut self) -> String {
        let (line, column, b0, b1) = self.token_at(self.position);
        self.removed_form(RemovedForm {
            line,
            column,
            byte_start: b0,
            byte_end: b1,
            message: "`thread { }` was removed: write `spawn { }` (ADR-0004)".into(),
            help: "`spawn { }` starts the block as a `Task`; inside a `scope { }` the scope waits for it, elsewhere keep the task and call `.wait()` (WJ0190)".into(),
            edits: vec![(b0, b1, "spawn".into())],
        })
    }

    /// `<-ch` (current token `<-`): WJ0191, fixed to `ch.recv()`.
    pub(in crate::parser) fn removed_channel_recv(&mut self) -> String {
        let arrow = self.position;
        let (line, column, b0, b1) = self.token_at(arrow);
        self.advance();
        let edits = match self.parse_unary_operand() {
            Ok(_) if self.position > arrow + 1 => {
                let (_, _, _, end) = self.token_at(self.position - 1);
                vec![(b0, b1, String::new()), (end, end, ".recv()".into())]
            }
            _ => Vec::new(),
        };
        self.removed_form(RemovedForm {
            line,
            column,
            byte_start: b0,
            byte_end: b1,
            message: "the `<-ch` receive operator was removed: write `ch.recv()` (ADR-0004)".into(),
            help: "channels are values with methods: `ch.recv()` receives, `ch.send(v)` sends; to compare with a negative number write `x < -1`".into(),
            edits,
        })
    }

    /// `left <- value` (current token `<-`): WJ0191, fixed to `left.send(value)`.
    pub(in crate::parser) fn removed_channel_send(&mut self, left: &Expression<'static>) -> String {
        let arrow = self.position;
        let (line, column, b0, b1) = self.token_at(arrow);
        let (_, _, _, left_end) = self.token_at(arrow.saturating_sub(1));
        self.advance();
        let simple_left = matches!(
            left,
            Expression::Identifier { .. }
                | Expression::FieldAccess { .. }
                | Expression::MethodCall { .. }
                | Expression::Call { .. }
                | Expression::Index { .. }
        );
        let value_start = self.position;
        let edits = match self.parse_expression() {
            Ok(_) if simple_left && self.position > value_start => {
                let (_, _, v0, _) = self.token_at(value_start);
                let (_, _, _, v1) = self.token_at(self.position - 1);
                vec![(left_end, v0, ".send(".into()), (v1, v1, ")".into())]
            }
            _ => Vec::new(),
        };
        self.removed_form(RemovedForm {
            line,
            column,
            byte_start: b0,
            byte_end: b1,
            message: "the `ch <- v` send operator was removed: write `ch.send(v)` (ADR-0004)".into(),
            help: "channels are values with methods: `ch.send(v)` sends, `ch.recv()` receives; to compare with a negative number write `x < -1`".into(),
            edits,
        })
    }
}
