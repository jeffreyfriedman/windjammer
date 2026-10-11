//! Integer, float, bool and char methods and numeric constants.

use super::*;
use crate::interpreter::ops::int_binary;
use crate::parser::BinaryOp;

fn int_of(v: &Value) -> (i128, IntTy) {
    match v {
        Value::Int(n, t) => (*n, *t),
        _ => (0, IntTy::Lit),
    }
}

/// The type of the result: receiver type unless it is a bare literal, then the argument's.
fn ty_with(t: IntTy, args: &[Value]) -> IntTy {
    if t != IntTy::Lit {
        return t;
    }
    match args.first() {
        Some(Value::Int(_, at)) => *at,
        _ => t,
    }
}

fn float_of(v: &Value) -> (f64, FloatTy) {
    match v {
        Value::Float(f, t) => (*f, *t),
        Value::Int(n, _) => (*n as f64, FloatTy::Lit),
        _ => (0.0, FloatTy::Lit),
    }
}

fn fl(f: f64, t: FloatTy) -> Value {
    Value::Float(if t == FloatTy::F32 { f as f32 as f64 } else { f }, t)
}

macro_rules! int_pure {
    ($r:expr, $name:literal, $f:expr) => {
        $r.pure(Kind::Int, $name, $f);
    };
}

macro_rules! float_fn {
    ($r:expr, $name:literal, $f:expr) => {
        $r.pure(Kind::Float, $name, |_, v, _| {
            let (x, t) = float_of(v);
            let f: fn(f64) -> f64 = $f;
            Ok(fl(f(x), t))
        });
    };
}

pub fn register(r: &mut Registry) {
    register_int(r);
    register_float(r);
    register_bool_char(r);
    register_consts(r);
}

fn wrapping(op: BinaryOp, v: &Value, args: &[Value]) -> R {
    let (a, at) = int_of(v);
    let (b, _) = int_of(&arg(args, 0, "wrapping op")?);
    let ty = ty_with(at, args);
    let res = match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a.wrapping_mul(b),
        _ => unreachable!(),
    };
    // i128/u128 multiplication may itself wrap in i128: acceptable (documented limit)
    Ok(Value::Int(ty.wrap(res), ty))
}

fn checked(op: BinaryOp, v: &Value, args: &[Value]) -> R {
    let (a, at) = int_of(v);
    let (b, bt) = int_of(&arg(args, 0, "checked op")?);
    let ty = ty_with(at, args);
    let _ = bt;
    Ok(match int_binary(op, a, ty, b, ty) {
        Ok(x) => Value::some(x),
        Err(_) => Value::none(),
    })
}

fn saturating(op: BinaryOp, v: &Value, args: &[Value]) -> R {
    let (a, at) = int_of(v);
    let (b, _) = int_of(&arg(args, 0, "saturating op")?);
    let ty = ty_with(at, args);
    let exact: Option<i128> = match op {
        BinaryOp::Add => a.checked_add(b),
        BinaryOp::Sub => a.checked_sub(b),
        BinaryOp::Mul => a.checked_mul(b),
        _ => None,
    };
    let r = match exact {
        Some(x) => x.clamp(ty.min(), ty.max()),
        None => {
            if (a < 0) != (b < 0) && op == BinaryOp::Mul || (op == BinaryOp::Sub && b > 0) {
                ty.min()
            } else {
                ty.max()
            }
        }
    };
    Ok(Value::Int(r, ty))
}

