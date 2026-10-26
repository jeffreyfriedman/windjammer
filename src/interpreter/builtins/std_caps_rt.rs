//! `caps::*`: the checks the compiled runtime (`windjammer_runtime::caps`) performs before a sink call. Generated code and
//! hand-written bridges call them; in the interpreter they ask the run's capability gate, so a program means the same
//! interpreted and compiled. `install` is a no-op (the runner installs the gate), `installed` says whether one is in force.

use super::*;
use crate::ir::effect_facts::EffectKind;

fn ask(it: &mut Interpreter, kind: EffectKind, resource: Option<&str>) -> R {
    it.cap_decide(&kind, resource)?;
    Ok(Value::Unit)
}

pub fn register(r: &mut Registry) {
    macro_rules! gate {
        ($name:literal, $kind:expr) => {
            r.func($name, |it, a| {
                let res = arg_str(&a, 0, $name)?;
                ask(it, $kind, Some(res.as_str()))
            });
        };
    }
    gate!("caps::fs_read", EffectKind::FsRead);
    gate!("caps::fs_write", EffectKind::FsWrite);
    gate!("caps::net_connect", EffectKind::NetConnect);
    gate!("caps::net_listen", EffectKind::NetListen);
    gate!("caps::env_read", EffectKind::EnvRead);
    gate!("caps::env_write", EffectKind::EnvWrite);
    gate!("caps::process", EffectKind::Process);
    r.func("caps::env_read_all", |it, _| ask(it, EffectKind::EnvRead, Some("*")));
    r.func("caps::db_open", |it, a| {
        let url = arg_str(&a, 0, "db_open")?;
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            ask(it, EffectKind::NetConnect, Some(url.as_str()))
        } else if url.as_str() != ":memory:" && url.as_str() != "sqlite::memory:" {
            ask(it, EffectKind::FsWrite, Some(url.strip_prefix("sqlite:").unwrap_or(url.as_str())))
        } else {
            Ok(Value::Unit)
        }
    });
    r.func("caps::check", |it, a| {
        let name = arg_str(&a, 0, "check")?;
        let kind = EffectKind::parse_builtin(name.as_str()).ok_or_else(|| semantic(format!("unknown effect `{name}`")))?;
        let resource = a.get(1).and_then(|v| if let Value::Str(s) = v { Some((**s).clone()) } else { None });
        ask(it, kind, resource.as_deref())
    });
    r.func("caps::install", |_, _| Ok(Value::Unit));
    r.func("caps::installed", |it, _| Ok(Value::Bool(it.caps.is_some())));
    r.func("caps::bypass", |it, a| {
        let f = arg(&a, 0, "bypass")?;
        it.cap_bypass(|it| it.call_value(&f, Vec::new()))
    });
}
