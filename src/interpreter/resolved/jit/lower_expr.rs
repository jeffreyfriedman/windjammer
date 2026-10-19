//! Expressions, assignments and effects of a loop unit (`lower.rs`), with the walker's semantics:
//! `ops::int_binary` (checked arithmetic in the result width, a literal adopting the other side's
//! width), `float_binary`, `cast`, `coerce` and `retag`. Anything outside the inline paths either
//! calls the walker's routine (`rt::int_op`) or refuses the loop.

use super::lower::{Bind, Lo, Val, E};
use super::rt;
use super::kinds::{code, full_i64, int_ok, mutant, unsigned_repr, Class, Elem, Shape, Ty};
use crate::interpreter::int::IntTy;
use crate::interpreter::ops::compound_to_binary;
use crate::interpreter::value::{type_is_float, FloatTy};
use crate::parser::{BinaryOp, CompoundOp, Expression, Literal, Type, UnaryOp};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{types, InstBuilder, Value as V};

fn is_cmp(op: BinaryOp) -> bool {
    matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge)
}

fn icc(op: BinaryOp, signed: bool) -> Option<IntCC> {
    Some(match (op, signed) {
        (BinaryOp::Eq, _) => IntCC::Equal,
        (BinaryOp::Ne, _) => IntCC::NotEqual,
        (BinaryOp::Lt, true) => IntCC::SignedLessThan,
        (BinaryOp::Le, true) => IntCC::SignedLessThanOrEqual,
        (BinaryOp::Gt, true) => IntCC::SignedGreaterThan,
        (BinaryOp::Ge, true) => IntCC::SignedGreaterThanOrEqual,
        (BinaryOp::Lt, false) => IntCC::UnsignedLessThan,
        (BinaryOp::Le, false) => IntCC::UnsignedLessThanOrEqual,
        (BinaryOp::Gt, false) => IntCC::UnsignedGreaterThan,
        (BinaryOp::Ge, false) => IntCC::UnsignedGreaterThanOrEqual,
        _ => return None,
    })
}

