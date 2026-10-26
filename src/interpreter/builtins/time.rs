//! `Duration`, `Instant`, `SystemTime` over the scheduler's virtual clock.
//!
//! Time seen by a program is `real elapsed + virtual sleep`, so `sleep(100ms)` returns at once but
//! `Instant::elapsed()` still reports at least 100ms. With the deterministic-time flag the real part
//! is dropped entirely and the wall clock starts at a fixed instant: runs are reproducible bit for bit.

use super::*;
use crate::interpreter::native::{downcast, Native};
use std::any::Any;

#[derive(Clone, Copy)]
pub struct Duration(pub i128);
#[derive(Clone, Copy)]
pub struct Instant(pub i128);
#[derive(Clone, Copy)]
pub struct SystemTime(pub i128);

/// Fixed wall-clock origin in deterministic mode (2023-11-14T22:13:20Z).
pub const FIXED_EPOCH_NS: i128 = 1_700_000_000_000_000_000;

macro_rules! time_native {
    ($t:ty, $name:literal) => {
        impl Native for $t {
            fn type_name(&self) -> &'static str {
                $name
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn order_key(&self) -> Option<i128> {
                Some(self.0)
            }
            fn debug(&self) -> String {
                self.show()
            }
        }
    };
}

impl Duration {
    fn show(&self) -> String {
        let ns = self.0;
        let (unit, div) = if ns >= 1_000_000_000 {
            ("s", 1_000_000_000)
        } else if ns >= 1_000_000 {
            ("ms", 1_000_000)
        } else if ns >= 1_000 {
            ("µs", 1_000)
        } else {
            ("ns", 1)
        };
        let (int, frac) = (ns / div, ns % div);
        if frac == 0 {
            return format!("{}{}", int, unit);
        }
        let width = (div as f64).log10() as usize;
        let f = format!("{:0width$}", frac, width = width);
        format!("{}.{}{}", int, f.trim_end_matches('0'), unit)
    }
}
impl Instant {
    fn show(&self) -> String {
        format!("Instant {{ ns: {} }}", self.0)
    }
}
impl SystemTime {
    fn show(&self) -> String {
        format!("SystemTime {{ ns: {} }}", self.0)
    }
}
time_native!(Duration, "Duration");
time_native!(Instant, "Instant");
time_native!(SystemTime, "SystemTime");

pub fn duration(ns: i128) -> Value {
    Value::Native(Rc::new(Duration(ns)))
}

pub fn duration_ns(v: &Value) -> R<i128> {
    downcast::<Duration>(v).map(|d| d.0).ok_or_else(|| semantic(format!("expected a Duration, found {}", v.type_name())))
}

impl Interpreter {
    /// Monotonic nanoseconds since the interpreter started (virtual sleep included).
    pub(crate) fn mono_ns(&self) -> i128 {
        let real = if self.sched.deterministic { 0 } else { self.clock_reading("clock.elapsed", || self.epoch.elapsed().as_nanos() as i128) };
        real + self.sched.clock as i128
    }
    /// Wall-clock nanoseconds since the Unix epoch.
    pub(crate) fn wall_ns(&self) -> i128 {
        let base = if self.sched.deterministic {
            FIXED_EPOCH_NS
        } else {
            self.clock_reading("clock.wall_base", || {
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i128).unwrap_or(0) - self.epoch.elapsed().as_nanos() as i128
            })
        };
        base + self.mono_ns()
    }
}

/// `+ - * /` between time values (and `Duration * int`); `None` = not a time operation.
pub fn native_binary(op: crate::parser::BinaryOp, l: &Value, r: &Value) -> Option<R> {
    use crate::parser::BinaryOp::*;
    let d = |x: &Value| downcast::<Duration>(x).map(|d| d.0);
    let int = |x: &Value| if let Value::Int(n, _) = x { Some(*n) } else { None };
    let out = |ns: i128, what: &str| -> R {
        if ns < 0 {
            Err(panic_ctl(format!("overflow when subtracting durations{}", what)))
        } else {
            Ok(duration(ns))
        }
    };
    if let (Some(a), Some(b)) = (d(l), d(r)) {
        return Some(match op {
            Add => Ok(duration(a + b)),
            Sub => out(a - b, ""),
            _ => return None,
        });
    }
    if let (Some(a), Some(n)) = (d(l), int(r)) {
        return Some(match op {
            Mul => Ok(duration(a * n)),
            Div if n != 0 => Ok(duration(a / n)),
            Div => Err(panic_ctl("divide by zero error when dividing duration by scalar")),
            _ => return None,
        });
    }
    if let (Some(n), Some(b)) = (int(l), d(r)) {
        if op == Mul {
            return Some(Ok(duration(n * b)));
        }
    }
    if let (Some(a), Some(b)) = (downcast::<Instant>(l), downcast::<Instant>(r)) {
        if op == Sub {
            return Some(Ok(duration((a.0 - b.0).max(0))));
        }
    }
    if let (Some(a), Some(b)) = (downcast::<Instant>(l), d(r)) {
        return Some(match op {
            Add => Ok(Value::Native(Rc::new(Instant(a.0 + b)))),
            Sub => Ok(Value::Native(Rc::new(Instant(a.0 - b)))),
            _ => return None,
        });
    }
    if let (Some(a), Some(b)) = (downcast::<SystemTime>(l), d(r)) {
        return Some(match op {
            Add => Ok(Value::Native(Rc::new(SystemTime(a.0 + b)))),
            Sub => Ok(Value::Native(Rc::new(SystemTime(a.0 - b)))),
            _ => return None,
        });
    }
    None
}

