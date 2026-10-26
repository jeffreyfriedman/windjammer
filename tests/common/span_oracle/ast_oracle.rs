//! Gate 2: parser/AST span invariants.
//!
//! `Expression::location()` is an identity key for inference (see src/parser/span_log.rs), so the
//! real spans come from the parser's span log (`Parser::enable_span_log`). The oracle checks every
//! node of every successfully parsed program against the *source text*, independently of the
//! parser: well-formed range, line/col recomputed from the byte offset, non-empty, inside its
//! parent, siblings ordered, text at the start is a legal first token of the node's kind, text at
//! the end is a legal last token, and the slice re-parses to the same expression kind.

use super::naive::line_col;
use windjammer::lexer::Lexer;
use windjammer::parser::ast::*;
use windjammer::parser::NodeSpan;
use windjammer::parser_impl::Parser;

/// Where spans come from; the real parser, or a hand-built table in mutation self-tests.
pub trait SpanSource {
    fn expr(&self, e: &Expression) -> Option<NodeSpan>;
    fn stmt(&self, s: &Statement) -> Option<NodeSpan>;
}

impl SpanSource for Parser {
    fn expr(&self, e: &Expression) -> Option<NodeSpan> {
        self.expr_span(e)
    }
    fn stmt(&self, s: &Statement) -> Option<NodeSpan> {
        self.stmt_span(s)
    }
}

#[derive(Default)]
pub struct Findings {
    /// "rule:NodeKind" -> (count, first example)
    pub by_class: std::collections::BTreeMap<String, (usize, String)>,
    pub nodes: usize,
}

impl Findings {
    fn add(&mut self, rule: &str, kind: &str, detail: String) {
        let e = self.by_class.entry(format!("{rule}:{kind}")).or_insert((0, detail));
        e.0 += 1;
    }
}

pub enum ParseOutcome {
    Ok(Findings),
    ParseError(String),
    LexerPanic(String),
}

pub fn check_source(file: &str, src: &str) -> ParseOutcome {
    let (file_s, src_s) = (file.to_string(), src.to_string());
    let r = std::panic::catch_unwind(move || {
        let tokens = Lexer::new(&src_s).tokenize_with_locations();
        let mut p = Parser::new_with_source(tokens, file_s.clone(), src_s.clone());
        p.enable_span_log();
        p.parse().map(|prog| check_program(&src_s, &prog, &p))
    });
    match r {
        Ok(Ok(f)) => ParseOutcome::Ok(f),
        Ok(Err(e)) => ParseOutcome::ParseError(e),
        Err(p) => ParseOutcome::LexerPanic(
            p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default(),
        ),
    }
}

pub fn check_program(src: &str, prog: &Program, spans: &dyn SpanSource) -> Findings {
    let mut f = Findings::default();
    let ctx = Ctx { src, spans };
    for item in &prog.items {
        ctx.item(item, &mut f);
    }
    f
}

/// Check hand-built items against a hand-built span table (mutation self-tests).
pub fn check_items(src: &str, items: &[Item], spans: &dyn SpanSource) -> Findings {
    let mut f = Findings::default();
    let ctx = Ctx { src, spans };
    for i in items {
        ctx.item(i, &mut f);
    }
    f
}

struct Ctx<'a> {
    src: &'a str,
    spans: &'a dyn SpanSource,
}

type ByteRange = (usize, usize);

impl<'a> Ctx<'a> {
    /// Structural checks on one node's span; returns its byte range when usable.
    fn check_span(&self, f: &mut Findings, kind: &str, span: Option<NodeSpan>, parent: Option<ByteRange>) -> Option<ByteRange> {
        f.nodes += 1;
        let Some(s) = span else {
            f.add("no-span", kind, "parser recorded no span".into());
            return None;
        };
        if s.byte_start > s.byte_end || s.byte_end > self.src.len() {
            f.add("bad-range", kind, format!("{}..{} (len {})", s.byte_start, s.byte_end, self.src.len()));
            return None;
        }
        if !self.src.is_char_boundary(s.byte_start) || !self.src.is_char_boundary(s.byte_end) {
            f.add("not-char-boundary", kind, format!("{}..{}", s.byte_start, s.byte_end));
            return None;
        }
        if s.byte_start == s.byte_end {
            f.add("empty-span", kind, format!("at byte {}", s.byte_start));
        }
        let (l, c) = line_col(self.src, s.byte_start);
        if (s.line, s.column) != (l, c) {
            f.add("line-col-mismatch", kind, format!("start ({},{}) != oracle ({l},{c}) for byte {}", s.line, s.column, s.byte_start));
        }
        let (el, ec) = line_col(self.src, s.byte_end);
        if (s.end_line, s.end_column) != (el, ec) {
            f.add("line-col-mismatch", kind, format!("end ({},{}) != oracle ({el},{ec}) for byte {}", s.end_line, s.end_column, s.byte_end));
        }
        if let Some((ps, pe)) = parent {
            if s.byte_start < ps || s.byte_end > pe {
                f.add("outside-parent", kind, format!("{}..{} not within parent {ps}..{pe}", s.byte_start, s.byte_end));
            }
        }
        Some((s.byte_start, s.byte_end))
    }