/// A place a statement stores to: a `Vec` element, a field of one, or a field of a struct local.
enum Target {
    Index(usize, Elem, Val),
    IndexField(usize, Shape, Val, &'static str),
    Field(usize, Shape, &'static str),
}

impl Lo<'_, '_> {
    fn k(&mut self, x: i64) -> V {
        self.b.ins().iconst(types::I64, x)
    }

    pub(super) fn cond(&mut self, e: E) -> Option<V> {
        let v = self.expr(e)?;
        (v.ty == Ty::Bool).then_some(v.v)
    }

    pub(super) fn expr(&mut self, e: E) -> Option<Val> {
        let r = self.expr_in(e);
        if r.is_none() {
            self.refused(|| format!("{e:?}"));
        }
        r
    }

    fn expr_in(&mut self, e: E) -> Option<Val> {
        Some(match e {
            Expression::Literal { value, .. } => self.literal(value)?,
            Expression::Identifier { name, .. } => match self.lookup(name)?.clone() {
                Bind::Var(id) => Val::of(self.b.use_var(self.vars[id].var), self.vars[id].ty),
                Bind::Handle(..) => return None,
            },
            Expression::Binary { left, op: op @ (BinaryOp::And | BinaryOp::Or), right, .. } => self.logic(*op, left, right)?,
            Expression::Binary { left, op, right, .. } => {
                let (l, r) = (self.expr(left)?, self.expr(right)?);
                self.binary(*op, l, r)?
            }
            Expression::Unary { op, operand, .. } if !matches!(operand, Expression::Range { .. }) => self.unary(*op, operand)?,
            Expression::Cast { expr, type_, .. } => {
                let v = self.expr(expr)?;
                self.cast(v, type_)?
            }
            Expression::Index { .. } | Expression::FieldAccess { .. } => {
                let t = self.target(e)?;
                self.load(&t)?
            }
            Expression::MethodCall { object, method, type_args: None, arguments, .. } if arguments.is_empty() => self.method(object, method)?,
            _ => return None,
        })
    }

    fn literal(&mut self, l: &Literal) -> Option<Val> {
        Some(match l {
            Literal::Int(i) => Val { v: self.k(*i), ty: Ty::Int(IntTy::Lit), k: Some(*i) },
            Literal::IntSuffixed(i, suf) => match suf.as_str() {
                "f64" => Val::of(self.b.ins().f64const(*i as f64), Ty::Flt(FloatTy::F64)),
                "f32" => return None,
                s => {
                    let t = IntTy::from_name(s).unwrap_or(IntTy::Lit);
                    if !int_ok(t) {
                        return None;
                    }
                    Val { v: self.k(*i), ty: Ty::Int(t), k: (t == IntTy::Lit).then_some(*i) }
                }
            },
            Literal::Float(f) => Val::of(self.b.ins().f64const(*f), Ty::Flt(FloatTy::Lit)),
            Literal::Bool(x) => Val::of(self.b.ins().iconst(types::I8, *x as i64), Ty::Bool),
            _ => return None,
        })
    }

    /// `&&` / `||`: the right side only when the left one does not decide.
    fn logic(&mut self, op: BinaryOp, left: E, right: E) -> Option<Val> {
        let l = self.cond(left)?;
        let (rhs, join) = (self.b.create_block(), self.b.create_block());
        self.b.append_block_param(join, types::I8);
        if op == BinaryOp::And {
            self.b.ins().brif(l, rhs, &[], join, &[l.into()]);
        } else {
            self.b.ins().brif(l, join, &[l.into()], rhs, &[]);
        }
        self.b.switch_to_block(rhs);
        self.b.seal_block(rhs);
        let r = self.cond(right)?;
        self.b.ins().jump(join, &[r.into()]);
        self.b.switch_to_block(join);
        self.b.seal_block(join);
        Some(Val::of(self.b.block_params(join)[0], Ty::Bool))
    }

    /// The walker's `binary` on two scalars.
    pub(super) fn binary(&mut self, op: BinaryOp, l: Val, r: Val) -> Option<Val> {
        match (l.ty, r.ty) {
            (Ty::Int(_), Ty::Int(_)) if is_cmp(op) => self.int_cmp(op, l, r),
            (Ty::Int(_), Ty::Int(_)) => self.int_arith(op, l, r),
            (Ty::Flt(a), Ty::Flt(b)) => self.flt(op, l.v, r.v, if a == FloatTy::Lit { b } else { a }),
            (Ty::Flt(a), Ty::Int(IntTy::Lit)) => {
                let y = self.b.ins().fcvt_from_sint(types::F64, r.v);
                self.flt(op, l.v, y, a)
            }
            (Ty::Int(IntTy::Lit), Ty::Flt(b)) => {
                let x = self.b.ins().fcvt_from_sint(types::F64, l.v);
                self.flt(op, x, r.v, b)
            }
            (Ty::Bool, Ty::Bool) => {
                let v = match op {
                    BinaryOp::BitAnd => self.b.ins().band(l.v, r.v),
                    BinaryOp::BitOr => self.b.ins().bor(l.v, r.v),
                    BinaryOp::BitXor => self.b.ins().bxor(l.v, r.v),
                    op => self.b.ins().icmp(icc(op, false)?, l.v, r.v),
                };
                Some(Val::of(v, Ty::Bool))
            }
            _ => None,
        }
    }

    fn flt(&mut self, op: BinaryOp, x: V, y: V, t: FloatTy) -> Option<Val> {
        let cc = match op {
            BinaryOp::Eq => Some(FloatCC::Equal),
            BinaryOp::Ne if mutant("nan") => Some(FloatCC::OrderedNotEqual),
            BinaryOp::Ne => Some(FloatCC::NotEqual),
            BinaryOp::Lt => Some(FloatCC::LessThan),
            BinaryOp::Le => Some(FloatCC::LessThanOrEqual),
            BinaryOp::Gt => Some(FloatCC::GreaterThan),
            BinaryOp::Ge => Some(FloatCC::GreaterThanOrEqual),
            _ => None,
        };
        if let Some(cc) = cc {
            return Some(Val::of(self.b.ins().fcmp(cc, x, y), Ty::Bool));
        }
        let v = match op {
            BinaryOp::Add => self.b.ins().fadd(x, y),
            BinaryOp::Sub => self.b.ins().fsub(x, y),
            BinaryOp::Mul => self.b.ins().fmul(x, y),
            BinaryOp::Div => self.b.ins().fdiv(x, y),
            BinaryOp::Mod => self.call("fmod", rt::fmod as *const () as usize, &[x, y], true, false),
            _ => return None,
        };
        Some(Val::of(v, Ty::Flt(t)))
    }

    /// Integer comparison of the walker's `i128` values.
    fn int_cmp(&mut self, op: BinaryOp, a: Val, b: Val) -> Option<Val> {
        let (Ty::Int(ta), Ty::Int(tb)) = (a.ty, b.ty) else { return None };
        let (ua, ub) = (unsigned_repr(ta), unsigned_repr(tb));
        if ua == ub {
            return Some(Val::of(self.b.ins().icmp(icc(op, !ua)?, a.v, b.v), Ty::Bool));
        }
        // one side signed: when it is negative it is below every unsigned value
        let (s, s_left) = if ua { (b.v, false) } else { (a.v, true) };
        let when_neg = match op {
            BinaryOp::Eq => false,
            BinaryOp::Ne => true,
            BinaryOp::Lt | BinaryOp::Le => s_left,
            _ => !s_left,
        };
        let neg = self.b.ins().icmp_imm(IntCC::SignedLessThan, s, 0);
        let u = self.b.ins().icmp(icc(op, false)?, a.v, b.v);
        let k = self.b.ins().iconst(types::I8, when_neg as i64);
        Some(Val::of(self.b.ins().select(neg, k, u), Ty::Bool))
    }

    /// Leave unless `r` is in the range of `t` (widths narrower than 64 bits).
    fn range(&mut self, r: V, t: IntTy) {
        if full_i64(t) || unsigned_repr(t) || mutant("range") {
            return;
        }
        let lo = self.b.ins().icmp_imm(IntCC::SignedLessThan, r, t.min() as i64);
        let hi = self.b.ins().icmp_imm(IntCC::SignedGreaterThan, r, t.max() as i64);
        let bad = self.b.ins().bor(lo, hi);
        self.bail_if(bad);
    }

    /// `ops::int_binary` for `+ - * / % & | ^`.
    fn int_arith(&mut self, op: BinaryOp, a: Val, b: Val) -> Option<Val> {
        let (Ty::Int(ta), Ty::Int(tb)) = (a.ty, b.ty) else { return None };
        let opi = rt::OPS.iter().position(|o| *o == op)?;
        let t = IntTy::unify(ta, tb);
        let (x, y) = (a.v, b.v);
        let bitwise = matches!(op, BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor);
        if !unsigned_repr(ta) && !unsigned_repr(tb) {
            let r = match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                    let (r, of) = match op {
                        BinaryOp::Add => self.b.ins().sadd_overflow(x, y),
                        BinaryOp::Sub => self.b.ins().ssub_overflow(x, y),
                        _ => self.b.ins().smul_overflow(x, y),
                    };
                    if !mutant("overflow") {
                        self.bail_if(of);
                    }
                    r
                }
                BinaryOp::Div | BinaryOp::Mod => {
                    let z = self.b.ins().icmp_imm(IntCC::Equal, y, 0);
                    if !mutant("divzero") {
                        self.bail_if(z);
                    }
                    let m = self.b.ins().icmp_imm(IntCC::Equal, x, i64::MIN);
                    let n = self.b.ins().icmp_imm(IntCC::Equal, y, -1);
                    let both = self.b.ins().band(m, n);
                    self.bail_if(both);
                    if op == BinaryOp::Div {
                        self.b.ins().sdiv(x, y)
                    } else {
                        self.b.ins().srem(x, y)
                    }
                }
                BinaryOp::BitAnd => self.b.ins().band(x, y),
                BinaryOp::BitOr => self.b.ins().bor(x, y),
                _ => self.b.ins().bxor(x, y),
            };
            if !bitwise {
                self.range(r, t);
            }
            return Some(Val::of(r, Ty::Int(t)));
        }
        let nonneg = |v: &Val, t: IntTy| unsigned_repr(t) || (t == IntTy::Lit && v.k.is_some_and(|k| k >= 0));
        if nonneg(&a, ta) && nonneg(&b, tb) {
            // both values are in 0..=u64::MAX and so is every result that fits `u64`/`usize`
            let r = match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul => {
                    let (r, of) = match op {
                        BinaryOp::Add => self.b.ins().uadd_overflow(x, y),
                        BinaryOp::Sub => self.b.ins().usub_overflow(x, y),
                        _ => self.b.ins().umul_overflow(x, y),
                    };
                    self.bail_if(of);
                    r
                }
                BinaryOp::Div | BinaryOp::Mod => {
                    let z = self.b.ins().icmp_imm(IntCC::Equal, y, 0);
                    self.bail_if(z);
                    if op == BinaryOp::Div {
                        self.b.ins().udiv(x, y)
                    } else {
                        self.b.ins().urem(x, y)
                    }
                }
                BinaryOp::BitAnd => self.b.ins().band(x, y),
                BinaryOp::BitOr => self.b.ins().bor(x, y),
                _ => self.b.ins().bxor(x, y),
            };
            return Some(Val::of(r, Ty::Int(t)));
        }
        // any other width combination: the walker's routine
        let args = [opi as i64, code(a.ty), code(b.ty), code(Ty::Int(t))].map(|c| self.k(c));
        let r = self.call("int_op", rt::int_op as *const () as usize, &[args[0], x, args[1], y, args[2], args[3]], false, true);
        Some(Val::of(r, Ty::Int(t)))
    }

    fn unary(&mut self, op: UnaryOp, operand: E) -> Option<Val> {
        match op {
            UnaryOp::Neg => {
                let v = self.expr(operand)?;
                match v.ty {
                    Ty::Int(t) if unsigned_repr(t) => {
                        let nz = self.b.ins().icmp_imm(IntCC::NotEqual, v.v, 0);
                        self.bail_if(nz);
                        Some(v)
                    }
                    Ty::Int(t) => {
                        let m = self.b.ins().icmp_imm(IntCC::Equal, v.v, i64::MIN);
                        self.bail_if(m);
                        let r = self.b.ins().ineg(v.v);
                        self.range(r, t);
                        Some(Val { v: r, ty: v.ty, k: v.k.and_then(i64::checked_neg) })
                    }
                    Ty::Flt(_) => Some(Val::of(self.b.ins().fneg(v.v), v.ty)),
                    Ty::Bool => None,
                }
            }
            UnaryOp::Not => {
                let v = self.expr(operand)?;
                let r = match v.ty {
                    Ty::Bool => self.b.ins().bxor_imm(v.v, 1),
                    Ty::Int(t) if t.signed() || unsigned_repr(t) => self.b.ins().bnot(v.v),
                    Ty::Int(t) => self.b.ins().bxor_imm(v.v, t.max() as i64),
                    Ty::Flt(_) => return None,
                };
                Some(Val::of(r, v.ty))
            }
            UnaryOp::Deref | UnaryOp::Ref => self.expr(operand),
            UnaryOp::MutRef => None,
        }
    }

    /// The walker's `as`.
    fn cast(&mut self, v: Val, ty: &Type) -> Option<Val> {
        let ty = self.it.resolve_alias(ty).clone();
        if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
            return None;
        }
        if let Some(t) = IntTy::from_type(&ty) {
            if !int_ok(t) || t == IntTy::Lit {
                return None;
            }
            let r = match v.ty {
                Ty::Int(_) if t.bits() == 64 => v.v,
                Ty::Int(_) => {
                    let narrow = types::Type::int(t.bits() as u16)?;
                    let r = self.b.ins().ireduce(narrow, v.v);
                    if t.signed() {
                        self.b.ins().sextend(types::I64, r)
                    } else {
                        self.b.ins().uextend(types::I64, r)
                    }
                }
                Ty::Flt(_) if unsigned_repr(t) => self.b.ins().fcvt_to_uint_sat(types::I64, v.v),
                Ty::Flt(_) if full_i64(t) => self.b.ins().fcvt_to_sint_sat(types::I64, v.v),
                Ty::Flt(_) if t.signed() => {
                    let s = self.b.ins().fcvt_to_sint_sat(types::I64, v.v);
                    let (lo, hi) = (self.k(t.min() as i64), self.k(t.max() as i64));
                    let s = self.b.ins().smax(s, lo);
                    self.b.ins().smin(s, hi)
                }
                Ty::Flt(_) => {
                    let u = self.b.ins().fcvt_to_uint_sat(types::I64, v.v);
                    let hi = self.k(t.max() as i64);
                    self.b.ins().umin(u, hi)
                }
                Ty::Bool => self.b.ins().uextend(types::I64, v.v),
            };
            return Some(Val::of(r, Ty::Int(t)));
        }
        if type_is_float(&ty) != Some(FloatTy::F64) {
            return None;
        }
        let r = match v.ty {
            Ty::Int(t) if unsigned_repr(t) => self.b.ins().fcvt_from_uint(types::F64, v.v),
            Ty::Int(_) => self.b.ins().fcvt_from_sint(types::F64, v.v),
            Ty::Flt(_) => v.v,
            Ty::Bool => return None,
        };
        Some(Val::of(r, Ty::Flt(FloatTy::F64)))
    }

    /// The walker's `coerce` of a `let x: T = v`.
    pub(super) fn coerce(&mut self, v: Val, ty: &Type) -> Option<Val> {
        let ty = self.it.resolve_alias(ty).clone();
        if matches!(ty, Type::Reference(_) | Type::MutableReference(_)) {
            return None;
        }
        if let Some(t) = IntTy::from_type(&ty) {
            if !int_ok(t) {
                return None;
            }
            return Some(match v.ty {
                // a literal must fit (a negative one wraps into a 64-bit unsigned width)
                Ty::Int(IntTy::Lit) => {
                    self.range(v.v, t);
                    Val::of(v.v, Ty::Int(t))
                }
                // another width is relabelled as is: the value must be representable
                Ty::Int(s) => {
                    if unsigned_repr(s) != unsigned_repr(t) {
                        let neg = self.b.ins().icmp_imm(IntCC::SignedLessThan, v.v, 0);
                        self.bail_if(neg);
                    }
                    Val::of(v.v, Ty::Int(t))
                }
                _ => v,
            });
        }
        match type_is_float(&ty) {
            Some(FloatTy::F64) => Some(match v.ty {
                Ty::Int(IntTy::Lit) => Val::of(self.b.ins().fcvt_from_sint(types::F64, v.v), Ty::Flt(FloatTy::F64)),
                Ty::Flt(_) => Val::of(v.v, Ty::Flt(FloatTy::F64)),
                _ => v,
            }),
            Some(_) => None,
            None if ty == Type::Bool => Some(v),
            None => None,
        }
    }

    /// The walker's `retag` of `x = v` where `x` has type `old`: a literal keeps the old width.
    fn retag(&mut self, old: Ty, v: Val) -> Option<Val> {
        Some(match (old, v.ty) {
            (Ty::Int(ot), Ty::Int(IntTy::Lit)) if ot != IntTy::Lit => {
                // a literal that does not fit stays a literal: that is the closure engine's
                if mutant("retag") {
                } else if unsigned_repr(ot) {
                    let neg = self.b.ins().icmp_imm(IntCC::SignedLessThan, v.v, 0);
                    self.bail_if(neg);
                } else {
                    self.range(v.v, ot);
                }
                Val::of(v.v, Ty::Int(ot))
            }
            (Ty::Flt(ot), Ty::Flt(FloatTy::Lit)) => Val::of(v.v, Ty::Flt(ot)),
            (Ty::Flt(ot), Ty::Int(IntTy::Lit)) => Val::of(self.b.ins().fcvt_from_sint(types::F64, v.v), Ty::Flt(ot)),
            _ => v,
        })
    }

    /// `x.len()` of a `Vec` local; `sqrt`/`abs`/`floor`/`ceil`/`trunc`/`round` of a float.
    fn method(&mut self, object: E, method: &str) -> Option<Val> {
        if method == "len" {
            let Expression::Identifier { name, .. } = object else { return None };
            let Bind::Handle(slot, Class::Vec(_)) = self.lookup(name)?.clone() else { return None };
            let s = self.k(slot as i64);
            let r = self.call("len", rt::len as *const () as usize, &[s], false, true);
            return Some(Val::of(r, Ty::Int(IntTy::Usize)));
        }
        let v = self.expr(object)?;
        let Ty::Flt(_) = v.ty else { return None };
        let r = match method {
            "sqrt" => self.b.ins().sqrt(v.v),
            "abs" => self.b.ins().fabs(v.v),
            "floor" => self.b.ins().floor(v.v),
            "ceil" => self.b.ins().ceil(v.v),
            "trunc" => self.b.ins().trunc(v.v),
            "round" => self.call("round", rt::round as *const () as usize, &[v.v], true, false),
            _ => return None,
        };
        Some(Val::of(r, v.ty))
    }

    /// An expression statement: `v.push(x)` on a `Vec` local.
    pub(super) fn effect(&mut self, e: E) -> Option<()> {
        let Expression::MethodCall { object: Expression::Identifier { name, .. }, method, type_args: None, arguments, .. } = e else { return None };
        let [(None, arg)] = arguments.as_slice() else { return None };
        if method != "push" {
            return None;
        }
        let Bind::Handle(slot, Class::Vec(_)) = self.lookup(name)?.clone() else { return None };
        let x = self.expr(arg)?;
        let bits = self.to_bits(x.v, x.ty);
        let (s, t) = (self.k(slot as i64), self.k(code(x.ty)));
        self.call("push", rt::push as *const () as usize, &[s, bits, t], false, true);
        Some(())
    }

    /// `v[i]`, `v[i].f` or `s.f` on locals holding a `Vec` / struct.
    fn target(&mut self, e: E) -> Option<Target> {
        let handle = |lo: &Self, e: E| match e {
            Expression::Identifier { name, .. } => match lo.lookup(name)? {
                Bind::Handle(slot, c) => Some((*slot, c.clone())),
                _ => None,
            },
            _ => None,
        };
        Some(match e {
            Expression::Index { object, index, .. } => {
                let (slot, Class::Vec(elem)) = handle(self, object)? else { return None };
                let i = self.expr(index)?;
                matches!(i.ty, Ty::Int(_)).then_some(())?;
                Target::Index(slot, elem, i)
            }
            Expression::FieldAccess { object: Expression::Index { object, index, .. }, field, .. } => {
                let (slot, Class::Vec(Elem::Struct(shape))) = handle(self, object)? else { return None };
                let i = self.expr(index)?;
                matches!(i.ty, Ty::Int(_)).then_some(())?;
                Target::IndexField(slot, shape, i, field.as_str())
            }
            Expression::FieldAccess { object, field, .. } => {
                let (slot, Class::Struct(shape)) = handle(self, object)? else { return None };
                Target::Field(slot, shape, field.as_str())
            }
            _ => return None,
        })
    }

    /// Read a target (its type is the one seen at entry; another one leaves the loop).
    fn load(&mut self, t: &Target) -> Option<Val> {
        let (r, ty) = match t {
            Target::Index(slot, Elem::Scalar(ty), i) => {
                let a = [*slot as i64, code(i.ty), code(*ty)].map(|c| self.k(c));
                (self.call("index_get", rt::index_get as *const () as usize, &[a[0], i.v, a[1], a[2]], false, true), *ty)
            }
            Target::IndexField(slot, shape, i, f) => {
                let (hint, ty) = shape.field(f)?;
                let id = shape.fields[hint].0.as_ptr() as i64;
                let a = [*slot as i64, code(i.ty), f.as_ptr() as i64, f.len() as i64, hint as i64, id, code(ty)].map(|c| self.k(c));
                (self.call("index_field_get", rt::index_field_get as *const () as usize, &[a[0], i.v, a[1], a[2], a[3], a[4], a[5], a[6]], false, true), ty)
            }
            Target::Field(slot, shape, f) => {
                let (hint, ty) = shape.field(f)?;
                let id = shape.fields[hint].0.as_ptr() as i64;
                let a = [*slot as i64, f.as_ptr() as i64, f.len() as i64, hint as i64, id, code(ty)].map(|c| self.k(c));
                (self.call("field_get", rt::field_get as *const () as usize, &a, false, true), ty)
            }
            _ => return None,
        };
        Some(Val::of(self.from_bits(r, ty), ty))
    }

    /// `=` (the helper `retag`s) of `x` into a target.
    fn store(&mut self, t: &Target, x: Val) -> Option<()> {
        let bits = self.to_bits(x.v, x.ty);
        let xt = self.k(code(x.ty));
        match t {
            Target::Index(slot, _, i) => {
                let a = [*slot as i64, code(i.ty)].map(|c| self.k(c));
                self.call("index_set", rt::index_set as *const () as usize, &[a[0], i.v, a[1], bits, xt], false, true);
            }
            Target::IndexField(slot, shape, i, f) => {
                let hint = shape.fields.iter().position(|(n, _)| &**n == *f)?;
                let id = shape.fields[hint].0.as_ptr() as i64;
                let a = [*slot as i64, code(i.ty), f.as_ptr() as i64, f.len() as i64, hint as i64, id].map(|c| self.k(c));
                self.call("index_field_set", rt::index_field_set as *const () as usize, &[a[0], i.v, a[1], a[2], a[3], a[4], a[5], bits, xt], false, true);
            }
            Target::Field(slot, shape, f) => {
                let hint = shape.fields.iter().position(|(n, _)| &**n == *f)?;
                let id = shape.fields[hint].0.as_ptr() as i64;
                let a = [*slot as i64, f.as_ptr() as i64, f.len() as i64, hint as i64, id].map(|c| self.k(c));
                self.call("field_set", rt::field_set as *const () as usize, &[a[0], a[1], a[2], a[3], a[4], bits, xt], false, true);
            }
        }
        Some(())
    }

    /// `target = value` / `target op= value` (the walker evaluates the value first).
    pub(super) fn assign(&mut self, target: E, value: E, op: Option<CompoundOp>) -> Option<()> {
        let bop = op.map(compound_to_binary);
        let rhs = self.expr(value)?;
        if let Expression::Identifier { name, .. } = target {
            let Bind::Var(id) = self.lookup(name)?.clone() else { return None };
            let cur = Val::of(self.b.use_var(self.vars[id].var), self.vars[id].ty);
            let new = match bop {
                None => self.retag(cur.ty, rhs)?,
                Some(bop) => self.binary(bop, cur, rhs)?,
            };
            return self.set_var(id, new);
        }
        let t = self.target(target)?;
        let new = match bop {
            None => rhs,
            Some(bop) => {
                let cur = self.load(&t)?;
                self.binary(bop, cur, rhs)?
            }
        };
        self.store(&t, new)
    }
}
