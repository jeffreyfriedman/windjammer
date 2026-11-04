//! Rust source of the crate host: one binary per set of surfaced crates (ADR-0031).
//!
//! The host is a small `cargo` binary that depends on the project's crates, decodes one request at a time from
//! a pipe, calls the crate function the surface declares and encodes the result in the layout of `wire.rs`. Every
//! call runs under `catch_unwind`, so a crate panic is an error reply naming the function, not a dead pipe.
//! Parameters are decoded owned and borrowed for the call with `&`/`&mut`, which deref-coerces exactly as the
//! compiled build's call sites do (`&String` to `&str`, `&Vec<T>` to `&[T]`); `&mut` parameters are sent back.
//! The declared return type is written on the binding (`let r: usize = ..`), so a surface that disagrees with the
//! crate fails the host build the same way it fails the compiled build.

use super::sig::{CrateFnSig, Pass, WireTy};

/// Bumped whenever the generated host or the wire layout changes (part of the cache key).
pub const GENERATOR_VERSION: u32 = 1;

/// The Rust type a parameter is decoded into.
pub fn rust_type(t: &WireTy) -> String {
    match t {
        WireTy::Unit => "()".into(),
        WireTy::Int(n) | WireTy::Float(n) => n.to_string(),
        WireTy::Bool => "bool".into(),
        WireTy::Char => "char".into(),
        WireTy::Str => "String".into(),
        WireTy::Vec(e) => format!("Vec<{}>", rust_type(e)),
        WireTy::Option(e) => format!("Option<{}>", rust_type(e)),
        WireTy::Result(a, b) => format!("Result<{}, {}>", rust_type(a), rust_type(b)),
        WireTy::Tuple(ts) => format!("({},)", ts.iter().map(rust_type).collect::<Vec<_>>().join(", ")),
    }
}

/// The `Wire` trait and the pipe loop, shared by every host.
const RUNTIME: &str = r#"
use std::io::{Read, Write};

trait Wire: Sized {
    fn enc(&self, o: &mut Vec<u8>);
    fn dec(i: &mut &[u8]) -> Result<Self, String>;
}

fn take<'a>(i: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
    if i.len() < n {
        return Err("truncated request".into());
    }
    let (a, b) = i.split_at(n);
    *i = b;
    Ok(a)
}
fn tag(i: &mut &[u8], want: u8, what: &str) -> Result<(), String> {
    let t = take(i, 1)?[0];
    if t != want {
        return Err(format!("expected {what}, got tag {t}"));
    }
    Ok(())
}
fn get32(i: &mut &[u8]) -> Result<usize, String> {
    Ok(u32::from_le_bytes(take(i, 4)?.try_into().unwrap()) as usize)
}
fn put32(o: &mut Vec<u8>, n: usize) {
    o.extend_from_slice(&(n as u32).to_le_bytes());
}

macro_rules! ints {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            fn enc(&self, o: &mut Vec<u8>) {
                o.push(1);
                o.extend_from_slice(&(*self as i128).to_le_bytes());
            }
            fn dec(i: &mut &[u8]) -> Result<Self, String> {
                tag(i, 1, stringify!($t))?;
                let n = i128::from_le_bytes(take(i, 16)?.try_into().unwrap());
                <$t>::try_from(n).map_err(|_| format!("{n} does not fit in {}", stringify!($t)))
            }
        }
    )*};
}
ints!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, usize);