    fn order(&self, r: Option<ByteRange>, kind: &str, f: &mut Findings, prev: &mut Option<ByteRange>) {
        if let Some((s, e)) = r {
            if let Some((_, pe)) = *prev {
                if s < pe && !(s == 0 && e == 0) {
                    // siblings may not overlap or go backwards
                    f.add("siblings-out-of-order", kind, format!("{s}..{e} starts before previous sibling ends at {pe}"));
                }
            }
            *prev = Some((s, e));
        }
    }

    fn slice(&self, r: ByteRange) -> &'a str {
        &self.src[r.0..r.1]
    }

    fn starts(&self, f: &mut Findings, kind: &str, r: Option<ByteRange>, opts: &[&str]) {
        let Some(r) = r else { return };
        let t = self.slice(r);
        if !opts.iter().any(|o| t.starts_with(o)) {
            f.add("wrong-first-token", kind, format!("slice {:?} should start with one of {opts:?}", t.chars().take(16).collect::<String>()));
        }
    }

    fn ends(&self, f: &mut Findings, kind: &str, r: Option<ByteRange>, opts: &[&str]) {
        let Some(r) = r else { return };
        let t = self.slice(r);
        if !opts.iter().any(|o| t.ends_with(o)) {
            let tail: String = t.chars().rev().take(16).collect::<Vec<_>>().into_iter().rev().collect();
            f.add("wrong-last-token", kind, format!("slice ...{tail:?} should end with one of {opts:?}"));
        }
    }

    /// The node's end must be its last child's end, give or take closing parentheses of a
    /// parenthesised operand and a statement-terminating `;`.
    fn ends_with_child(&self, f: &mut Findings, kind: &str, r: Option<ByteRange>, child: Option<NodeSpan>) {
        if let (Some(r), Some(c)) = (r, child) {
            let ok = r.1 >= c.byte_end
                && self.src.get(c.byte_end..r.1).is_some_and(|gap| gap.chars().all(|ch| ch == ')' || ch == ';' || ch.is_whitespace()));
            if !ok {
                f.add("end-not-last-child", kind, format!("ends at {} but last child ends at {}", r.1, c.byte_end));
            }
        }
    }

    fn expr(&self, e: &Expression, f: &mut Findings, parent: Option<ByteRange>) {
        use Expression::*;
        let kind = expr_kind(e);
        let sp = self.spans.expr(e);
        let me = self.check_span(f, kind, sp, parent);
        match e {
            Identifier { name, .. } => {
                self.starts(f, kind, me, &[name.as_str()]);
                self.ends(f, kind, me, &[name.as_str()]);
            }
            Literal { value, .. } => match value {
                windjammer::parser::ast::Literal::Bool(b) => {
                    let w = if *b { "true" } else { "false" };
                    self.starts(f, kind, me, &[w]);
                    self.ends(f, kind, me, &[w]);
                }
                windjammer::parser::ast::Literal::Char(_) => {
                    self.starts(f, kind, me, &["'"]);
                    self.ends(f, kind, me, &["'"]);
                }
                windjammer::parser::ast::Literal::String(_) => {
                    self.starts(f, kind, me, &["\"", "r#", "r\""]);
                    self.ends(f, kind, me, &["\"", "#"]);
                }
                _ => self.starts(f, kind, me, &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]),
            },
            Unary { .. } => self.starts(f, kind, me, &["-", "!", "*", "&"]),
            Closure { .. } => self.starts(f, kind, me, &["|", "move"]),
            Tuple { .. } => {
                self.starts(f, kind, me, &["("]);
                self.ends(f, kind, me, &[")"]);
            }
            Array { .. } => {
                self.starts(f, kind, me, &["["]);
                self.ends(f, kind, me, &["]"]);
            }
            Block { .. } => {
                // Blocks also wrap `if`/`match`/loop expressions that are used as values.
                self.starts(f, kind, me, &["{", "unsafe", "if", "match", "while", "loop", "for", "async", "thread"]);
                self.ends(f, kind, me, &["}"]);
            }
            MapLiteral { .. } => {
                self.starts(f, kind, me, &["{"]);
                self.ends(f, kind, me, &["}"]);
            }
            Call { .. } | MethodCall { .. } => self.ends(f, kind, me, &[")"]),
            Index { .. } => self.ends(f, kind, me, &["]"]),
            TryOp { .. } => self.ends(f, kind, me, &["?"]),
            Await { .. } => self.ends(f, kind, me, &["await"]),
            StructLiteral { name, .. } => {
                self.starts(f, kind, me, &[name.rsplit("::").next().unwrap_or(name), name]);
                self.ends(f, kind, me, &["}"]);
            }
            MacroInvocation { name, .. } => self.starts(f, kind, me, &[name.as_str(), "\"", "format"]),
            AsyncCall { .. } => self.starts(f, kind, me, &["async"]),
            SpawnCall { .. } => self.starts(f, kind, me, &["spawn"]),
            ChannelRecv { .. } => self.starts(f, kind, me, &["<-"]),
            _ => {}
        }
        let mut prev: Option<ByteRange> = None;
        let mut child = |c: &Expression, f: &mut Findings, prev: &mut Option<ByteRange>| {
            self.expr(c, f, me);
            let r = self.spans.expr(c).map(|s| (s.byte_start, s.byte_end));
            self.order(r, kind, f, prev);
        };
        match e {
            Literal { .. } | Identifier { .. } => {}
            Binary { left, right, .. } => {
                child(left, f, &mut prev);
                child(right, f, &mut prev);
                self.starts_with_child(f, kind, me, self.spans.expr(left));
                self.ends_with_child(f, kind, me, self.spans.expr(right));
            }
            Unary { operand, .. } => {
                child(operand, f, &mut prev);
                self.ends_with_child(f, kind, me, self.spans.expr(operand));
            }
            Call { function, arguments, .. } => {
                child(function, f, &mut prev);
                for (_, a) in arguments {
                    child(a, f, &mut prev);
                }
                self.starts_with_child(f, kind, me, self.spans.expr(function));
            }
            MethodCall { object, arguments, .. } => {
                child(object, f, &mut prev);
                for (_, a) in arguments {
                    child(a, f, &mut prev);
                }
                self.starts_with_child(f, kind, me, self.spans.expr(object));
            }
            FieldAccess { object, field, .. } => {
                child(object, f, &mut prev);
                self.starts_with_child(f, kind, me, self.spans.expr(object));
                self.ends(f, kind, me, &[field.as_str()]);
            }
            StructLiteral { fields, .. } => {
                for (_, v) in fields {
                    child(v, f, &mut prev);
                }
            }
            MapLiteral { pairs, .. } => {
                for (k, v) in pairs {
                    child(k, f, &mut prev);
                    child(v, f, &mut prev);
                }
            }
            Range { start, end, .. } => {
                child(start, f, &mut prev);
                child(end, f, &mut prev);
            }
            Closure { body, .. } => {
                child(body, f, &mut prev);
                self.ends_with_child(f, kind, me, self.spans.expr(body));
            }
            Cast { expr, .. } | TryOp { expr, .. } | Await { expr, .. } | AsyncCall { expr, .. } | SpawnCall { expr, .. } => {
                child(expr, f, &mut prev);
                if !matches!(e, Cast { .. } | AsyncCall { .. } | SpawnCall { .. }) {
                    self.starts_with_child(f, kind, me, self.spans.expr(expr));
                }
            }
            Index { object, index, .. } => {
                child(object, f, &mut prev);
                child(index, f, &mut prev);
                self.starts_with_child(f, kind, me, self.spans.expr(object));
            }
            Tuple { elements, .. } | Array { elements, .. } => {
                for c in elements {
                    child(c, f, &mut prev);
                }
            }
            MacroInvocation { args, .. } => {
                // Arguments of a lowered interpolated string lie inside its format literal, so
                // they are checked for containment but not for sibling order.
                for c in args {
                    let mut unordered = None;
                    child(c, f, &mut unordered);
                }
            }
            ChannelSend { channel, value, .. } => {
                child(channel, f, &mut prev);
                child(value, f, &mut prev);
                self.starts_with_child(f, kind, me, self.spans.expr(channel));
                self.ends_with_child(f, kind, me, self.spans.expr(value));
            }
            ChannelRecv { channel, .. } => child(channel, f, &mut prev),
            Block { statements, .. } => {
                for s in statements {
                    self.stmt(s, f, me, &mut prev);
                }
            }
        }
        if let Some(r) = me {
            self.reparse(f, e, kind, r);
        }
    }

    fn starts_with_child(&self, f: &mut Findings, kind: &str, r: Option<ByteRange>, child: Option<NodeSpan>) {
        if let (Some(r), Some(c)) = (r, child) {
            // A parenthesised base `(a).b` legitimately starts before its child.
            let before = &self.src[r.0..c.byte_start.max(r.0).min(self.src.len())];
            if c.byte_start < r.0 || (r.0 != c.byte_start && !before.chars().all(|ch| ch == '(' || ch.is_whitespace())) {
                f.add("start-not-first-child", kind, format!("starts at {} but first child starts at {}", r.0, c.byte_start));
            }
        }
    }

    /// The node's own text must parse as one expression of the same kind.
    fn reparse(&self, f: &mut Findings, e: &Expression, kind: &str, r: ByteRange) {
        use Expression::*;
        // Blocks/maps are ambiguous in isolation; synthetic format() macro nodes have no source form.
        if matches!(e, Block { .. } | MapLiteral { .. } | MacroInvocation { .. } | Literal { value: windjammer::parser::ast::Literal::String(_), .. }) {
            return;
        }
        let text = self.slice(r).to_string();
        let want = kind;
        let res = std::panic::catch_unwind(move || {
            // Parse the slice as the only statement of a function body (no public expression entry
            // point exists, and none is needed): it must be exactly one expression statement.
            let wrapped = format!("fn __t() {{\n{text}\n}}\n");
            let tokens = Lexer::new(&wrapped).tokenize_with_locations();
            let mut p = Parser::new_with_source(tokens, String::new(), wrapped);
            p.parse().map(|prog| match prog.items.first() {
                Some(Item::Function { decl, .. }) if decl.body.len() == 1 => match decl.body[0] {
                    Statement::Expression { expr, .. } => Some((expr_kind(expr), true)),
                    _ => None,
                },
                Some(Item::Function { decl, .. }) if decl.body.len() > 1 => Some(("<several statements>", false)),
                _ => None,
            })
        });
        match res {
            Ok(Ok(None)) => {} // the slice is a statement form (let/if/match...), not an expression
            Ok(Ok(Some((got, all)))) => {
                if got != want {
                    f.add("reparse-kind-mismatch", kind, format!("{:?} re-parses as {got}", self.slice(r).chars().take(40).collect::<String>()));
                } else if !all {
                    f.add("reparse-leftover-tokens", kind, format!("{:?}", self.slice(r).chars().take(40).collect::<String>()));
                }
            }
            Ok(Err(err)) => f.add("reparse-error", kind, format!("{:?}: {err}", self.slice(r).chars().take(40).collect::<String>())),
            Err(_) => f.add("reparse-error", kind, "panic".into()),
        }
    }

    fn stmts(&self, ss: &[&Statement], f: &mut Findings, parent: Option<ByteRange>) {
        let mut prev = None;
        for s in ss {
            self.stmt(s, f, parent, &mut prev);
        }
    }

    fn stmt(&self, s: &Statement, f: &mut Findings, parent: Option<ByteRange>, prev: &mut Option<ByteRange>) {
        use Statement::*;
        let kind = stmt_kind(s);
        let sp = self.spans.stmt(s);
        let me = self.check_span(f, kind, sp, parent);
        match s {
            Let { .. } => self.starts(f, kind, me, &["let"]),
            Const { .. } => self.starts(f, kind, me, &["const", "pub"]),
            Static { .. } => self.starts(f, kind, me, &["static", "pub"]),
            Return { .. } => self.starts(f, kind, me, &["return"]),
            If { .. } => self.starts(f, kind, me, &["if"]),
            Match { .. } => self.starts(f, kind, me, &["match", "if", "while"]),
            For { .. } => self.starts(f, kind, me, &["for"]),
            Loop { .. } => self.starts(f, kind, me, &["loop", "while"]),
            While { .. } => self.starts(f, kind, me, &["while"]),
            Thread { .. } => self.starts(f, kind, me, &["thread"]),
            Async { .. } => self.starts(f, kind, me, &["async"]),
            Defer { .. } => self.starts(f, kind, me, &["defer"]),
            Break { .. } => self.starts(f, kind, me, &["break"]),
            Continue { .. } => self.starts(f, kind, me, &["continue"]),
            Use { .. } => self.starts(f, kind, me, &["use", "pub"]),
            Assignment { .. } | Expression { .. } => {}
        }
        self.order(me, kind, f, prev);
        match s {
            Let { value, else_block, .. } => {
                self.expr(value, f, me);
                if let Some(b) = else_block {
                    self.stmts(b, f, me);
                }
            }
            Const { value, .. } | Static { value, .. } => self.expr(value, f, me),
            Assignment { target, value, .. } => {
                self.expr(target, f, me);
                self.expr(value, f, me);
                self.ends_with_child(f, kind, me, self.spans.expr(value));
            }
            Return { value, .. } => {
                if let Some(v) = value {
                    self.expr(v, f, me);
                    self.ends_with_child(f, kind, me, self.spans.expr(v));
                }
            }
            Expression { expr, .. } => {
                self.expr(expr, f, me);
                self.ends_with_child(f, kind, me, self.spans.expr(expr));
            }
            If { condition, then_block, else_block, .. } => {
                self.expr(condition, f, me);
                self.stmts(then_block, f, me);
                if let Some(b) = else_block {
                    self.stmts(b, f, me);
                }
            }
            Match { value, arms, .. } => {
                self.expr(value, f, me);
                for a in arms {
                    if let Some(g) = a.guard {
                        self.expr(g, f, me);
                    }
                    self.expr(a.body, f, me);
                }
            }
            For { iterable, body, .. } => {
                self.expr(iterable, f, me);
                self.stmts(body, f, me);
            }
            While { condition, body, .. } => {
                self.expr(condition, f, me);
                self.stmts(body, f, me);
            }
            Loop { body, .. } | Thread { body, .. } | Async { body, .. } => self.stmts(body, f, me),
            Defer { statement, .. } => {
                let mut p = None;
                self.stmt(statement, f, me, &mut p)
            }
            Break { .. } | Continue { .. } | Use { .. } => {}
        }
    }

    fn item(&self, it: &Item, f: &mut Findings) {
        match it {
            Item::Function { decl, .. } => self.stmts(&decl.body, f, None),
            Item::Impl { block, .. } => {
                for func in &block.functions {
                    self.stmts(&func.body, f, None);
                }
            }
            Item::Const { value, .. } | Item::Static { value, .. } => self.expr(value, f, None),
            Item::Macro { expr, .. } => self.expr(expr, f, None),
            _ => {}
        }
    }
}

