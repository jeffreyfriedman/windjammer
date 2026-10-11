//! True source spans for AST nodes, recorded on request.
//!
//! `Expression::location()` / `Statement::location()` are *identity keys* for the numeric
//! inference passes (they record the position of the token following a node and are shared by
//! nested nodes), so they are not spans. When `Parser::enable_span_log()` is called, every node
//! allocated by the parser also gets a real `[start, end)` span here: the start is the first token
//! of the innermost parse entry (expression, binary operand, primary, statement) that allocated
//! it and the end is the end of the last token consumed when it was allocated.
//!
//! The spans are validated by tests/span_oracle_suite.rs (gate 2), which checks them against the
//! source text independently of the parser.

use crate::parser::ast::{Expression, Parameter, Statement};
use crate::parser_impl::Parser;
use std::collections::HashMap;

/// Byte range plus line/column rendering (1-based, columns in chars, end exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Default)]
pub(crate) struct SpanLog {
    spans: HashMap<usize, NodeSpan>,
    /// Parameter spans keyed by the address of the `FunctionDecl.parameters` buffer.
    params: HashMap<usize, Vec<NodeSpan>>,
    /// Token index at which each active parse entry started.
    starts: Vec<usize>,
}

/// An immutable copy of the recorded spans that outlives borrows of the parser (the AST it
/// describes must stay alive: lookups are by node address).
#[derive(Clone, Default)]
pub struct SpanSnapshot {
    spans: HashMap<usize, NodeSpan>,
    params: HashMap<usize, Vec<NodeSpan>>,
}

impl SpanSnapshot {
    pub fn expr(&self, e: &Expression) -> Option<NodeSpan> {
        self.spans.get(&(e as *const Expression as usize)).copied()
    }
    pub fn stmt(&self, s: &Statement) -> Option<NodeSpan> {
        self.spans.get(&(s as *const Statement as usize)).copied()
    }
    /// Span of parameter `index` of a function whose `parameters` slice is `params`.
    pub fn param(&self, params: &[Parameter], index: usize) -> Option<NodeSpan> {
        self.params.get(&(params.as_ptr() as usize))?.get(index).copied()
    }
}

impl Parser {
    /// Copy of the spans recorded so far (empty if logging is off).
    pub fn span_snapshot(&self) -> SpanSnapshot {
        match self.span_log.borrow().as_ref() {
            Some(l) => SpanSnapshot { spans: l.spans.clone(), params: l.params.clone() },
            None => SpanSnapshot::default(),
        }
    }

    /// Record the span of each parameter (token ranges `[start, end)`), keyed by the buffer.
    pub(crate) fn record_param_spans(&self, params: &[Parameter], ranges: &[(usize, usize)]) {
        let mut guard = self.span_log.borrow_mut();
        let Some(log) = guard.as_mut() else { return };
        if params.is_empty() || params.len() != ranges.len() {
            return;
        }
        let spans = ranges
            .iter()
            .filter_map(|&(a, b)| {
                let first = self.tokens.get(a)?;
                let last = self.tokens.get(b.checked_sub(1)?)?;
                Some(NodeSpan {
                    byte_start: first.byte_start,
                    byte_end: last.byte_end,
                    line: first.line,
                    column: first.column,
                    end_line: last.end_line,
                    end_column: last.end_column,
                })
            })
            .collect::<Vec<_>>();
        if spans.len() == params.len() {
            log.params.insert(params.as_ptr() as usize, spans);
        }
    }

    /// Start recording node spans (see module docs). Call before `parse()`.
    pub fn enable_span_log(&mut self) {
        *self.span_log.borrow_mut() = Some(SpanLog::default());
    }

    /// Span of an expression allocated by this parser (None if logging was off).
    pub fn expr_span(&self, e: &Expression) -> Option<NodeSpan> {
        self.span_for(e as *const Expression as usize)
    }

    /// Span of a statement allocated by this parser (None if logging was off).
    pub fn stmt_span(&self, s: &Statement) -> Option<NodeSpan> {
        self.span_for(s as *const Statement as usize)
    }

    fn span_for(&self, addr: usize) -> Option<NodeSpan> {
        self.span_log.borrow().as_ref()?.spans.get(&addr).copied()
    }

    /// Copy the spans recorded by a sub-parser (string-interpolation bodies) into this log.
    pub(crate) fn adopt_spans(&self, other: &Parser) {
        let theirs = other.span_log.borrow();
        let mut ours = self.span_log.borrow_mut();
        if let (Some(t), Some(o)) = (theirs.as_ref(), ours.as_mut()) {
            o.spans.extend(t.spans.iter().map(|(k, v)| (*k, *v)));
        }
    }

    pub(crate) fn span_enter(&self) {
        if let Some(log) = self.span_log.borrow_mut().as_mut() {
            log.starts.push(self.position);
        }
    }

    pub(crate) fn span_exit(&self) {
        if let Some(log) = self.span_log.borrow_mut().as_mut() {
            log.starts.pop();
        }
    }

    pub(crate) fn record_span(&self, addr: usize) {
        let mut guard = self.span_log.borrow_mut();
        let Some(log) = guard.as_mut() else { return };
        let end_excl = self.position.min(self.tokens.len());
        let start_idx = log.starts.last().copied().unwrap_or_else(|| end_excl.saturating_sub(1));
        let Some(first) = self.tokens.get(start_idx).or_else(|| self.tokens.last()) else { return };
        let last = if end_excl > start_idx { &self.tokens[end_excl - 1] } else { first };
        log.spans.insert(
            addr,
            NodeSpan {
                byte_start: first.byte_start,
                byte_end: last.byte_end.max(first.byte_start),
                line: first.line,
                column: first.column,
                end_line: if last.byte_end >= first.byte_start { last.end_line } else { first.line },
                end_column: if last.byte_end >= first.byte_start { last.end_column } else { first.column },
            },
        );
    }
}