fn register_int(r: &mut Registry) {
    int_pure!(r, "abs", |_, v, _| {
        let (n, t) = int_of(v);
        if !t.fits(n.abs()) {
            return Err(panic_ctl("attempt to negate with overflow"));
        }
        Ok(Value::Int(n.abs(), t))
    });
    int_pure!(r, "pow", |_, v, a| {
        let (n, t) = int_of(v);
        let e = arg_int(&a, 0, "pow")?;
        let mut acc: i128 = 1;
        for _ in 0..e {
            acc = acc.checked_mul(n).ok_or_else(|| panic_ctl("attempt to multiply with overflow"))?;
            if !t.fits(acc) {
                return Err(panic_ctl("attempt to multiply with overflow"));
            }
        }
        Ok(Value::Int(acc, t))
    });
    int_pure!(r, "wrapping_add", |_, v, a| wrapping(BinaryOp::Add, v, &a));
    int_pure!(r, "wrapping_sub", |_, v, a| wrapping(BinaryOp::Sub, v, &a));
    int_pure!(r, "wrapping_mul", |_, v, a| wrapping(BinaryOp::Mul, v, &a));
    int_pure!(r, "wrapping_neg", |_, v, _| {
        let (n, t) = int_of(v);
        Ok(Value::Int(t.wrap(-n), t))
    });
    int_pure!(r, "wrapping_pow", |_, v, a| {
        let (n, t) = int_of(v);
        let e = arg_int(&a, 0, "wrapping_pow")?;
        let mut acc: i128 = 1;
        for _ in 0..e {
            acc = t.wrap(acc.wrapping_mul(n));
        }
        Ok(Value::Int(acc, t))
    });
    int_pure!(r, "wrapping_shl", |_, v, a| {
        let (n, t) = int_of(v);
        let s = arg_int(&a, 0, "wrapping_shl")? as u32 % t.bits();
        Ok(Value::Int(t.wrap(n << s), t))
    });
    int_pure!(r, "wrapping_shr", |_, v, a| {
        let (n, t) = int_of(v);
        let s = arg_int(&a, 0, "wrapping_shr")? as u32 % t.bits();
        let un = if n < 0 && !t.signed() { n } else { n };
        Ok(Value::Int(t.wrap(un >> s), t))
    });
    int_pure!(r, "checked_add", |_, v, a| checked(BinaryOp::Add, v, &a));
    int_pure!(r, "checked_sub", |_, v, a| checked(BinaryOp::Sub, v, &a));
    int_pure!(r, "checked_mul", |_, v, a| checked(BinaryOp::Mul, v, &a));
    int_pure!(r, "checked_div", |_, v, a| checked(BinaryOp::Div, v, &a));
    int_pure!(r, "checked_rem", |_, v, a| checked(BinaryOp::Mod, v, &a));
    int_pure!(r, "checked_pow", |_, v, a| {
        let (n, t) = int_of(v);
        let e = arg_int(&a, 0, "checked_pow")?;
        let mut acc: i128 = 1;
        for _ in 0..e {
            match acc.checked_mul(n) {
                Some(x) if t.fits(x) => acc = x,
                _ => return Ok(Value::none()),
            }
        }
        Ok(Value::some(Value::Int(acc, t)))
    });
    int_pure!(r, "checked_neg", |_, v, _| {
        let (n, t) = int_of(v);
        Ok(if t.fits(-n) { Value::some(Value::Int(-n, t)) } else { Value::none() })
    });
    int_pure!(r, "saturating_add", |_, v, a| saturating(BinaryOp::Add, v, &a));
    int_pure!(r, "saturating_sub", |_, v, a| saturating(BinaryOp::Sub, v, &a));
    int_pure!(r, "saturating_mul", |_, v, a| saturating(BinaryOp::Mul, v, &a));
    int_pure!(r, "overflowing_add", |_, v, a| {
        let (x, t) = int_of(v);
        let (y, _) = int_of(&arg(&a, 0, "overflowing_add")?);
        let t = ty_with(t, &a);
        let s = x + y;
        Ok(Value::tuple(vec![Value::Int(t.wrap(s), t), Value::Bool(!t.fits(s))]))
    });
    int_pure!(r, "overflowing_mul", |_, v, a| {
        let (x, t) = int_of(v);
        let (y, _) = int_of(&arg(&a, 0, "overflowing_mul")?);
        let t = ty_with(t, &a);
        let s = x.wrapping_mul(y);
        Ok(Value::tuple(vec![Value::Int(t.wrap(s), t), Value::Bool(!t.fits(s))]))
    });
    int_pure!(r, "rem_euclid", |_, v, a| {
        let (x, t) = int_of(v);
        let (y, _) = int_of(&arg(&a, 0, "rem_euclid")?);
        if y == 0 {
            return Err(panic_ctl("attempt to calculate the remainder with a divisor of zero"));
        }
        Ok(Value::Int(x.rem_euclid(y), ty_with(t, &a)))
    });
    int_pure!(r, "div_euclid", |_, v, a| {
        let (x, t) = int_of(v);
        let (y, _) = int_of(&arg(&a, 0, "div_euclid")?);
        if y == 0 {
            return Err(panic_ctl("attempt to divide by zero"));
        }
        Ok(Value::Int(x.div_euclid(y), ty_with(t, &a)))
    });
    int_pure!(r, "abs_diff", |_, v, a| {
        let (x, t) = int_of(v);
        let (y, _) = int_of(&arg(&a, 0, "abs_diff")?);
        let ut = match t {
            IntTy::I8 => IntTy::U8,
            IntTy::I16 => IntTy::U16,
            IntTy::I32 => IntTy::U32,
            IntTy::I64 | IntTy::Lit => IntTy::U64,
            IntTy::Isize => IntTy::Usize,
            IntTy::I128 => IntTy::U128,
            o => o,
        };
        Ok(Value::Int((x - y).abs(), ut))
    });
    int_pure!(r, "signum", |_, v, _| {
        let (n, t) = int_of(v);
        Ok(Value::Int(n.signum(), t))
    });
    int_pure!(r, "is_positive", |_, v, _| Ok(Value::Bool(int_of(v).0 > 0)));
    int_pure!(r, "is_negative", |_, v, _| Ok(Value::Bool(int_of(v).0 < 0)));
    int_pure!(r, "count_ones", |_, v, _| {
        let (n, t) = int_of(v);
        let u = (n & mask(t)) as u128;
        Ok(Value::Int(u.count_ones() as i128, IntTy::U32))
    });
    int_pure!(r, "count_zeros", |_, v, _| {
        let (n, t) = int_of(v);
        let u = (n & mask(t)) as u128;
        Ok(Value::Int((t.bits() - u.count_ones()) as i128, IntTy::U32))
    });
    int_pure!(r, "leading_zeros", |_, v, _| {
        let (n, t) = int_of(v);
        let u = (n & mask(t)) as u128;
        Ok(Value::Int((u.leading_zeros() - (128 - t.bits())) as i128, IntTy::U32))
    });
    int_pure!(r, "trailing_zeros", |_, v, _| {
        let (n, t) = int_of(v);
        let u = (n & mask(t)) as u128;
        let tz = if u == 0 { t.bits() } else { u.trailing_zeros() };
        Ok(Value::Int(tz as i128, IntTy::U32))
    });
    int_pure!(r, "rotate_left", |_, v, a| {
        let (n, t) = int_of(v);
        let k = (arg_int(&a, 0, "rotate_left")? as u32) % t.bits();
        let bits = t.bits();
        let u = (n & mask(t)) as u128;
        let m = if bits == 128 { u128::MAX } else { (1u128 << bits) - 1 };
        let rot = ((u << k) | (u >> (bits - k) % bits.max(1))) & m;
        let rot = if k == 0 { u } else { rot };
        Ok(Value::Int(t.wrap(rot as i128), t))
    });
    int_pure!(r, "rotate_right", |_, v, a| {
        let (n, t) = int_of(v);
        let bits = t.bits();
        let k = (arg_int(&a, 0, "rotate_right")? as u32) % bits;
        let u = (n & mask(t)) as u128;
        let m = if bits == 128 { u128::MAX } else { (1u128 << bits) - 1 };
        let rot = if k == 0 { u } else { ((u >> k) | (u << (bits - k))) & m };
        Ok(Value::Int(t.wrap(rot as i128), t))
    });
    int_pure!(r, "swap_bytes", |_, v, _| {
        let (n, t) = int_of(v);
        let u = (n & mask(t)) as u128;
        let bytes = (t.bits() / 8) as usize;
        let mut out: u128 = 0;
        for i in 0..bytes {
            out |= ((u >> (8 * i)) & 0xff) << (8 * (bytes - 1 - i));
        }
        Ok(Value::Int(t.wrap(out as i128), t))
    });
    int_pure!(r, "is_power_of_two", |_, v, _| {
        let n = int_of(v).0;
        Ok(Value::Bool(n > 0 && (n & (n - 1)) == 0))
    });
    int_pure!(r, "next_power_of_two", |_, v, _| {
        let (n, t) = int_of(v);
        let mut p: i128 = 1;
        while p < n {
            p <<= 1;
        }
        Ok(Value::Int(p, t))
    });
    int_pure!(r, "isqrt", |_, v, _| {
        let (n, t) = int_of(v);
        let mut x = (n as f64).sqrt() as i128;
        while x * x > n {
            x -= 1;
        }
        while (x + 1) * (x + 1) <= n {
            x += 1;
        }
        Ok(Value::Int(x, t))
    });
    for name in ["min", "max"] {
        let _ = name;
    }
    r.pure(Kind::Int, "min", |_, v, a| {
        let (x, t) = int_of(v);
        let (y, _) = int_of(&arg(&a, 0, "min")?);
        Ok(Value::Int(x.min(y), ty_with(t, &a)))
    });
    r.pure(Kind::Int, "max", |_, v, a| {
        let (x, t) = int_of(v);
        let (y, _) = int_of(&arg(&a, 0, "max")?);
        Ok(Value::Int(x.max(y), ty_with(t, &a)))
    });
    r.pure(Kind::Int, "clamp", |_, v, a| {
        let (x, t) = int_of(v);
        let lo = arg_int(&a, 0, "clamp")?;
        let hi = arg_int(&a, 1, "clamp")?;
        if lo > hi {
            return Err(panic_ctl("assertion failed: min <= max"));
        }
        Ok(Value::Int(x.clamp(lo, hi), t))
    });
    r.pure(Kind::Int, "to_string", |it, v, _| Ok(Value::str(it.display_string(v)?)));
    r.pure(Kind::Int, "is_ascii_digit", |_, v, _| Ok(Value::Bool((48..=57).contains(&int_of(v).0))));
    r.pure(Kind::Int, "is_ascii_alphabetic", |_, v, _| {
        let n = int_of(v).0;
        Ok(Value::Bool((65..=90).contains(&n) || (97..=122).contains(&n)))
    });
    r.pure(Kind::Int, "is_ascii_whitespace", |_, v, _| Ok(Value::Bool(matches!(int_of(v).0, 32 | 9 | 10 | 12 | 13))));
    r.pure(Kind::Int, "to_ascii_uppercase", |_, v, _| {
        let (n, t) = int_of(v);
        Ok(Value::Int(if (97..=122).contains(&n) { n - 32 } else { n }, t))
    });
    r.pure(Kind::Int, "to_ascii_lowercase", |_, v, _| {
        let (n, t) = int_of(v);
        Ok(Value::Int(if (65..=90).contains(&n) { n + 32 } else { n }, t))
    });
    r.pure(Kind::Int, "to_le_bytes", |_, v, _| {
        let (n, t) = int_of(v);
        let bytes = (t.bits() / 8) as usize;
        Ok(Value::vec((0..bytes).map(|i| Value::Int((n >> (8 * i)) & 0xff, IntTy::U8)).collect()))
    });
    r.pure(Kind::Int, "to_be_bytes", |_, v, _| {
        let (n, t) = int_of(v);
        let bytes = (t.bits() / 8) as usize;
        Ok(Value::vec((0..bytes).rev().map(|i| Value::Int((n >> (8 * i)) & 0xff, IntTy::U8)).collect()))
    });
}