fn expr_kind(e: &Expression) -> &'static str {
    use Expression::*;
    match e {
        Literal { .. } => "Literal",
        Identifier { .. } => "Identifier",
        Binary { .. } => "Binary",
        Unary { .. } => "Unary",
        Call { .. } => "Call",
        MethodCall { .. } => "MethodCall",
        FieldAccess { .. } => "FieldAccess",
        StructLiteral { .. } => "StructLiteral",
        MapLiteral { .. } => "MapLiteral",
        Range { .. } => "Range",
        Closure { .. } => "Closure",
        Cast { .. } => "Cast",
        Index { .. } => "Index",
        Tuple { .. } => "Tuple",
        Array { .. } => "Array",
        MacroInvocation { .. } => "MacroInvocation",
        TryOp { .. } => "TryOp",
        Await { .. } => "Await",
        AsyncCall { .. } => "AsyncCall",
        SpawnCall { .. } => "SpawnCall",
        ChannelSend { .. } => "ChannelSend",
        ChannelRecv { .. } => "ChannelRecv",
        Block { .. } => "Block",
    }
}

fn stmt_kind(s: &Statement) -> &'static str {
    use Statement::*;
    match s {
        Let { .. } => "Let",
        Const { .. } => "Const",
        Static { .. } => "Static",
        Assignment { .. } => "Assignment",
        Return { .. } => "Return",
        Expression { .. } => "ExprStmt",
        If { .. } => "If",
        Match { .. } => "Match",
        For { .. } => "For",
        Loop { .. } => "Loop",
        While { .. } => "While",
        Thread { .. } => "Thread",
        Async { .. } => "Async",
        Defer { .. } => "Defer",
        Break { .. } => "Break",
        Continue { .. } => "Continue",
        Use { .. } => "Use",
    }
}

fn item_kind(i: &Item) -> &'static str {
    match i {
        Item::Function { .. } => "Function",
        Item::Struct { .. } => "Struct",
        Item::Enum { .. } => "Enum",
        Item::Trait { .. } => "Trait",
        Item::Impl { .. } => "Impl",
        Item::Const { .. } => "ItemConst",
        Item::Static { .. } => "ItemStatic",
        Item::Macro { .. } => "ItemMacro",
        Item::ExternLet { .. } => "ExternLet",
        Item::Use { .. } => "ItemUse",
        Item::Mod { .. } => "Mod",
        Item::BoundAlias { .. } => "BoundAlias",
        Item::TypeAlias { .. } => "TypeAlias",
    }
}
