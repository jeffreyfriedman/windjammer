//! Host bridge for `extern fn` (ADR-0023): the interpreter calls the project's Rust code through a
//! dynamic library compiled once and cached (built by `cli::host_build`; this module never spawns cargo).
//!
//! The compiled path declares `extern fn f(a: int, s: string) -> string` as `extern "C" { fn f(a: i64,
//! s: FfiString) -> FfiString; }` and links the project's hand-written Rust. The bridge library links the
//! same Rust and adds one uniform shim per extern fn, `__wj_shim_<name>(args, n, out) -> i32`, that converts
//! [`WjVal`]s to the C signature and calls the real symbol. Both paths call the same Rust code through the
//! same C ABI. Types that cannot cross are rejected by name before anything runs.

use super::engine::Interpreter;
use super::error::{semantic, unsupported, R};
use super::int::IntTy;
use super::value::{FloatTy, Value};
use crate::parser::{Item, Program, Type};
use std::collections::HashMap;
use std::rc::Rc;

/// One value across the bridge. `tag`: 0 unit, 1 int, 2 float, 3 bool, 4 string (ptr/len, UTF-8).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WjVal {
    pub tag: u32,
    pub i: i64,
    pub f: f64,
    pub ptr: *const u8,
    pub len: usize,
}

impl WjVal {
    fn zero() -> Self {
        WjVal { tag: 0, i: 0, f: 0.0, ptr: std::ptr::null(), len: 0 }
    }
}

/// Bridgeable kinds and their C type in the compiled path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abi {
    Unit,
    Int(&'static str),
    Float(&'static str),
    Bool,
    Str,
}

impl Abi {
    fn rust(self) -> &'static str {
        match self {
            Abi::Unit => "()",
            Abi::Int(t) | Abi::Float(t) => t,
            Abi::Bool => "bool",
            Abi::Str => "FfiString",
        }
    }
}

/// Map a declared Windjammer type to its bridge ABI (mirrors the compiled `extern "C"` signature).
pub fn abi_of(t: Option<&Type>) -> Result<Abi, String> {
    let Some(t) = t else { return Ok(Abi::Unit) };
    let shown = || crate::type_display::format_wj_type(t);
    let name = match t {
        Type::Int => "i64",
        Type::Int32 => "i32",
        Type::Uint => "u64",
        Type::Float => "f64",
        Type::Bool => "bool",
        Type::String => "string",
        Type::Custom(n) => n.as_str(),
        Type::Reference(inner) => {
            return match abi_of(Some(inner)) {
                Ok(Abi::Str) => Ok(Abi::Str),
                _ => Err(shown()),
            }
        }
        _ => return Err(shown()),
    };
    Ok(match name {
        "i64" | "int" => Abi::Int("i64"),
        "i32" => Abi::Int("i32"),
        "i16" => Abi::Int("i16"),
        "i8" => Abi::Int("i8"),
        "u64" | "uint" => Abi::Int("u64"),
        "u32" => Abi::Int("u32"),
        "u16" => Abi::Int("u16"),
        "u8" => Abi::Int("u8"),
        "usize" => Abi::Int("usize"),
        "isize" => Abi::Int("isize"),
        "f64" | "float" => Abi::Float("f64"),
        "f32" => Abi::Float("f32"),
        "bool" => Abi::Bool,
        "string" | "String" | "str" => Abi::Str,
        _ => return Err(shown()),
    })
}

/// An `extern fn` as the bridge sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternSig {
    pub name: String,
    pub params: Vec<(String, Abi)>,
    pub ret: Abi,
}

const SUPPORTED: &str = "supported: integers, floats, bool, string";

