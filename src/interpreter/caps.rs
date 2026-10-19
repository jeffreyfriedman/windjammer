//! Capability gate (ADR-0021): every standard-library sink the program calls is checked against the
//! run's grant before it executes. The sink table and the grant live in the semantic layer
//! (`crate::ir::effect_facts`); this file only maps interpreter values to resources and decides what
//! a denial does (stop with exit status 126, or warn while the local profile is being phased in).

use super::engine::Interpreter;
use super::error::{ErrorKind, RuntimeError, R};
use super::value::Value;
use crate::ir::effect_facts::grant::Grant;
use crate::ir::effect_facts::sinks::{self, Res};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapMode {
    /// A denied sink stops the run (exit status 126).
    Enforce,
    /// A denied sink runs and prints a warning once (decision D1: warn, then enforce).
    Warn,
}

pub struct CapGate {
    pub grant: Grant,
    pub mode: CapMode,
    warned: HashSet<String>,
    /// Every effect kind the run exercised (checked), for the soundness gate (dynamic set within the static set).
    pub seen: std::collections::BTreeSet<crate::ir::effect_facts::EffectKind>,
}

impl CapGate {
    pub fn new(grant: Grant, mode: CapMode) -> CapGate {
        CapGate { grant, mode, warned: HashSet::new(), seen: Default::default() }
    }
}

/// The directory of the nearest `wj.toml` at or above `start`.
fn manifest_dir(start: &Path) -> Option<PathBuf> {
    let mut cur = if start.is_dir() { Some(start) } else { start.parent() };
    while let Some(d) = cur {
        if d.join("wj.toml").is_file() {
            return Some(d.to_path_buf());
        }
        cur = d.parent();
    }
    None
}

/// What a run of `entry` may do:
/// * `--sandbox` / `--allow` / a `[capabilities]` table: exactly that grant, enforced;
/// * otherwise the local profile (project directory + `time`/`random`/`console`): warn now, enforce later
///   (set `WJ_CAPS=enforce` to try the future default, `WJ_CAPS=off` to disable the gate).
pub fn gate_for_run(entry: &Path, sandbox: bool, allow: &[String], allow_network: bool) -> Result<Option<CapGate>, String> {
    let env_mode = std::env::var("WJ_CAPS").unwrap_or_default();
    if env_mode == "off" && !sandbox && allow.is_empty() {
        return Ok(None);
    }
    let dir = manifest_dir(entry);
    let root = dir.clone().unwrap_or_else(|| entry.parent().map(Path::to_path_buf).unwrap_or_default());
    let root = if root.as_os_str().is_empty() { std::env::current_dir().unwrap_or_default() } else { root };
    let manifest = match &dir {
        Some(_) if sandbox => None,
        Some(d) => {
            let text = std::fs::read_to_string(d.join("wj.toml")).map_err(|e| format!("cannot read wj.toml: {e}"))?;
            crate::ir::effect_facts::grant_from_manifest_text(&text, d)?
        }
        None => None,
    };
    let explicit = sandbox || manifest.is_some() || !allow.is_empty() || allow_network;
    let mut grant = match manifest {
        Some(g) => g,
        None if sandbox => Grant::sandbox(&root),
        None => Grant::local_profile(&root),
    };
    for a in allow {
        grant.allow_flag(a)?;
    }
    if allow_network {
        grant.allow_flag("net.connect=*")?;
    }
    let mode = if explicit || env_mode == "enforce" { CapMode::Enforce } else { CapMode::Warn };
    Ok(Some(CapGate::new(grant, mode)))
}

impl Interpreter {
    pub fn set_caps(&mut self, gate: Option<CapGate>) {
        self.caps = gate;
    }

    /// Effect kinds exercised so far (empty without a gate).
    pub fn caps_seen(&self) -> Vec<crate::ir::effect_facts::EffectKind> {
        self.caps.as_ref().map(|g| g.seen.iter().cloned().collect()).unwrap_or_default()
    }