fn mask(t: IntTy) -> i128 {
    if t.bits() >= 127 {
        -1
    } else {
        (1i128 << t.bits()) - 1
    }
}

fn register_float(r: &mut Registry) {
    float_fn!(r, "sqrt", |x| x.sqrt());
    float_fn!(r, "cbrt", |x| x.cbrt());
    float_fn!(r, "abs", |x| x.abs());
    float_fn!(r, "floor", |x| x.floor());
    float_fn!(r, "ceil", |x| x.ceil());
    float_fn!(r, "round", |x| x.round());
    float_fn!(r, "trunc", |x| x.trunc());
    float_fn!(r, "fract", |x| x.fract());
    float_fn!(r, "signum", |x| x.signum());
    float_fn!(r, "sin", |x| x.sin());
    float_fn!(r, "cos", |x| x.cos());
    float_fn!(r, "tan", |x| x.tan());
    float_fn!(r, "asin", |x| x.asin());
    float_fn!(r, "acos", |x| x.acos());
    float_fn!(r, "atan", |x| x.atan());
    float_fn!(r, "sinh", |x| x.sinh());
    float_fn!(r, "cosh", |x| x.cosh());
    float_fn!(r, "tanh", |x| x.tanh());
    float_fn!(r, "exp", |x| x.exp());
    float_fn!(r, "exp2", |x| x.exp2());
    float_fn!(r, "ln", |x| x.ln());
    float_fn!(r, "log10", |x| x.log10());
    float_fn!(r, "log2", |x| x.log2());
    float_fn!(r, "to_degrees", |x| x.to_degrees());
    float_fn!(r, "to_radians", |x| x.to_radians());
    float_fn!(r, "recip", |x| 1.0 / x);
    r.pure(Kind::Float, "powi", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.powi(arg_int(&a, 0, "powi")? as i32), t))
    });
    r.pure(Kind::Float, "powf", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.powf(arg_f64(&a, 0, "powf")?), t))
    });
    r.pure(Kind::Float, "log", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.log(arg_f64(&a, 0, "log")?), t))
    });
    r.pure(Kind::Float, "atan2", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.atan2(arg_f64(&a, 0, "atan2")?), t))
    });
    r.pure(Kind::Float, "hypot", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.hypot(arg_f64(&a, 0, "hypot")?), t))
    });
    r.pure(Kind::Float, "copysign", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.copysign(arg_f64(&a, 0, "copysign")?), t))
    });
    r.pure(Kind::Float, "mul_add", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.mul_add(arg_f64(&a, 0, "mul_add")?, arg_f64(&a, 1, "mul_add")?), t))
    });
    r.pure(Kind::Float, "min", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.min(arg_f64(&a, 0, "min")?), t))
    });
    r.pure(Kind::Float, "max", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.max(arg_f64(&a, 0, "max")?), t))
    });
    r.pure(Kind::Float, "clamp", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.clamp(arg_f64(&a, 0, "clamp")?, arg_f64(&a, 1, "clamp")?), t))
    });
    r.pure(Kind::Float, "rem_euclid", |_, v, a| {
        let (x, t) = float_of(v);
        Ok(fl(x.rem_euclid(arg_f64(&a, 0, "rem_euclid")?), t))
    });
    r.pure(Kind::Float, "is_nan", |_, v, _| Ok(Value::Bool(float_of(v).0.is_nan())));
    r.pure(Kind::Float, "is_infinite", |_, v, _| Ok(Value::Bool(float_of(v).0.is_infinite())));
    r.pure(Kind::Float, "is_finite", |_, v, _| Ok(Value::Bool(float_of(v).0.is_finite())));
    r.pure(Kind::Float, "is_sign_negative", |_, v, _| Ok(Value::Bool(float_of(v).0.is_sign_negative())));
    r.pure(Kind::Float, "to_bits", |_, v, _| {
        let (x, t) = float_of(v);
        Ok(if t == FloatTy::F32 {
            Value::Int((x as f32).to_bits() as i128, IntTy::U32)
        } else {
            Value::Int(x.to_bits() as i128, IntTy::U64)
        })
    });
    r.pure(Kind::Float, "to_string", |it, v, _| Ok(Value::str(it.display_string(v)?)));
    r.pure(Kind::Float, "total_cmp", |_, v, a| {
        let (x, _) = float_of(v);
        Ok(ordering(x.total_cmp(&arg_f64(&a, 0, "total_cmp")?)))
    });
}