impl Wire for f64 {
    fn enc(&self, o: &mut Vec<u8>) {
        o.push(2);
        o.extend_from_slice(&self.to_bits().to_le_bytes());
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        tag(i, 2, "float")?;
        Ok(f64::from_bits(u64::from_le_bytes(take(i, 8)?.try_into().unwrap())))
    }
}
impl Wire for f32 {
    fn enc(&self, o: &mut Vec<u8>) {
        (*self as f64).enc(o)
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        Ok(f64::dec(i)? as f32)
    }
}
impl Wire for bool {
    fn enc(&self, o: &mut Vec<u8>) {
        o.push(3);
        o.push(*self as u8);
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        tag(i, 3, "bool")?;
        Ok(take(i, 1)?[0] != 0)
    }
}
impl Wire for char {
    fn enc(&self, o: &mut Vec<u8>) {
        o.push(4);
        o.extend_from_slice(&(*self as u32).to_le_bytes());
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        tag(i, 4, "char")?;
        let c = u32::from_le_bytes(take(i, 4)?.try_into().unwrap());
        char::from_u32(c).ok_or_else(|| format!("invalid char {c:#x}"))
    }
}
impl Wire for String {
    fn enc(&self, o: &mut Vec<u8>) {
        o.push(5);
        put32(o, self.len());
        o.extend_from_slice(self.as_bytes());
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        tag(i, 5, "string")?;
        let n = get32(i)?;
        String::from_utf8(take(i, n)?.to_vec()).map_err(|_| "string is not UTF-8".to_string())
    }
}
impl Wire for () {
    fn enc(&self, o: &mut Vec<u8>) {
        o.push(0);
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        tag(i, 0, "()")
    }
}
impl<T: Wire> Wire for Vec<T> {
    fn enc(&self, o: &mut Vec<u8>) {
        o.push(6);
        put32(o, self.len());
        for x in self {
            x.enc(o);
        }
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        tag(i, 6, "Vec")?;
        let n = get32(i)?;
        (0..n).map(|_| T::dec(i)).collect()
    }
}
impl<T: Wire> Wire for Option<T> {
    fn enc(&self, o: &mut Vec<u8>) {
        match self {
            None => o.push(7),
            Some(x) => {
                o.push(8);
                x.enc(o);
            }
        }
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        match take(i, 1)?[0] {
            7 => Ok(None),
            8 => Ok(Some(T::dec(i)?)),
            t => Err(format!("expected Option, got tag {t}")),
        }
    }
}
impl<T: Wire, E: Wire> Wire for Result<T, E> {
    fn enc(&self, o: &mut Vec<u8>) {
        match self {
            Ok(x) => {
                o.push(9);
                x.enc(o);
            }
            Err(e) => {
                o.push(10);
                e.enc(o);
            }
        }
    }
    fn dec(i: &mut &[u8]) -> Result<Self, String> {
        match take(i, 1)?[0] {
            9 => Ok(Ok(T::dec(i)?)),
            10 => Ok(Err(E::dec(i)?)),
            t => Err(format!("expected Result, got tag {t}")),
        }
    }
}
macro_rules! tuples {
    ($(($n:expr; $($t:ident $i:tt),+))*) => {$(
        impl<$($t: Wire),+> Wire for ($($t,)+) {
            fn enc(&self, o: &mut Vec<u8>) {
                o.push(11);
                put32(o, $n);
                $(self.$i.enc(o);)+
            }
            fn dec(i: &mut &[u8]) -> Result<Self, String> {
                tag(i, 11, "tuple")?;
                if get32(i)? != $n {
                    return Err("tuple arity".into());
                }
                Ok(($($t::dec(i)?,)+))
            }
        }
    )*};
}
tuples! {
    (1; A 0) (2; A 0, B 1) (3; A 0, B 1, C 2) (4; A 0, B 1, C 2, D 3)
    (5; A 0, B 1, C 2, D 3, E 4) (6; A 0, B 1, C 2, D 3, E 4, F 5)
}

thread_local! {
    static PANIC: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

fn pipes() -> (Box<dyn Read>, Box<dyn Write>) {
    #[cfg(unix)]
    {
    if std::env::var_os("WJ_CRATE_HOST_FDS").is_some() {
        use std::os::fd::FromRawFd;
        // SAFETY: the interpreter opened fds 3 (requests) and 4 (replies) for this process and nothing else uses them.
        let (r, w) = unsafe { (std::fs::File::from_raw_fd(3), std::fs::File::from_raw_fd(4)) };
        return (Box::new(std::io::BufReader::new(r)), Box::new(w));
    }
    }
    (Box::new(std::io::stdin()), Box::new(std::io::stdout()))
}

fn frame(w: &mut dyn Write, body: &[u8]) -> std::io::Result<()> {
    let mut f = Vec::with_capacity(body.len() + 4);
    f.extend_from_slice(&(body.len() as u32).to_le_bytes());
    f.extend_from_slice(body);
    w.write_all(&f)?;
    w.flush()
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };
        let at = info.location().map(|l| format!(" at {}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
        PANIC.with(|p| *p.borrow_mut() = format!("panicked{at}:\n{msg}"));
    }));
    let (mut rd, mut wr) = pipes();
    if frame(&mut *wr, KEY.as_bytes()).is_err() {
        return;
    }
    let mut buf = Vec::new();
    loop {
        let mut lb = [0u8; 4];
        if rd.read_exact(&mut lb).is_err() {
            return;
        }
        buf.resize(u32::from_le_bytes(lb) as usize, 0);
        if rd.read_exact(&mut buf).is_err() {
            return;
        }
        let mut i = &buf[..];
        let mut out = Vec::new();
        let mut body = Vec::new();
        let r = match String::dec(&mut i) {
            Err(e) => Ok(Err(e)),
            Ok(name) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(&name, &mut i, &mut body))),
        };
        match r {
            Ok(Ok(())) => {
                out.push(0);
                out.extend_from_slice(&body);
            }
            Ok(Err(m)) => {
                out.push(2);
                m.enc(&mut out);
            }
            Err(_) => {
                out.push(1);
                PANIC.with(|p| p.borrow().clone()).enc(&mut out);
            }
        }
        if frame(&mut *wr, &out).is_err() {
            return;
        }
    }
}
"#;

