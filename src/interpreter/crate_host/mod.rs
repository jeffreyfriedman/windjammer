//! Calls into Rust crate dependencies from the interpreter (ADR-0031).
//!
//! `cli::crate_host_build` builds, once per set of surfaced crates, a small host binary (`gen.rs`) and hands its
//! path and signature table to the run; this module spawns it on the first crate call and keeps it for the run
//! (`process.rs`). A call `strsim::levenshtein(a, b)` is, in order: the capability gate (the surface's `@pure` /
//! `@uses(..)`, otherwise `ffi(<crate>)`, before anything is sent), the effect port (record logs the call and its
//! result; replay answers from the log and never starts the host; simulate runs `@pure` functions and answers
//! the others from an explicit stub), then the call itself: arguments encoded by their surface types (`wire.rs`),
//! one request, one reply, `&mut` arguments written back. A crate panic is an interpreter panic naming the
//! function; a host that dies mid-call is an error naming the function and how it died.

pub mod gen;
pub mod handle;
pub mod process;
pub mod sig;
pub mod stub;
pub mod wire;

use super::effect_port::Kind;
use super::engine::Interpreter;
use super::error::{panic_ctl, semantic, Ctl, ErrorKind, RuntimeError};
use super::value::Value;
use crate::ir::effect_facts::EffectKind;
use crate::parser::{Expression, UnaryOp};
use serde_json::Value as Json;
use sig::{CrateFnSig, Pass, WireTy};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

type R = Result<Value, Ctl>;

/// The crate functions one host serves, and the host itself (not started until the first call).
pub struct CrateHost {
    sigs: HashMap<String, CrateFnSig>,
    crates: Vec<String>,
    process: RefCell<process::HostProcess>,
    /// `--simulate` answers for functions that are not `@pure`: `crate::fn` -> result (effect-log JSON).
    stubs: HashMap<String, Json>,
    /// Ids of opaque values whose last handle is gone, sent before the next call.
    release: handle::ReleaseQueue,
    /// `crate::Type` -> the interpreter's name of the type (`Type`), interned once per host.
    type_names: HashMap<String, &'static str>,
}

impl CrateHost {
    pub fn new(exe: PathBuf, key: String, sigs: Vec<CrateFnSig>) -> CrateHost {
        let mut crates: Vec<String> = sigs.iter().map(|s| s.krate.clone()).collect();
        crates.sort();
        crates.dedup();
        let type_names = gen::handle_types(&sigs)
            .into_iter()
            .map(|q| {
                let short: &'static str = Box::leak(q.rsplit("::").next().unwrap_or(&q).to_string().into_boxed_str());
                (q, short)
            })
            .collect();
        CrateHost {
            sigs: sigs.into_iter().map(|s| (s.path(), s)).collect(),
            crates,
            process: RefCell::new(process::HostProcess::new(exe, key)),
            stubs: HashMap::new(),
            release: Default::default(),
            type_names,
        }
    }

    /// Simulation stubs (`fixtures/rust_stubs.json`: `{ "crate::fn": <result> }`).
    pub fn with_stubs(mut self, stubs: HashMap<String, Json>) -> CrateHost {
        self.stubs = stubs;
        self
    }

    pub fn serves(&self, path: &str) -> bool {
        self.sigs.contains_key(path)
    }

    pub fn serves_crate(&self, name: &str) -> bool {
        self.crates.iter().any(|c| c == name)
    }

    /// A handle value for an object the host just returned.
    fn handle_value(&self, qual: &str, id: u64) -> Value {
        let name = self.type_names.get(qual).copied().unwrap_or("handle");
        handle::value(qual, name, id, Some(self.release.clone()))
    }

    /// Send the queued releases (one frame for all of them). A failure here surfaces on the next real call.
    fn flush_releases(&self) {
        let ids: Vec<u64> = match self.release.lock() {
            Ok(mut q) if !q.is_empty() => std::mem::take(&mut *q),
            _ => return,
        };
        let mut req = Vec::new();
        let ids = Value::vec(ids.into_iter().map(|i| Value::Int(i as i128, super::int::IntTy::U64)).collect());
        if wire::encode(&Value::str("__wj_release"), &WireTy::Str, &mut req).is_ok()
            && wire::encode(&ids, &WireTy::Vec(Box::new(WireTy::Int("u64"))), &mut req).is_ok()
        {
            let _ = self.process.borrow_mut().call(&req);
        }
    }
}

