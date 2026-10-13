//! Language intrinsics (ADR-0014 Phase 1) for the interpreter.
//!
//! `crate::intrinsics::Intrinsic` is the single name registry: every plain call `name(..)` and
//! every deprecated macro spelling `name!(..)` that the registry knows is evaluated here, and
//! nowhere else. Observable behaviour (stdout/stderr split, panic message text, `debug` output)
//! mirrors the compiled Rust backend. Shapes the interpreter does not model are explicit
//! `Unsupported` errors (exit 2), never a guess.

use super::engine::Interpreter;
use super::error::{semantic, unsupported, R};
use super::eval_expr::expr_loc;
use super::value::Value;
pub(crate) use crate::intrinsics::Intrinsic;
use crate::parser::ast::{BinaryOp, Expression, Literal, UnaryOp};

type Expr = &'static Expression<'static>;

impl Interpreter {
    /// Does a plain call to `name` mean the intrinsic (no local variable or user item shadows it)?
    pub(crate) fn intrinsic_for_call(&self, name: &str) -> Option<Intrinsic> {
        let intr = Intrinsic::from_call_name(name)?;
        let m = &self.modules[self.scope.module];
        if self.lookup_var(name).is_some() || m.funcs.contains_key(name) || m.imports.contains_key(name) {
            return None;
        }
        Some(intr)
    }

    /// Evaluate a registry intrinsic. `whole` is the call/macro node (panic and `debug` location).
    pub(crate) fn eval_intrinsic(&mut self, intr: Intrinsic, whole: Expr, args: &[Expr], is_repeat: bool) -> R {
        match intr {
            Intrinsic::Print | Intrinsic::Println | Intrinsic::Eprint | Intrinsic::Eprintln => {
                let mut s = self.message_text(args, 0)?.unwrap_or_default();
                if intr.appends_newline() {
                    s.push('\n');
                }
                if intr.writes_to_stderr() {
                    self.write_err(&s);
                } else {
                    self.write_out(&s);
                }
                Ok(Value::Unit)
            }
            Intrinsic::Format => {
                let vals = self.eval_all_exprs(args)?;
                Ok(Value::str(self.format_args(&vals)?))
            }
            Intrinsic::Vec => {
                if is_repeat && args.len() == 2 {
                    let x = self.eval(args[0])?;
                    let n = match self.eval(args[1])? {
                        Value::Int(n, _) if n >= 0 => n as usize,
                        _ => return Err(semantic("vec![x; n] needs a non-negative integer length")),
                    };
                    return Ok(Value::vec(vec![x; n]));
                }
                Ok(Value::vec(self.eval_all_exprs(args)?))
            }
            Intrinsic::Panic | Intrinsic::Todo | Intrinsic::Unreachable => {
                let user = self.message_text(args, 0)?;
                let msg = match (intr, user) {
                    (Intrinsic::Panic, None) => "explicit panic".to_string(),
                    (Intrinsic::Panic, Some(m)) => m,
                    (_, None) => intr.default_panic_message().unwrap_or("panic").to_string(),
                    (_, Some(m)) => format!("{}: {}", intr.default_panic_message().unwrap_or("panic"), m),
                };
                Err(self.panic_at(whole, msg))
            }
            Intrinsic::Assert => {
                let Some(cond) = args.first() else {
                    return Err(unsupported("`assert()` without a condition"));
                };
                if matches!(self.eval(cond)?, Value::Bool(true)) {
                    return Ok(Value::Unit);
                }
                let msg = match self.message_text(args, 1)? {
                    Some(m) => m,
                    None => match render_expr(cond) {
                        Some(t) => format!("assertion failed: {}", t),
                        None => "assertion failed".to_string(),
                    },
                };
                Err(self.panic_at(whole, msg))
            }
            Intrinsic::AssertEq | Intrinsic::AssertNe => {
                if args.len() < 2 {
                    return Err(unsupported(format!("`{}` needs two operands", intr.call_name())));
                }
                let a = self.eval(args[0])?;
                let b = self.eval(args[1])?;
                let want_eq = intr == Intrinsic::AssertEq;
                let eq = matches!(self.binary(BinaryOp::Eq, a.clone(), b.clone())?, Value::Bool(true));
                if eq == want_eq {
                    return Ok(Value::Unit);
                }
                let extra = match self.message_text(args, 2)? {
                    Some(m) => format!(": {}", m),
                    None => String::new(),
                };
                let l = self.debug_string(&a)?;
                let r = self.debug_string(&b)?;
                Err(self.panic_at(
                    whole,
                    format!(
                        "assertion `left {} right` failed{}\n  left: {}\n right: {}",
                        if want_eq { "==" } else { "!=" },
                        extra,
                        l,
                        r
                    ),
                ))
            }
            Intrinsic::Debug => {
                if args.len() != 1 {
                    return Err(unsupported("`debug` takes exactly one argument"));
                }
                let v = self.eval(args[0])?;
                let mut pretty = String::new();
                self.debug_fmt(&v, true, 0, &mut pretty)?;
                let at = self.span_of(expr_loc(whole)).map(|s| s.to_string()).unwrap_or_else(|| "<unknown>".into());
                let text = render_expr(args[0]).unwrap_or_else(|| "<expr>".into());
                self.write_err(&format!("[{}] {} = {}\n", at, text, pretty));
                Ok(v)
            }
        }
    }