    /// True when the grant lets the program talk to the network (the http port must then be live).
    pub fn caps_grant_network(&self) -> bool {
        self.caps.as_ref().is_some_and(|g| g.grant.scope(&crate::ir::effect_facts::EffectKind::NetConnect).is_some())
    }

    /// Gate a call into a host-bridge `extern fn` (effect `ffi`, resource = the function name).
    pub(crate) fn cap_gate_ffi(&mut self, name: &str) -> R<()> {
        self.cap_decide(&crate::ir::effect_facts::EffectKind::Ffi, Some(name))
    }

    /// Gate one builtin call. Cheap for non-sinks: one hash lookup.
    pub(crate) fn cap_gate(&mut self, path: &str, args: &[Value]) -> R<()> {
        let Some(uses) = sinks::lookup(path) else { return Ok(()) };
        for u in uses {
            let resource: Option<String> = match u.res {
                Res::None => None,
                Res::Fixed(s) => Some(s.to_string()),
                Res::Arg(n) => Some(arg_text(args.get(n))),
                Res::CmdArg(n) => Some(crate::ir::effect_facts::scope_match::parse_command(&arg_text(args.get(n))).0),
            };
            self.cap_decide(&u.effect, resource.as_deref())?;
        }
        Ok(())
    }
}

impl Interpreter {
    /// Check one effect use against the grant; stop (enforce) or warn once (local profile).
    fn cap_decide(&mut self, effect: &crate::ir::effect_facts::EffectKind, resource: Option<&str>) -> R<()> {
        let Some(gate) = self.caps.as_mut() else { return Ok(()) };
        if !gate.seen.contains(effect) {
            gate.seen.insert(effect.clone());
        }
        if let Err(d) = gate.grant.check(effect, resource) {
            let text = format!("capability denied: {}\n  = grant in force: {}\n  = to allow: {}", d.subject(), d.grant, d.fix);
            match gate.mode {
                CapMode::Enforce => return Err(RuntimeError::new(ErrorKind::CapabilityDenied, text).into()),
                CapMode::Warn => {
                    if gate.warned.insert(d.subject()) {
                        eprintln!(
                            "warning[WJ0040]: {} is not in this project's capabilities; it will be denied in a future release\n  = to allow: {}",
                            d.subject(),
                            d.fix
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

fn arg_text(v: Option<&Value>) -> String {
    match v {
        Some(Value::Str(s)) => (**s).clone(),
        // anything that is not a plain string (a Path object, a computed value) is checked by its display form
        Some(other) => format!("{}", other.type_name()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::builtins::Registry;

    /// Independent oracle: the module prefix of a builtin decides whether it needs a classification,
    /// so a new `fs::`/`env::`/`http::`... builtin cannot bypass the gate by being left out of the sink table.
    #[test]
    fn interpreter_sinks_are_classified() {
        let mut unclassified = Vec::new();
        for p in Registry::standard().function_paths() {
            let module = p.split("::").next().unwrap_or(p);
            if sinks::AUTHORITY_MODULES.contains(&module) && sinks::lookup(p).is_none() && !sinks::is_not_sink(p) {
                unclassified.push(p);
            }
        }
        assert!(unclassified.is_empty(), "builtins of authority modules with no sink row and not in NOT_SINKS: {unclassified:?}");
    }

    #[test]
    fn every_sink_row_names_a_registered_builtin_or_an_intrinsic() {
        let reg = Registry::standard();
        let paths: std::collections::HashSet<_> = reg.function_paths().into_iter().collect();
        let intrinsics = ["print", "println", "eprint", "eprintln"];
        let stale: Vec<_> = sinks::all_paths().filter(|p| !paths.contains(p) && !intrinsics.contains(p)).collect();
        assert!(stale.is_empty(), "sink rows with no interpreter builtin (renamed or removed?): {stale:?}");
    }
}