fn fail(kind: ErrorKind, msg: String) -> Ctl {
    Ctl::Err(Box::new(RuntimeError::new(kind, msg)))
}

/// What the gate asks before a call into a surfaced crate function: the effects its surface declares (none for `@pure`), else
/// `ffi(<crate>)`. The static effect pass (`CrateEffects::uses`) must say the same; `tests` below hold the two together.
pub(crate) fn gate_requests(sig: &CrateFnSig) -> Vec<(EffectKind, Option<String>)> {
    match &sig.effects {
        None => vec![(EffectKind::Ffi, Some(sig.krate.clone()))],
        Some(ks) => ks.iter().map(|k| (k.clone(), None)).collect(),
    }
}

impl Interpreter {
    /// Attach the crate host before loading the program, so the preflight knows which crate calls run here.
    pub fn attach_crate_host(&mut self, h: CrateHost) {
        self.crate_host = Some(Rc::new(h));
    }

    /// The same host for several interpreters on one thread (one process for a whole `wj test` run).
    pub fn attach_shared_crate_host(&mut self, h: Rc<CrateHost>) {
        self.crate_host = Some(h);
    }

    /// `crate::Type::method` when `recv` is a handle to an opaque crate value whose surface declares `method`.
    pub(crate) fn crate_method_path(&self, recv: &Value, method: &str) -> Option<String> {
        let (qual, _) = handle::parts(recv)?;
        let path = format!("{qual}::{method}");
        self.is_crate_fn(&path).then_some(path)
    }

    /// Is `path` (`strsim::levenshtein`) a crate function the attached host serves?
    pub(crate) fn is_crate_fn(&self, path: &str) -> bool {
        self.crate_host.as_ref().is_some_and(|h| h.serves(path))
    }

    /// Arguments of a crate call: a `&mut` parameter gets the place of its argument (written as `v` or `&mut v`,
    /// as the compiled build borrows it), every other argument its value.
    pub(crate) fn eval_crate_args(&mut self, path: &str, arguments: &'static [(Option<String>, &'static Expression<'static>)]) -> Result<Vec<Value>, Ctl> {
        let host = self.crate_host.clone().expect("eval_crate_args without a crate host");
        let sig = &host.sigs[path];
        let mut out = Vec::with_capacity(arguments.len());
        for (i, (_, a)) in arguments.iter().enumerate() {
            let inner = match a {
                Expression::Unary { op: UnaryOp::MutRef, operand, .. } => *operand,
                _ => *a,
            };
            if sig.params.get(i).is_some_and(|(_, p, t)| *p == Pass::BorrowMut && !matches!(t, WireTy::Handle(_))) {
                if let Some(p) = self.resolve_place(inner)? {
                    out.push(Value::Ref(Box::new(p)));
                    continue;
                }
            }
            out.push(self.eval(inner)?);
        }
        Ok(out)
    }