/// The host's `main.rs` for `sigs`; `key` is the cache key the host announces when it starts.
pub fn host_source(key: &str, sigs: &[CrateFnSig]) -> String {
    let mut s = format!(
        "// generated by wj (crate host, ADR-0031, generator {GENERATOR_VERSION}). Do not edit.\n#![allow(unused_mut, unused_variables, clippy::all)]\nconst KEY: &str = {key:?};\n"
    );
    s.push_str(RUNTIME);
    s.push_str("\nfn call(name: &str, i: &mut &[u8], o: &mut Vec<u8>) -> Result<(), String> {\n    match name {\n");
    for f in sigs {
        s.push_str(&format!("        {:?} => {{\n", f.path()));
        let mut args = Vec::new();
        let mut outs = Vec::new();
        for (k, (_, pass, t)) in f.params.iter().enumerate() {
            s.push_str(&format!("            let mut a{k}: {} = Wire::dec(i)?;\n", rust_type(t)));
            args.push(match pass {
                Pass::Owned => format!("a{k}"),
                Pass::Borrow => format!("&a{k}"),
                Pass::BorrowMut => {
                    outs.push(format!("a{k}"));
                    format!("&mut a{k}")
                }
            });
        }
        s.push_str(&format!(
            "            let r: {} = ::{}::{}({});\n            put32(o, {});\n            r.enc(o);\n",
            rust_type(&f.ret),
            f.krate,
            f.name,
            args.join(", "),
            1 + outs.len()
        ));
        for a in outs {
            s.push_str(&format!("            {a}.enc(o);\n"));
        }
        s.push_str("            Ok(())\n        }\n");
    }
    s.push_str("        _ => Err(format!(\"the crate host has no function `{name}`\")),\n    }\n}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(name: &str, params: Vec<(Pass, WireTy)>, ret: WireTy) -> CrateFnSig {
        CrateFnSig {
            krate: "demo".into(),
            name: name.into(),
            params: params.into_iter().enumerate().map(|(i, (p, t))| (format!("p{i}"), p, t)).collect(),
            ret,
            effects: None,
        }
    }

    #[test]
    fn shims_borrow_as_the_compiled_call_sites_do() {
        let s = host_source(
            "k1",
            &[
                sig("lev", vec![(Pass::Borrow, WireTy::Str), (Pass::Borrow, WireTy::Str)], WireTy::Int("usize")),
                sig("fill", vec![(Pass::BorrowMut, WireTy::Vec(Box::new(WireTy::Int("i64")))), (Pass::Owned, WireTy::Int("u8"))], WireTy::Unit),
                sig("split", vec![(Pass::Borrow, WireTy::Str)], WireTy::Option(Box::new(WireTy::Vec(Box::new(WireTy::Str))))),
            ],
        );
        assert!(s.contains("const KEY: &str = \"k1\";"));
        assert!(s.contains("let r: usize = ::demo::lev(&a0, &a1);\n            put32(o, 1);"), "{s}");
        assert!(s.contains("let mut a0: Vec<i64> = Wire::dec(i)?;"), "{s}");
        assert!(s.contains("let r: () = ::demo::fill(&mut a0, a1);\n            put32(o, 2);\n            r.enc(o);\n            a0.enc(o);"), "{s}");
        assert!(s.contains("let r: Option<Vec<String>> = ::demo::split(&a0);"), "{s}");
        assert!(s.contains("\"demo::split\" => {"));
    }

    #[test]
    fn tuple_types_keep_the_trailing_comma() {
        assert_eq!(rust_type(&WireTy::Tuple(vec![WireTy::Char])), "(char,)");
        assert_eq!(rust_type(&WireTy::Result(Box::new(WireTy::Unit), Box::new(WireTy::Str))), "Result<(), String>");
    }
}
