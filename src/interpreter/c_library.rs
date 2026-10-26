//! `extern fn` that names a symbol of the process's C library (`abs`, `sqrt`, `getpid`, ...) with scalar
//! parameters: called directly through the dynamic loader. The compiled path declares the same
//! `extern "C" { fn abs(x: i32) -> i32; }` and the linker resolves it against libc, so both paths run the same
//! symbol. Everything else needs the host bridge (ADR-0023); a name neither provides fails to link when
//! compiled, and is refused here before `main` runs.

use super::engine::{FuncDef, Interpreter};
use super::error::{semantic, unsupported, R};
use super::host_bridge::{abi_of, Abi, ExternSig};
use super::int::IntTy;
use super::value::{FloatTy, Value};

/// Scalar arguments by register class: integers/bools, then doubles (System V and AAPCS64 assign the two
/// classes independently, so interleaving in the declaration does not matter).
#[derive(Debug, PartialEq)]
pub(crate) struct Scalars {
    pub ints: Vec<i64>,
    pub floats: Vec<f64>,
}

#[cfg(unix)]
pub(crate) fn symbol(name: &str) -> Option<*const ()> {
    use libloading::os::unix::Library;
    let mut libs = vec![Library::this()];
    #[cfg(target_os = "linux")]
    libs.extend(unsafe { Library::new("libm.so.6") }.ok());
    for lib in libs {
        // SAFETY: only the address is taken; it is called after the signature check in `call`.
        if let Ok(s) = unsafe { lib.get::<*const ()>(name.as_bytes()) } {
            let p = *s;
            if !p.is_null() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(not(unix))]
pub(crate) fn symbol(_name: &str) -> Option<*const ()> {
    None
}

/// Can the C library serve this extern fn: the symbol exists and every type is a scalar (no `f32`, no strings).
pub(crate) fn callable(sig: &ExternSig) -> bool {
    let scalar = |a: &Abi| matches!(a, Abi::Int(_) | Abi::Bool | Abi::Unit) || matches!(a, Abi::Float("f64"));
    sig.params.iter().all(|(_, a)| scalar(a) && *a != Abi::Unit)
        && scalar(&sig.ret)
        && sig.params.iter().filter(|(_, a)| matches!(a, Abi::Float(_))).count() <= 4
        && sig.params.iter().filter(|(_, a)| !matches!(a, Abi::Float(_))).count() <= 6
        && symbol(&sig.name).is_some()
}

macro_rules! dispatch {
    ($p:expr, $i:expr, $f:expr, $r:ty) => {{
        use std::mem::transmute as t;
        let (i, f) = (&$i, &$f);
        // SAFETY: `$p` is the address of a C function whose parameters are `i.len()` integers and `f.len()`
        // doubles and whose result is `$r`: `callable` and `call` checked that against the declaration.
        unsafe {
            match (i.len(), f.len()) {
                (0, 0) => t::<_, extern "C" fn() -> $r>($p)(),
                (1, 0) => t::<_, extern "C" fn(i64) -> $r>($p)(i[0]),
                (2, 0) => t::<_, extern "C" fn(i64, i64) -> $r>($p)(i[0], i[1]),
                (3, 0) => t::<_, extern "C" fn(i64, i64, i64) -> $r>($p)(i[0], i[1], i[2]),
                (4, 0) => t::<_, extern "C" fn(i64, i64, i64, i64) -> $r>($p)(i[0], i[1], i[2], i[3]),
                (5, 0) => t::<_, extern "C" fn(i64, i64, i64, i64, i64) -> $r>($p)(i[0], i[1], i[2], i[3], i[4]),
                (6, 0) => t::<_, extern "C" fn(i64, i64, i64, i64, i64, i64) -> $r>($p)(i[0], i[1], i[2], i[3], i[4], i[5]),
                (0, 1) => t::<_, extern "C" fn(f64) -> $r>($p)(f[0]),
                (0, 2) => t::<_, extern "C" fn(f64, f64) -> $r>($p)(f[0], f[1]),
                (0, 3) => t::<_, extern "C" fn(f64, f64, f64) -> $r>($p)(f[0], f[1], f[2]),
                (0, 4) => t::<_, extern "C" fn(f64, f64, f64, f64) -> $r>($p)(f[0], f[1], f[2], f[3]),
                (1, 1) => t::<_, extern "C" fn(i64, f64) -> $r>($p)(i[0], f[0]),
                (1, 2) => t::<_, extern "C" fn(i64, f64, f64) -> $r>($p)(i[0], f[0], f[1]),
                (2, 1) => t::<_, extern "C" fn(i64, i64, f64) -> $r>($p)(i[0], i[1], f[0]),
                (2, 2) => t::<_, extern "C" fn(i64, i64, f64, f64) -> $r>($p)(i[0], i[1], f[0], f[1]),
                (3, 1) => t::<_, extern "C" fn(i64, i64, i64, f64) -> $r>($p)(i[0], i[1], i[2], f[0]),
                _ => return Err(unsupported("extern fn with this mix of integer and float parameters")),
            }
        }
    }};
}

impl Interpreter {
    /// Call `extern fn f` in the C library.
    pub(crate) fn call_c_library(&mut self, f: &FuncDef, args: Vec<Value>) -> R {
        self.cap_gate_ffi(&f.name)?;
        let Some(p) = symbol(&f.name) else {
            return Err(unsupported(format!(
                "extern function `{}`: no host bridge provides it and the C library has no such symbol (the compiled build cannot link it either)",
                f.name
            )));
        };
        if args.len() != f.params.len() {
            return Err(semantic(format!("extern fn `{}` takes {} arguments, got {}", f.name, f.params.len(), args.len())));
        }
        let mut sc = Scalars { ints: Vec::new(), floats: Vec::new() };
        for (a, param) in args.into_iter().zip(f.params) {
            let a = super::place::deref_value(a)?;
            match (abi_of(Some(&param.type_)), &a) {
                (Ok(Abi::Int(_)), Value::Int(n, _)) => sc.ints.push(*n as i64),
                (Ok(Abi::Bool), Value::Bool(b)) => sc.ints.push(*b as i64),
                (Ok(Abi::Float("f64")), Value::Float(x, _)) => sc.floats.push(*x),
                (Ok(Abi::Float("f64")), Value::Int(n, IntTy::Lit)) => sc.floats.push(*n as f64),
                _ => return Err(unsupported(format!("extern fn `{}`: argument `{}` ({}) cannot be passed to the C library", f.name, param.name, a.type_name()))),
            }
        }
        let ret = abi_of(f.ret).map_err(|t| unsupported(format!("extern fn `{}`: return type `{}` cannot cross the C library boundary", f.name, t)))?;
        Ok(match ret {
            Abi::Unit => {
                let _: () = dispatch!(p, sc.ints, sc.floats, ());
                Value::Unit
            }
            Abi::Int(name) => {
                let r: i64 = dispatch!(p, sc.ints, sc.floats, i64);
                let ty = IntTy::from_name(name).unwrap_or(IntTy::I64);
                Value::Int(ty.wrap(r as i128), ty)
            }
            Abi::Bool => {
                let r: i64 = dispatch!(p, sc.ints, sc.floats, i64);
                Value::Bool(r & 0xff != 0)
            }
            Abi::Float("f64") => {
                let r: f64 = dispatch!(p, sc.ints, sc.floats, f64);
                Value::Float(r, FloatTy::F64)
            }
            _ => return Err(unsupported(format!("extern fn `{}`: return type cannot cross the C library boundary", f.name))),
        })
    }
}