    /// A call into a surfaced crate function: gate, port, host.
    pub(crate) fn call_crate_fn(&mut self, path: &str, args: Vec<Value>) -> R {
        let host = self.crate_host.clone().expect("call_crate_fn without a crate host");
        let sig = host.sigs[path].clone();
        for (kind, resource) in gate_requests(&sig) {
            self.cap_decide(&kind, resource.as_deref())?;
        }
        if self.limits.is_some() && sig.effects.as_ref().is_none_or(|k| !k.is_empty()) {
            self.limit_call()?;
        }
        if args.len() != sig.params.len() {
            return Err(semantic(format!("`{path}` takes {} arguments, got {}", sig.params.len(), args.len())));
        }
        let mut places = Vec::new();
        let mut plain = Vec::with_capacity(args.len());
        for (i, (a, (_, pass, t))) in args.into_iter().zip(&sig.params).enumerate() {
            if *pass == Pass::BorrowMut && !matches!(t, WireTy::Handle(_)) {
                places.push(a.clone());
            }
            let v = super::place::deref_value(a)?;
            // `T: AsRef<[u8]>` takes a string too: its UTF-8 bytes, as the compiled call passes them
            let v = match v {
                Value::Str(s) if sig.bytes_or_str.contains(&i) => Value::vec(s.bytes().map(|b| Value::Int(b as i128, super::int::IntTy::U8)).collect()),
                v => v,
            };
            plain.push(v);
        }
        let result = match self.port_kind() {
            None => self.crate_call_live(&host, &sig, &plain)?,
            Some(Kind::Simulate) => self.crate_call_simulated(&host, &sig, &plain)?,
            Some(_) => {
                let (h, s) = (host.clone(), sig.clone());
                let mut live = |it: &mut Interpreter, a: &[Value]| it.crate_call_live(&h, &s, a);
                self.port_call_with("ffi", path, &plain, &mut live)?
            }
        };
        self.crate_copy_out(&sig, &places, result)
    }

    /// Split `(result, &mut values..)` and write the values back to the argument places.
    fn crate_copy_out(&mut self, sig: &CrateFnSig, places: &[Value], result: Value) -> R {
        if places.is_empty() {
            return Ok(result);
        }
        let items = result.items().to_vec();
        let Some((ret, outs)) = items.split_first() else {
            return Err(semantic(format!("`{}`: the crate host did not return its `&mut` arguments", sig.path())));
        };
        for (place, v) in places.iter().zip(outs) {
            match place {
                Value::Ref(p) => p.write(v.clone())?,
                other => return Err(semantic(format!("`{}`: a `&mut` argument must be a place, found {}", sig.path(), other.type_name()))),
            }
        }
        Ok(ret.clone())
    }

    /// The call itself. With `&mut` parameters the value is the tuple `(result, mut_args..)`.
    fn crate_call_live(&mut self, host: &CrateHost, sig: &CrateFnSig, args: &[Value]) -> R {
        let path = sig.path();
        let mut req = Vec::new();
        wire::encode(&Value::str(path.clone()), &WireTy::Str, &mut req).map_err(|m| semantic(m))?;
        for (a, (name, _, t)) in args.iter().zip(&sig.params) {
            wire::encode(a, t, &mut req).map_err(|m| semantic(format!("`{path}`: argument `{name}`: {m}")))?;
        }
        // the crate writes to the same stdout as the program: keep the order the compiled program has
        self.flush();
        host.flush_releases();
        let reply = host.process.borrow_mut().call(&req);
        let body = match reply {
            Ok(process::Reply::Values(b)) => b,
            Ok(process::Reply::Panic(m)) => return Err(panic_ctl(format!("Rust crate function `{path}` {m}"))),
            Ok(process::Reply::Rejected(m)) => return Err(semantic(format!("the crate host rejected `{path}`: {m}"))),
            Err(process::Failure::Start(m)) => return Err(fail(ErrorKind::Semantic, m)),
            Err(process::Failure::Died(status)) => return Err(died(&path, status)),
        };
        let mut inp = &body[..];
        let n = u32::from_le_bytes(inp.get(..4).and_then(|b| b.try_into().ok()).ok_or_else(|| semantic(format!("`{path}`: empty reply")))?);
        inp = &inp[4..];
        let mk = |q: &str, id: u64| host.handle_value(q, id);
        let ret = wire::decode_with(&mut inp, &sig.ret, &mk).map_err(|m| semantic(format!("`{path}`: {m}")))?;
        let muts: Vec<&WireTy> = sig.params.iter().filter(|(_, p, t)| *p == Pass::BorrowMut && !matches!(t, WireTy::Handle(_))).map(|(_, _, t)| t).collect();
        if n as usize != 1 + muts.len() {
            return Err(semantic(format!("`{path}`: the crate host sent {n} values, expected {}", 1 + muts.len())));
        }
        if muts.is_empty() {
            return Ok(ret);
        }
        let mut all = vec![ret];
        for t in muts {
            all.push(wire::decode_with(&mut inp, t, &mk).map_err(|m| semantic(format!("`{path}`: {m}")))?);
        }
        Ok(Value::tuple(all))
    }