fn register_bool_char(r: &mut Registry) {
    r.pure(Kind::Bool, "then_some", |_, v, a| {
        Ok(if matches!(v, Value::Bool(true)) { Value::some(arg(&a, 0, "then_some")?) } else { Value::none() })
    });
    r.pure(Kind::Bool, "then", |it, v, a| {
        if matches!(v, Value::Bool(true)) {
            let f = arg(&a, 0, "then")?;
            Ok(Value::some(it.call_value(&f, vec![])?))
        } else {
            Ok(Value::none())
        }
    });
    r.pure(Kind::Bool, "to_string", |it, v, _| Ok(Value::str(it.display_string(v)?)));

    fn ch(v: &Value) -> char {
        if let Value::Char(c) = v { *c } else { '\0' }
    }
    macro_rules! char_pred {
        ($name:literal, $f:expr) => {
            r.pure(Kind::Char, $name, |_, v, _| {
                let f: fn(char) -> bool = $f;
                Ok(Value::Bool(f(ch(v))))
            });
        };
    }
    char_pred!("is_alphabetic", |c| c.is_alphabetic());
    char_pred!("is_numeric", |c| c.is_numeric());
    char_pred!("is_alphanumeric", |c| c.is_alphanumeric());
    char_pred!("is_whitespace", |c| c.is_whitespace());
    char_pred!("is_uppercase", |c| c.is_uppercase());
    char_pred!("is_lowercase", |c| c.is_lowercase());
    char_pred!("is_control", |c| c.is_control());
    char_pred!("is_ascii", |c| c.is_ascii());
    char_pred!("is_ascii_digit", |c| c.is_ascii_digit());
    char_pred!("is_ascii_alphabetic", |c| c.is_ascii_alphabetic());
    char_pred!("is_ascii_alphanumeric", |c| c.is_ascii_alphanumeric());
    char_pred!("is_ascii_uppercase", |c| c.is_ascii_uppercase());
    char_pred!("is_ascii_lowercase", |c| c.is_ascii_lowercase());
    char_pred!("is_ascii_punctuation", |c| c.is_ascii_punctuation());
    char_pred!("is_ascii_whitespace", |c| c.is_ascii_whitespace());
    char_pred!("is_ascii_hexdigit", |c| c.is_ascii_hexdigit());
    r.pure(Kind::Char, "to_ascii_uppercase", |_, v, _| Ok(Value::Char(ch(v).to_ascii_uppercase())));
    r.pure(Kind::Char, "to_ascii_lowercase", |_, v, _| Ok(Value::Char(ch(v).to_ascii_lowercase())));
    r.pure(Kind::Char, "to_uppercase", |_, v, _| Ok(new_iter(ch(v).to_uppercase().map(Value::Char).collect())));
    r.pure(Kind::Char, "to_lowercase", |_, v, _| Ok(new_iter(ch(v).to_lowercase().map(Value::Char).collect())));
    r.pure(Kind::Char, "to_string", |_, v, _| Ok(Value::str(ch(v).to_string())));
    r.pure(Kind::Char, "len_utf8", |_, v, _| Ok(Value::usize(ch(v).len_utf8())));
    r.pure(Kind::Char, "to_digit", |_, v, a| {
        let radix = arg_int(&a, 0, "to_digit")? as u32;
        Ok(match ch(v).to_digit(radix) {
            Some(d) => Value::some(Value::Int(d as i128, IntTy::U32)),
            None => Value::none(),
        })
    });
    r.pure(Kind::Char, "is_digit", |_, v, a| {
        let radix = arg_int(&a, 0, "is_digit")? as u32;
        Ok(Value::Bool(ch(v).is_digit(radix)))
    });
}