/// Every `extern fn` in the programs, or an error naming the first item whose type cannot cross the bridge.
pub fn extern_signatures(programs: &[&Program<'_>]) -> Result<Vec<ExternSig>, String> {
    fn walk(items: &[Item<'_>], out: &mut Vec<ExternSig>) -> Result<(), String> {
        for it in items {
            match it {
                Item::Function { decl, .. } if decl.is_extern => {
                    let mut params = Vec::new();
                    for p in &decl.parameters {
                        let a = abi_of(Some(&p.type_)).map_err(|t| {
                            format!("extern fn `{}`: parameter `{}: {}` cannot cross the host bridge ({})", decl.name, p.name, t, SUPPORTED)
                        })?;
                        params.push((p.name.clone(), a));
                    }
                    let ret = abi_of(decl.return_type.as_ref()).map_err(|t| {
                        format!("extern fn `{}`: return type `{}` cannot cross the host bridge ({})", decl.name, t, SUPPORTED)
                    })?;
                    if !out.iter().any(|s| s.name == decl.name) {
                        out.push(ExternSig { name: decl.name.clone(), params, ret });
                    }
                }
                Item::Mod { items, .. } => walk(items, out)?,
                _ => {}
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    for p in programs {
        walk(&p.items, &mut out)?;
    }
    Ok(out)
}

/// Rust source of the shim module compiled into the bridge library next to the project's Rust.
pub fn shim_source(sigs: &[ExternSig]) -> String {
    let mut s = String::from(concat!(
        "// generated by wj (host bridge shims, ADR-0023). Do not edit.\n",
        "#[repr(C)]\n#[derive(Clone, Copy)]\n",
        "pub struct WjVal { pub tag: u32, pub i: i64, pub f: f64, pub ptr: *const u8, pub len: usize }\n",
        "#[repr(C)]\npub struct FfiString { pub ptr: *const u8, pub len: usize }\n",
        "fn wj_str(v: &WjVal) -> FfiString {\n",
        "    let bytes: Box<[u8]> = unsafe { std::slice::from_raw_parts(v.ptr, v.len) }.to_vec().into_boxed_slice();\n",
        "    let len = bytes.len();\n",
        "    FfiString { ptr: Box::into_raw(bytes) as *const u8, len }\n",
        "}\n",
        "#[no_mangle]\npub unsafe extern \"C\" fn __wj_free(ptr: *mut u8, len: usize) {\n",
        "    if !ptr.is_null() { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len))); }\n",
        "}\n",
        "extern \"C\" {\n",
    ));
    for sig in sigs {
        let ps: Vec<String> = sig.params.iter().map(|(n, a)| format!("{}: {}", n, a.rust())).collect();
        let ret = if sig.ret == Abi::Unit { String::new() } else { format!(" -> {}", sig.ret.rust()) };
        s.push_str(&format!("    fn {}({}){};\n", sig.name, ps.join(", "), ret));
    }
    s.push_str("}\n");
    for sig in sigs {
        let args: Vec<String> = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, (_, a))| match a {
                Abi::Int(t) => format!("a[{}].i as {}", i, t),
                Abi::Float(t) => format!("a[{}].f as {}", i, t),
                Abi::Bool => format!("a[{}].i != 0", i),
                Abi::Str => format!("wj_str(&a[{}])", i),
                Abi::Unit => "()".into(),
            })
            .collect();
        let (tag, fields) = match sig.ret {
            Abi::Unit => (0, "i: 0, f: 0.0, ptr: std::ptr::null(), len: 0"),
            Abi::Int(_) => (1, "i: r as i64, f: 0.0, ptr: std::ptr::null(), len: 0"),
            Abi::Float(_) => (2, "i: 0, f: r as f64, ptr: std::ptr::null(), len: 0"),
            Abi::Bool => (3, "i: r as i64, f: 0.0, ptr: std::ptr::null(), len: 0"),
            Abi::Str => (4, "i: 0, f: 0.0, ptr: r.ptr, len: r.len"),
        };
        s.push_str(&format!(
            "#[no_mangle]\n#[allow(unused_variables, clippy::unit_arg)]\npub unsafe extern \"C\" fn __wj_shim_{name}(args: *const WjVal, n: usize, out: *mut WjVal) -> i32 {{\n    if n != {n} {{ return 1; }}\n    let a = std::slice::from_raw_parts(args, n);\n    let r = {name}({args});\n    *out = WjVal {{ tag: {tag}, {fields} }};\n    0\n}}\n",
            name = sig.name,
            n = sig.params.len(),
            args = args.join(", "),
            tag = tag,
            fields = fields,
        ));
    }
    s
}

type Shim = unsafe extern "C" fn(*const WjVal, usize, *mut WjVal) -> i32;
type Free = unsafe extern "C" fn(*mut u8, usize);

/// A loaded bridge library and the signatures it provides.
pub struct HostLib {
    _lib: libloading::Library,
    shims: HashMap<String, (Shim, ExternSig)>,
    free: Free,
}

impl HostLib {
    /// Load `path` and resolve one shim per signature; a missing shim is an error naming the function.
    pub fn open(path: &std::path::Path, sigs: &[ExternSig]) -> Result<HostLib, String> {
        // SAFETY: the library is the bridge crate wj generated for exactly these signatures.
        let lib = unsafe { libloading::Library::new(path) }
            .map_err(|e| format!("cannot load host bridge {}: {}", path.display(), e))?;
        let mut shims = HashMap::new();
        for s in sigs {
            let sym = format!("__wj_shim_{}", s.name);
            // SAFETY: shim symbols have the `Shim` signature by construction (`shim_source`).
            let f: Shim = unsafe { lib.get::<Shim>(sym.as_bytes()) }
                .map(|f| *f)
                .map_err(|_| format!("host bridge has no shim for extern fn `{}`", s.name))?;
            shims.insert(s.name.clone(), (f, s.clone()));
        }
        // SAFETY: `__wj_free` is emitted by `shim_source` with the `Free` signature.
        let free: Free = unsafe { lib.get::<Free>(b"__wj_free") }.map(|f| *f).map_err(|e| format!("host bridge: {}", e))?;
        Ok(HostLib { _lib: lib, shims, free })
    }

    pub fn has(&self, name: &str) -> bool {
        self.shims.contains_key(name)
    }
}

impl Interpreter {
    /// True when a loaded host library provides `extern fn name`.
    pub(crate) fn has_host_fn(&self, name: &str) -> bool {
        self.host.as_ref().is_some_and(|h| h.has(name))
    }

    /// Attach a bridge library (before loading the program, so the preflight sees it).
    pub fn attach_host(&mut self, lib: HostLib) {
        self.host = Some(Rc::new(lib));
    }

    /// Call `extern fn name` through the bridge.
    pub(crate) fn call_host(&mut self, name: &str, args: Vec<Value>) -> R {
        self.cap_gate_ffi(name)?;
        let Some(host) = self.host.clone() else {
            return Err(unsupported(format!("extern function `{}`", name)));
        };
        let Some((shim, sig)) = host.shims.get(name) else {
            return Err(unsupported(format!("extern function `{}` (not provided by the host bridge)", name)));
        };
        if args.len() != sig.params.len() {
            return Err(semantic(format!("extern fn `{}` takes {} arguments, got {}", name, sig.params.len(), args.len())));
        }
        let mut keep: Vec<Rc<String>> = Vec::new();
        let mut vals = Vec::with_capacity(args.len());
        for (a, (pname, abi)) in args.into_iter().zip(&sig.params) {
            let a = super::place::deref_value(a)?;
            let mut v = WjVal::zero();
            match (abi, &a) {
                (Abi::Int(_), Value::Int(n, _)) => {
                    v.tag = 1;
                    v.i = *n as i64;
                }
                (Abi::Float(_), Value::Float(x, _)) => {
                    v.tag = 2;
                    v.f = *x;
                }
                (Abi::Bool, Value::Bool(b)) => {
                    v.tag = 3;
                    v.i = *b as i64;
                }
                (Abi::Str, Value::Str(s)) => {
                    v.tag = 4;
                    v.ptr = s.as_ptr();
                    v.len = s.len();
                    keep.push(s.clone());
                }
                _ => return Err(semantic(format!("extern fn `{}`: argument `{}` is a {}", name, pname, a.type_name()))),
            }
            vals.push(v);
        }
        let mut out = WjVal::zero();
        // SAFETY: the shim reads `vals.len()` values (strings borrowed from `keep`, alive here) and writes `out`.
        let rc = unsafe { shim(vals.as_ptr(), vals.len(), &mut out) };
        drop(keep);
        if rc != 0 {
            return Err(semantic(format!("host bridge rejected the call to `{}`", name)));
        }
        Ok(match sig.ret {
            Abi::Unit => Value::Unit,
            Abi::Int(t) => Value::Int(out.i as i128, int_ty(t)),
            Abi::Float(t) => Value::Float(out.f, if t == "f32" { FloatTy::F32 } else { FloatTy::F64 }),
            Abi::Bool => Value::Bool(out.i != 0),
            Abi::Str => {
                // SAFETY: the shim returned an owned boxed byte slice; copy it, then free it in the library.
                let s = unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(out.ptr, out.len)).into_owned() };
                unsafe { (host.free)(out.ptr as *mut u8, out.len) };
                Value::str(s)
            }
        })
    }
}

fn int_ty(t: &str) -> IntTy {
    match t {
        "i8" => IntTy::I8,
        "i16" => IntTy::I16,
        "i32" => IntTy::I32,
        "u8" => IntTy::U8,
        "u16" => IntTy::U16,
        "u32" => IntTy::U32,
        "u64" => IntTy::U64,
        "usize" => IntTy::Usize,
        "isize" => IntTy::Isize,
        _ => IntTy::I64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shims_mirror_the_compiled_extern_signatures() {
        let sigs = vec![
            ExternSig { name: "add".into(), params: vec![("a".into(), Abi::Int("i64")), ("b".into(), Abi::Int("i64"))], ret: Abi::Int("i64") },
            ExternSig { name: "greet".into(), params: vec![("n".into(), Abi::Str)], ret: Abi::Str },
        ];
        let s = shim_source(&sigs);
        assert!(s.contains("fn add(a: i64, b: i64) -> i64;"), "{s}");
        assert!(s.contains("fn greet(n: FfiString) -> FfiString;"), "{s}");
        assert!(s.contains("pub unsafe extern \"C\" fn __wj_shim_greet"));
    }

    #[test]
    fn unbridgeable_types_are_named() {
        let src = "extern fn takes_vec(v: Vec<int>) -> int\nfn main() {}\n";
        let p = super::super::loader::parse_source(src, "t.wj").unwrap();
        let e = extern_signatures(&[&p]).unwrap_err();
        assert!(e.contains("extern fn `takes_vec`: parameter `v"), "{e}");
    }
}