    /// `--simulate`: a `@pure` function is a function of its arguments and runs; anything else needs a stub.
    fn crate_call_simulated(&mut self, host: &CrateHost, sig: &CrateFnSig, args: &[Value]) -> R {
        if sig.effects.as_ref().is_some_and(|k| k.is_empty()) {
            return self.crate_call_live(host, sig, args);
        }
        let path = sig.path();
        match host.stubs.get(&path) {
            Some(j) => stub::value(j, &sig.ret).map_err(|m| semantic(format!("fixtures/rust_stubs.json, `{path}`: {m}"))),
            None => Err(fail(
                ErrorKind::Unsupported,
                format!(
                    "`{path}` calls into Rust crate `{}` (effect ffi({})), which the simulated world does not run: declare it `@pure` in its surface if it only computes from its arguments, or give its result in fixtures/rust_stubs.json (`{{ \"{path}\": ... }}`)",
                    sig.krate, sig.krate
                ),
            )),
        }
    }
}

fn died(path: &str, status: Option<std::process::ExitStatus>) -> Ctl {
    let how = match status {
        Some(s) if s.code().is_some() => {
            // the crate ended the process (`std::process::exit`): the compiled program would exit with that code
            return fail(ErrorKind::Exit(s.code().unwrap_or(1)), String::new());
        }
        Some(s) => signal_text(&s),
        None => "for an unknown reason".to_string(),
    };
    fail(ErrorKind::Panic, format!("the Rust crate host died during `{path}` ({how})"))
}

#[cfg(unix)]
fn signal_text(s: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    match s.signal() {
        Some(n) => format!("killed by signal {n}"),
        None => s.to_string(),
    }
}

#[cfg(not(unix))]
fn signal_text(s: &std::process::ExitStatus) -> String {
    s.to_string()
}

#[cfg(test)]
mod parity {
    use super::*;
    use crate::ir::effect_facts::analysis::Resource;
    use crate::ir::effect_facts::CrateEffects;
    use std::path::PathBuf;

    /// Crate calls in the sink-parity audit: for every function of a surface, the effects the interpreter's gate asks for before
    /// the call equal the effects the static pass attributes to the same call (`wj check`, the capability lock, the durable steps).
    #[test]
    fn the_gate_asks_what_the_static_pass_says() {
        let text = "@pure\npub fn levenshtein(a: string, b: string) -> usize\n@uses(net.connect)\npub fn fetch(url: string) -> string\n@uses(fs.read, env.read)\npub fn load(p: string) -> string\npub fn split(s: string) -> Option<Vec<string>>\n";
        let s = crate::package::rust_surface::parse_surface("demo", PathBuf::from("rust/demo.wji"), text.to_string());
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        let mut stat = CrateEffects::default();
        stat.add_crate("demo");
        for f in &s.fns {
            stat.declare("demo", &f.name, crate::ir::effect_facts::analysis::declared_effects(f));
        }
        assert_eq!(s.fns.len(), 4);
        for f in &s.fns {
            let sig = super::sig::crate_fn_sig("demo", f).expect("a bridgeable signature");
            let asked: Vec<(EffectKind, Option<String>)> = gate_requests(&sig);
            let said: Vec<(EffectKind, Option<String>)> = stat
                .uses(&format!("demo::{}", f.name))
                .expect("demo is a registered crate")
                .into_iter()
                .map(|(k, r)| (k, match r {
                    Resource::Lit(l) => Some(l),
                    _ => None,
                }))
                .collect();
            assert_eq!(asked, said, "demo::{}", f.name);
        }
        // a function the surface does not declare, and a crate with no surface at all, are `ffi(<crate>)` statically
        assert_eq!(stat.uses("demo::not_declared"), Some(vec![(EffectKind::Ffi, Resource::Lit("demo".into()))]));
    }
}
