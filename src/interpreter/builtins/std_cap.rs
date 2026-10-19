//! `std::cap`: capability values (ADR-0021 section 6). A value is an unforgeable handle (no constructor outside this
//! file) holding a narrowed [`Grant`]; an operation through it is checked against that grant and then performed with
//! the ambient gate bypassed, so a dependency that is *passed* a `Net` needs no ambient `net.connect` of its own.
//! Minting (`cap::net()`) is checked against the minting package's grant; narrowing (`.only(..)`, `.under(..)`,
//! `.read_only()`) can only shrink.

use super::*;
use crate::interpreter::error::{ErrorKind, RuntimeError};
use crate::interpreter::native::{downcast, Native};
use crate::ir::effect_facts::grant::Grant;
use crate::ir::effect_facts::scope_match::canonical_abs;
use crate::ir::effect_facts::EffectKind;
use std::any::Any;

macro_rules! cap_native {
    ($t:ty, $n:literal) => {
        impl Native for $t {
            fn type_name(&self) -> &'static str {
                $n
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

struct NetCap(Grant);
struct FsCap(Grant);
struct EnvCap(Grant);
struct ProcCap(Grant);
struct ClockCap(Grant);
struct RngCap(Grant);
cap_native!(NetCap, "CapNet");
cap_native!(FsCap, "CapFs");
cap_native!(EnvCap, "CapEnv");
cap_native!(ProcCap, "CapProcess");
cap_native!(ClockCap, "CapClock");
cap_native!(RngCap, "CapRng");

fn denied(d: crate::ir::effect_facts::Denial) -> crate::interpreter::error::Ctl {
    RuntimeError::new(
        ErrorKind::CapabilityDenied,
        format!(
            "capability denied: {}\n  = grant in force: the capability value you were given ({})\n  = to allow: mint a wider value where the ambient grant allows it (`cap::{}().only([..])`), or pass a different one",
            d.subject(),
            d.grant,
            d.effect.name().split('.').next().unwrap_or("net"),
        ),
    )
    .into()
}

fn list(it: &mut Interpreter, v: Value) -> R<Vec<String>> {
    Ok(it.to_items(v)?.iter().filter_map(|x| if let Value::Str(s) = x { Some((**s).clone()) } else { None }).collect())
}

fn mint(it: &mut Interpreter, kind: EffectKind) -> R<Grant> {
    it.cap_mint(&kind, None).map_err(denied)
}

fn narrowed(it: &mut Interpreter, g: &Grant, kind: EffectKind, patterns: Value) -> R<Grant> {
    let ps = list(it, patterns)?;
    g.derive_scope(&kind, Some(&ps)).map_err(denied)
}

/// Check `grant` allows `kind` on `resource`, then run the builtin `path` with the ambient gate bypassed.
fn through(it: &mut Interpreter, grant: &Grant, kind: EffectKind, resource: Option<&str>, path: &str, args: Vec<Value>) -> R {
    grant.check(&kind, resource).map_err(denied)?;
    it.cap_bypass(|it| it.call_builtin_fn(path, args))
}

fn native<T: Native>(g: Grant, ctor: fn(Grant) -> T) -> Value {
    Value::Native(Rc::new(ctor(g)))
}

fn me<'a, T: Native>(v: &'a Value, what: &str) -> R<&'a T> {
    downcast::<T>(v).ok_or_else(|| semantic(format!("expected a {what} capability")))
}

pub fn register(r: &mut Registry) {
    for t in ["Net", "Fs", "Env", "Clock"] {
        r.type_name(t);
    }
    r.func("cap::net", |it, _| Ok(native(mint(it, EffectKind::NetConnect)?, NetCap)));
    r.func("cap::env", |it, _| Ok(native(mint(it, EffectKind::EnvRead)?, EnvCap)));
    r.func("cap::process", |it, _| Ok(native(mint(it, EffectKind::Process)?, ProcCap)));
    r.func("cap::clock", |it, _| Ok(native(mint(it, EffectKind::Time)?, ClockCap)));
    r.func("cap::rng", |it, _| Ok(native(mint(it, EffectKind::Random)?, RngCap)));
    r.func("cap::fs", |it, _| {
        let mut g = it.cap_mint(&EffectKind::FsRead, None).map_err(denied)?;
        // a Fs value carries both scopes; the write scope is optional (a read-only grant mints a read-only value)
        if let Ok(w) = it.cap_mint(&EffectKind::FsWrite, None) {
            g = merge(g, w);
        }
        Ok(native(g, FsCap))
    });

    // ---- Net
    r.native("CapNet", "only", |it, v, a| {
        let g = me::<NetCap>(v, "Net")?;
        let patterns = arg(&a, 0, "only")?;
        Ok(native(narrowed(it, &g.0, EffectKind::NetConnect, patterns)?, NetCap))
    });
    r.native("CapNet", "get", |it, v, a| net_call(it, v, a, "http::get"));
    r.native("CapNet", "delete", |it, v, a| net_call(it, v, a, "http::delete"));
    r.native("CapNet", "head", |it, v, a| net_call(it, v, a, "http::head"));
    r.native("CapNet", "post", |it, v, a| net_call(it, v, a, "http::post"));
    r.native("CapNet", "put", |it, v, a| net_call(it, v, a, "http::put"));

    // ---- Fs
    r.native("CapFs", "read_only", |_, v, _| {
        let mut g = me::<FsCap>(v, "Fs")?.0.clone();
        g.drop_scope(&EffectKind::FsWrite);
        Ok(native(g, FsCap))
    });
    r.native("CapFs", "under", |_, v, a| {
        let g = &me::<FsCap>(v, "Fs")?.0;
        let dir = arg_str(&a, 0, "under")?;
        let abs = canonical_abs(dir.as_str(), &std::env::current_dir().unwrap_or_default()).display().to_string();
        let pat = vec![format!("{abs}/**")];
        // narrow every scope the value has; a direction the directory is not inside of is dropped (read-only result)
        let mut out = Grant::sandbox(g.root());
        let mut first_err = None;
        let mut any = false;
        for k in [EffectKind::FsRead, EffectKind::FsWrite] {
            if g.scope(&k).is_none() {
                continue;
            }
            match g.derive_scope(&k, Some(&pat)) {
                Ok(d) => {
                    out = merge(out, d);
                    any = true;
                }
                Err(e) => first_err = first_err.or(Some(e)),
            }
        }
        if !any {
            return Err(denied(first_err.expect("a value always has a scope")));
        }
        Ok(native(out, FsCap))
    });
    r.native("CapFs", "read_to_string", |it, v, a| fs_call(it, v, a, EffectKind::FsRead, "fs::read_to_string"));
    r.native("CapFs", "read", |it, v, a| fs_call(it, v, a, EffectKind::FsRead, "fs::read"));
    r.native("CapFs", "read_dir", |it, v, a| fs_call(it, v, a, EffectKind::FsRead, "fs::read_dir"));
    r.native("CapFs", "exists", |it, v, a| fs_call(it, v, a, EffectKind::FsRead, "fs::exists"));
    r.native("CapFs", "write", |it, v, a| fs_call(it, v, a, EffectKind::FsWrite, "fs::write"));
    r.native("CapFs", "append", |it, v, a| fs_call(it, v, a, EffectKind::FsWrite, "fs::append"));
    r.native("CapFs", "create_dir_all", |it, v, a| fs_call(it, v, a, EffectKind::FsWrite, "fs::create_dir_all"));
    r.native("CapFs", "remove_file", |it, v, a| fs_call(it, v, a, EffectKind::FsWrite, "fs::remove_file"));

    // ---- Env, Process
    r.native("CapEnv", "only", |it, v, a| {
        let g = me::<EnvCap>(v, "Env")?;
        let patterns = arg(&a, 0, "only")?;
        Ok(native(narrowed(it, &g.0, EffectKind::EnvRead, patterns)?, EnvCap))
    });
    r.native("CapEnv", "var", |it, v, a| {
        let name = arg_str(&a, 0, "var")?;
        let g = &me::<EnvCap>(v, "Env")?.0;
        through(it, g, EffectKind::EnvRead, Some(name.as_str()), "env::var", a)
    });
    r.native("CapProcess", "only", |it, v, a| {
        let g = me::<ProcCap>(v, "Process")?;
        let patterns = arg(&a, 0, "only")?;
        Ok(native(narrowed(it, &g.0, EffectKind::Process, patterns)?, ProcCap))
    });
    r.native("CapProcess", "run", |it, v, a| {
        let cmd = arg_str(&a, 0, "run")?;
        let g = &me::<ProcCap>(v, "Process")?.0;
        let prog = crate::ir::effect_facts::scope_match::parse_command(cmd.as_str()).0;
        through(it, g, EffectKind::Process, Some(&prog), "process::run", a)
    });

    // ---- Clock, Rng (no resource: the value only carries the right to read time / draw random numbers)
    r.native("CapClock", "now_millis", |it, v, a| {
        let g = &me::<ClockCap>(v, "Clock")?.0;
        through(it, g, EffectKind::Time, None, "time::now_millis", a)
    });
    r.native("CapRng", "range", |it, v, a| {
        let g = &me::<RngCap>(v, "Rng")?.0;
        through(it, g, EffectKind::Random, None, "random::range", a)
    });
}

fn merge(mut a: Grant, b: Grant) -> Grant {
    a.absorb(b);
    a
}

fn net_call(it: &mut Interpreter, v: &Value, a: Vec<Value>, path: &str) -> R {
    let url = arg_str(&a, 0, "net")?;
    let g = &me::<NetCap>(v, "Net")?.0;
    through(it, g, EffectKind::NetConnect, Some(url.as_str()), path, a)
}

fn fs_call(it: &mut Interpreter, v: &Value, a: Vec<Value>, kind: EffectKind, path: &str) -> R {
    let p = arg_str(&a, 0, "fs")?;
    let g = &me::<FsCap>(v, "Fs")?.0;
    through(it, g, kind, Some(p.as_str()), path, a)
}
