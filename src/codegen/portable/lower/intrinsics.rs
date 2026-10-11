//! Lowering of language intrinsics (ADR-0014 Phase 1): print / assert / panic / todo /
//! unreachable / debug / `format` / list literal.
//!
//! This is the only place in the portable layer that knows what an intrinsic *is*: names come from
//! `crate::intrinsics::Intrinsic`. Everything is desugared to existing IR (`Print`, `Format`,
//! `Panic`, `If`, `Block`), so the Go and JS/TS emitters need no per-intrinsic code and cannot
//! disagree about stdout/stderr routing, messages or evaluation order.

use super::{Lowerer, PResult};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;
use crate::intrinsics::Intrinsic;
use crate::parser::{BinaryOp, Expression, Literal, UnaryOp};

fn debug_spec() -> FmtSpec {
    FmtSpec { debug: true, ..FmtSpec::default() }
}

fn lit(s: &str) -> FmtPiece {
    FmtPiece::Lit(s.to_string())
}

fn bin_symbol(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
    }
}

/// Source-like text of an expression for `debug(x)` (`[file:line:col] <label> = value`).
fn label(e: &Expression) -> String {
    let list = |args: &[(Option<String>, &Expression)]| args.iter().map(|(_, a)| label(a)).collect::<Vec<_>>().join(", ");
    match e {
        Expression::Identifier { name, .. } => name.clone(),
        Expression::Literal { value, .. } => match value {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => format!("{s:?}"),
            Literal::Bool(b) => b.to_string(),
            Literal::Char(c) => format!("{c:?}"),
            Literal::IntSuffixed(n, s) => format!("{n}{s}"),
        },
        Expression::FieldAccess { object, field, .. } => format!("{}.{field}", label(object)),
        Expression::Index { object, index, .. } => format!("{}[{}]", label(object), label(index)),
        Expression::Call { function, arguments, .. } => format!("{}({})", label(function), list(arguments)),
        Expression::MethodCall { object, method, arguments, .. } => format!("{}.{method}({})", label(object), list(arguments)),
        Expression::Binary { left, op, right, .. } => format!("{} {} {}", label(left), bin_symbol(op), label(right)),
        Expression::Unary { op: UnaryOp::Neg, operand, .. } => format!("-{}", label(operand)),
        Expression::Unary { op: UnaryOp::Not, operand, .. } => format!("!{}", label(operand)),
        _ => "<expression>".to_string(),
    }
}

impl<'e> Lowerer<'e> {
    /// A user function or local named like an intrinsic wins (matches the Rust backend).
    pub(super) fn shadows_intrinsic(&self, name: &str) -> bool {
        self.env.fns.contains_key(name) || self.lookup(name).is_some()
    }

    pub(super) fn intrinsic(&mut self, intr: Intrinsic, args: &[&Expression], is_repeat: bool, want: Option<&Ty>) -> PResult<PExpr> {
        match intr {
            Intrinsic::Print | Intrinsic::Println | Intrinsic::Eprint | Intrinsic::Eprintln => {
                let pieces = self.message_pieces(args)?;
                Ok(PExpr::new(Ty::Unit, PK::Print { newline: intr.appends_newline(), to_stderr: intr.writes_to_stderr(), pieces }))
            }
            Intrinsic::Format => {
                let pieces = self.template_pieces(args)?;
                Ok(PExpr::new(Ty::Str, PK::Format(pieces)))
            }
            Intrinsic::Panic | Intrinsic::Todo | Intrinsic::Unreachable => self.diverge(intr, args),
            Intrinsic::Assert => self.assert_cond(args),
            Intrinsic::AssertEq | Intrinsic::AssertNe => self.assert_compare(intr, args),
            Intrinsic::Debug => self.debug_value(args, want),
            Intrinsic::Vec => self.list_literal(args, is_repeat, want),
        }
    }

    /// `template, args..` (legacy `{}` template) or a single message expression.
    fn template_pieces(&mut self, args: &[&Expression]) -> PResult<Vec<FmtPiece>> {
        match args.first() {
            None => Ok(vec![]),
            Some(Expression::Literal { value: Literal::String(s), .. }) => self.format_pieces(s, &args[1..]),
            Some(other) if args.len() == 1 => {
                let v = self.expr(other, None)?;
                match *v.kind {
                    PK::Format(pieces) => Ok(pieces),
                    kind => Ok(vec![FmtPiece::Arg(PExpr { kind: Box::new(kind), ..v }, FmtSpec::default())]),
                }
            }
            Some(_) => self.err("a format template must be a string literal"),
        }
    }

    /// Message of print/panic/assert: a lone string literal prints verbatim (`println("{}")`
    /// prints `{}`); interpolated strings arrive as a `format` node; a literal followed by
    /// arguments is the deprecated `{}` template.
    fn message_pieces(&mut self, args: &[&Expression]) -> PResult<Vec<FmtPiece>> {
        match args {
            [Expression::Literal { value: Literal::String(s), .. }] => Ok(vec![lit(s)]),
            _ => self.template_pieces(args),
        }
    }

    fn panic_stmt(pieces: Vec<FmtPiece>) -> PStmt {
        PStmt::Expr(PExpr::new(Ty::Unknown, PK::Panic(PExpr::new(Ty::Str, PK::Format(pieces)))))
    }