pub fn register(r: &mut Registry) {
    for t in ["Duration", "Instant", "SystemTime"] {
        r.type_name(t);
    }
    r.add_const("Duration::ZERO", duration(0));
    r.add_const("Duration::MAX", duration(u64::MAX as i128 * 1_000_000_000 + 999_999_999));
    r.add_const("UNIX_EPOCH", Value::Native(Rc::new(SystemTime(0))));
    r.add_const("SystemTime::UNIX_EPOCH", Value::Native(Rc::new(SystemTime(0))));
    r.func("Duration::from_secs", |_, a| Ok(duration(arg_int(&a, 0, "from_secs")? * 1_000_000_000)));
    r.func("Duration::from_millis", |_, a| Ok(duration(arg_int(&a, 0, "from_millis")? * 1_000_000)));
    r.func("Duration::from_micros", |_, a| Ok(duration(arg_int(&a, 0, "from_micros")? * 1_000)));
    r.func("Duration::from_nanos", |_, a| Ok(duration(arg_int(&a, 0, "from_nanos")?)));
    r.func("Duration::from_secs_f64", |_, a| Ok(duration((arg_f64(&a, 0, "from_secs_f64")? * 1e9).round() as i128)));
    r.func("Duration::from_secs_f32", |_, a| Ok(duration((arg_f64(&a, 0, "from_secs_f32")? * 1e9).round() as i128)));
    r.func("Duration::new", |_, a| Ok(duration(arg_int(&a, 0, "new")? * 1_000_000_000 + arg_int(&a, 1, "new")?)));
    r.func("Duration::default", |_, _| Ok(duration(0)));
    r.func("Instant::now", |it, _| Ok(Value::Native(Rc::new(Instant(it.mono_ns())))));
    r.func("SystemTime::now", |it, _| Ok(Value::Native(Rc::new(SystemTime(it.wall_ns())))));

    fn dur(v: &Value) -> i128 {
        downcast::<Duration>(v).map(|d| d.0).unwrap_or(0)
    }
    r.native("Duration", "as_secs", |_, v, _| Ok(Value::Int(dur(v) / 1_000_000_000, IntTy::U64)));
    r.native("Duration", "as_millis", |_, v, _| Ok(Value::Int(dur(v) / 1_000_000, IntTy::U128)));
    r.native("Duration", "as_micros", |_, v, _| Ok(Value::Int(dur(v) / 1_000, IntTy::U128)));
    r.native("Duration", "as_nanos", |_, v, _| Ok(Value::Int(dur(v), IntTy::U128)));
    r.native("Duration", "as_seconds", |_, v, _| Ok(Value::Int(dur(v) / 1_000_000_000, IntTy::I64)));
    r.native("Duration", "as_secs_f64", |_, v, _| Ok(Value::float(dur(v) as f64 / 1e9)));
    r.native("Duration", "as_secs_f32", |_, v, _| Ok(Value::Float((dur(v) as f64 / 1e9) as f32 as f64, FloatTy::F32)));
    r.native("Duration", "subsec_nanos", |_, v, _| Ok(Value::Int(dur(v) % 1_000_000_000, IntTy::U32)));
    r.native("Duration", "subsec_millis", |_, v, _| Ok(Value::Int(dur(v) % 1_000_000_000 / 1_000_000, IntTy::U32)));
    r.native("Duration", "subsec_micros", |_, v, _| Ok(Value::Int(dur(v) % 1_000_000_000 / 1_000, IntTy::U32)));
    r.native("Duration", "is_zero", |_, v, _| Ok(Value::Bool(dur(v) == 0)));
    r.native("Duration", "clone", |_, v, _| Ok(v.clone()));
    r.native("Duration", "checked_add", |_, v, a| Ok(Value::some(duration(dur(v) + duration_ns(&arg(&a, 0, "checked_add")?)?))));
    r.native("Duration", "saturating_sub", |_, v, a| Ok(duration((dur(v) - duration_ns(&arg(&a, 0, "saturating_sub")?)?).max(0))));
    r.native("Duration", "checked_sub", |_, v, a| {
        let d = dur(v) - duration_ns(&arg(&a, 0, "checked_sub")?)?;
        Ok(if d < 0 { Value::none() } else { Value::some(duration(d)) })
    });
    fn inst(v: &Value) -> i128 {
        downcast::<Instant>(v).map(|d| d.0).unwrap_or(0)
    }
    r.native("Instant", "elapsed", |it, v, _| Ok(duration((it.mono_ns() - inst(v)).max(0))));
    r.native("Instant", "duration_since", |_, v, a| {
        let o = downcast::<Instant>(&arg(&a, 0, "duration_since")?).map(|i| i.0).unwrap_or(0);
        Ok(duration((inst(v) - o).max(0)))
    });
    r.native("Instant", "clone", |_, v, _| Ok(v.clone()));
    fn st(v: &Value) -> i128 {
        downcast::<SystemTime>(v).map(|d| d.0).unwrap_or(0)
    }
    r.native("SystemTime", "duration_since", |_, v, a| {
        let o = downcast::<SystemTime>(&arg(&a, 0, "duration_since")?).map(|i| i.0).unwrap_or(0);
        let d = st(v) - o;
        Ok(if d >= 0 { Value::ok(duration(d)) } else { Value::err(duration(-d)) })
    });
    r.native("SystemTime", "elapsed", |it, v, _| Ok(Value::ok(duration((it.wall_ns() - st(v)).max(0)))));
    r.native("SystemTime", "clone", |_, v, _| Ok(v.clone()));
}