fn register_consts(r: &mut Registry) {
    let ints = [
        ("i8", IntTy::I8),
        ("i16", IntTy::I16),
        ("i32", IntTy::I32),
        ("i64", IntTy::I64),
        ("int", IntTy::I64),
        ("i128", IntTy::I128),
        ("isize", IntTy::Isize),
        ("u8", IntTy::U8),
        ("u16", IntTy::U16),
        ("u32", IntTy::U32),
        ("u64", IntTy::U64),
        ("uint", IntTy::U64),
        ("u128", IntTy::U128),
        ("usize", IntTy::Usize),
    ];
    for (n, t) in ints {
        r.add_const(format!("{}::MAX", n), Value::Int(t.max(), t));
        r.add_const(format!("{}::MIN", n), Value::Int(t.min(), t));
        r.add_const(format!("{}::BITS", n), Value::Int(t.bits() as i128, IntTy::U32));
    }
    for (n, t) in [("f64", FloatTy::F64), ("float", FloatTy::F64), ("f32", FloatTy::F32)] {
        let (max, min, eps) = if t == FloatTy::F32 {
            (f32::MAX as f64, f32::MIN as f64, f32::EPSILON as f64)
        } else {
            (f64::MAX, f64::MIN, f64::EPSILON)
        };
        r.add_const(format!("{}::MAX", n), Value::Float(max, t));
        r.add_const(format!("{}::MIN", n), Value::Float(min, t));
        r.add_const(format!("{}::EPSILON", n), Value::Float(eps, t));
        r.add_const(format!("{}::INFINITY", n), Value::Float(f64::INFINITY, t));
        r.add_const(format!("{}::NEG_INFINITY", n), Value::Float(f64::NEG_INFINITY, t));
        r.add_const(format!("{}::NAN", n), Value::Float(f64::NAN, t));
        let c = |v: f64| Value::Float(if t == FloatTy::F32 { v as f32 as f64 } else { v }, t);
        for (cn, v) in [
            ("PI", std::f64::consts::PI),
            ("E", std::f64::consts::E),
            ("TAU", std::f64::consts::TAU),
            ("SQRT_2", std::f64::consts::SQRT_2),
            ("LN_2", std::f64::consts::LN_2),
            ("LN_10", std::f64::consts::LN_10),
            ("FRAC_PI_2", std::f64::consts::FRAC_PI_2),
            ("FRAC_PI_4", std::f64::consts::FRAC_PI_4),
            ("FRAC_1_SQRT_2", std::f64::consts::FRAC_1_SQRT_2),
        ] {
            r.add_const(format!("{}::consts::{}", n, cn), c(v));
        }
    }
}