    /// Message/template arguments starting at `from`: none -> `None`; a single argument is the
    /// finished string (an interpolation already lowered, or a literal printed verbatim);
    /// several arguments are the legacy `{}` template plus its operands.
    fn message_text(&mut self, args: &[Expr], from: usize) -> R<Option<String>> {
        let rest = args.get(from..).unwrap_or(&[]);
        match rest {
            [] => Ok(None),
            [one] => {
                let v = self.eval(one)?;
                Ok(Some(self.display_string(&v)?))
            }
            many => {
                let vals = self.eval_all_exprs(many)?;
                Ok(Some(self.format_args(&vals)?))
            }
        }
    }

    fn eval_all_exprs(&mut self, args: &[Expr]) -> R<Vec<Value>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            out.push(self.eval(a)?);
        }
        Ok(out)
    }

    /// Plain call `name(args)` for a registry intrinsic.
    pub(crate) fn eval_intrinsic_call(
        &mut self,
        intr: Intrinsic,
        whole: Expr,
        arguments: &'static [(Option<String>, Expr)],
    ) -> R {
        if arguments.iter().any(|(label, _)| label.is_some()) {
            return Err(unsupported(format!("named arguments in `{}(..)`", intr.call_name())));
        }
        let args: Vec<Expr> = arguments.iter().map(|(_, e)| *e).collect();
        self.eval_intrinsic(intr, whole, &args, false)
    }
}

/// Source-like text of an expression as rustc's `stringify!` prints it (used by
/// `assertion failed: <cond>` and `debug`). `None` for shapes we do not render.
fn render_expr(e: Expr) -> Option<String> {
    Some(match e {
        Expression::Identifier { name, .. } => name.clone(),
        Expression::Literal { value, .. } => match value {
            Literal::Int(i) => i.to_string(),
            Literal::IntSuffixed(i, s) => format!("{}{}", i, s),
            Literal::Float(f) => {
                let s = format!("{:?}", f);
                s
            }
            Literal::String(s) => format!("{:?}", s),
            Literal::Char(c) => format!("{:?}", c),
            Literal::Bool(b) => b.to_string(),
        },
        Expression::Binary { left, op, right, .. } => {
            format!("{} {} {}", render_expr(left)?, binop_text(*op), render_expr(right)?)
        }
        Expression::Unary { op, operand, .. } => {
            let p = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
                UnaryOp::Ref => "&",
                UnaryOp::MutRef => "&mut ",
                UnaryOp::Deref => "*",
            };
            format!("{}{}", p, render_expr(operand)?)
        }
        Expression::FieldAccess { object, field, .. } => format!("{}.{}", render_expr(object)?, field),
        Expression::MethodCall { object, method, arguments, type_args: None, .. } => {
            format!("{}.{}({})", render_expr(object)?, method, render_args(arguments)?)
        }
        Expression::Call { function, arguments, .. } => {
            format!("{}({})", render_expr(function)?, render_args(arguments)?)
        }
        Expression::Index { object, index, .. } => format!("{}[{}]", render_expr(object)?, render_expr(index)?),
        _ => return None,
    })
}

fn render_args(args: &[(Option<String>, Expr)]) -> Option<String> {
    let parts: Option<Vec<String>> = args.iter().map(|(_, a)| render_expr(a)).collect();
    Some(parts?.join(", "))
}

fn binop_text(op: BinaryOp) -> &'static str {
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