    fn diverge(&mut self, intr: Intrinsic, args: &[&Expression]) -> PResult<PExpr> {
        let user = self.message_pieces(args)?;
        let pieces = match (intr.default_panic_message(), user.is_empty()) {
            (None, true) => vec![lit("explicit panic")],
            (None, false) => user,
            (Some(base), true) => vec![lit(base)],
            (Some(base), false) => std::iter::once(lit(&format!("{base}: "))).chain(user).collect(),
        };
        let msg = PExpr::new(Ty::Str, PK::Format(pieces));
        Ok(PExpr::new(Ty::Unknown, PK::Panic(msg)))
    }

    fn fail_unless(cond: PExpr, then: Vec<PStmt>) -> PStmt {
        let not = PExpr::new(Ty::Bool, PK::Unary(UnKind::Not, cond));
        PStmt::Expr(PExpr::new(Ty::Unit, PK::If { cond: not, then: PBlock::of(then), els: PBlock::default() }))
    }

    fn assert_cond(&mut self, args: &[&Expression]) -> PResult<PExpr> {
        let Some(first) = args.first() else { return self.err("assert without condition") };
        let cond = self.expr(first, Some(&Ty::Bool))?;
        let user = self.message_pieces(&args[1..])?;
        let pieces = if user.is_empty() { vec![lit("assertion failed")] } else { user };
        Ok(PExpr::new(Ty::Unit, PK::Block(PBlock::of(vec![Self::fail_unless(cond, vec![Self::panic_stmt(pieces)])]))))
    }

    /// `assert_eq(a, b[, msg])`: operands are evaluated once, failure prints both with `{:?}`.
    fn assert_compare(&mut self, intr: Intrinsic, args: &[&Expression]) -> PResult<PExpr> {
        if args.len() < 2 {
            return self.err(format!("{} needs two arguments", intr.call_name()));
        }
        let a = self.expr(args[0], None)?;
        let b = self.expr(args[1], Some(&a.ty))?;
        self.constrain(&a.ty, &b.ty);
        let (lname, rname) = (self.fresh("wj_left"), self.fresh("wj_right"));
        let (lt, rt) = (a.ty.clone(), b.ty.clone());
        let (l, r) = (PExpr::new(lt.clone(), PK::Local(lname.clone())), PExpr::new(rt.clone(), PK::Local(rname.clone())));
        let (op, sym) = if intr == Intrinsic::AssertEq { (BinKind::Eq, "==") } else { (BinKind::Ne, "!=") };
        let cond = PExpr::new(Ty::Bool, PK::Binary(op, l.clone(), r.clone()));
        let user = self.message_pieces(&args[2..])?;
        let mut pieces = vec![lit(&format!("assertion `left {sym} right` failed"))];
        if !user.is_empty() {
            pieces.push(lit(": "));
            pieces.extend(user);
        }
        pieces.extend([lit("\n  left: "), FmtPiece::Arg(l, debug_spec()), lit("\n right: "), FmtPiece::Arg(r, debug_spec())]);
        let stmts = vec![
            PStmt::Let { name: lname, ty: lt, mutable: false, init: Some(a) },
            PStmt::Let { name: rname, ty: rt, mutable: false, init: Some(b) },
            Self::fail_unless(cond, vec![Self::panic_stmt(pieces)]),
        ];
        Ok(PExpr::new(Ty::Unit, PK::Block(PBlock::of(stmts))))
    }

    /// `debug(x)`: stderr gets `[file:line:col] label = value`, the (copied) value is returned.
    fn debug_value(&mut self, args: &[&Expression], want: Option<&Ty>) -> PResult<PExpr> {
        let [arg] = args else { return self.err("debug takes exactly one argument") };
        let v = self.expr(arg, want)?;
        let v = self.own(v);
        let ty = v.ty.clone();
        let loc = match arg.location() {
            Some(l) => format!("{}:{}:{}", l.file.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(), l.line, l.column),
            None => format!("line {}", self.line),
        };
        let name = self.fresh("wj_dbg");
        let shown = PExpr::new(ty.clone(), PK::Local(name.clone()));
        let pieces = vec![lit(&format!("[{loc}] {} = ", label(arg))), FmtPiece::Arg(shown.clone(), debug_spec())];
        let print = PExpr::new(Ty::Unit, PK::Print { newline: true, to_stderr: true, pieces });
        let stmts = vec![PStmt::Let { name, ty: ty.clone(), mutable: false, init: Some(v) }, PStmt::Expr(print)];
        Ok(PExpr::new(ty, PK::Block(PBlock { stmts, tail: Some(shown) })))
    }

    /// `[a, b, c]` / `vec![..]` / `[x; n]`.
    fn list_literal(&mut self, args: &[&Expression], is_repeat: bool, want: Option<&Ty>) -> PResult<PExpr> {
        if !is_repeat {
            return self.vec_literal(args, want);
        }
        let elem_want = match want {
            Some(Ty::Vec(t)) => Some((**t).clone()),
            _ => None,
        };
        let elem = self.expr(args[0], elem_want.as_ref())?;
        let n = self.expr(args[1], Some(&Ty::usize()))?;
        let n = self.ensure_usize(n);
        Ok(PExpr::new(Ty::Vec(Box::new(elem.ty.clone())), PK::VecRepeat(self.own(elem), n)))
    }
}
