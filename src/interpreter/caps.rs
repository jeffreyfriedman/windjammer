//! Capability gate (ADR-0021): every standard-library sink the program calls is checked against the
//! grant of the package that contains the call before it executes. The sink table and the grants live in
//! the semantic layer (`crate::ir::effect_facts`); this file only maps interpreter values to resources
//! and decides what a denial does (stop with exit status 126, or warn while the local profile is phased in).

use super::engine::Interpreter;
use super::error::{ErrorKind, RuntimeError, R};
use super::value::Value;
use crate::ir::effect_facts::grant::Grant;
use crate::ir::effect_facts::sinks::{self, Res};
use crate::ir::effect_facts::{CapLock, EffectKind, PackageGrants};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapMode {
    /// A denied sink stops the run (exit status 126).
    Enforce,
    /// A denied sink runs and prints a warning once (decision D1: warn, then enforce).
    Warn,
}

/// What the local profile does when no grant is given (ADR-0021 D1, owner decision 2026-10-19): warn through
/// the 0.50 line, enforce from 0.51.0. `gate_for_run` reads only this constant; the test below fails the
/// release that bumps the version without flipping it.
pub const LOCAL_PROFILE_MODE: CapMode = CapMode::Warn;

pub struct CapGate {
    /// The application's grant.
    pub grant: Grant,
    /// Dependency grants (crate name -> grant); empty unless dependencies are enforced.
    deps: BTreeMap<String, Grant>,
    default_dep: Option<Grant>,
    pub mode: CapMode,
    warned: HashSet<String>,
    /// Every effect kind the run exercised (checked), for the soundness gate (dynamic set within the static set).
    pub seen: BTreeSet<EffectKind>,
    /// `console` is granted without a scope: printing skips the check (only `seen` is recorded).
    console_open: bool,
    console_seen: bool,
    /// Nonzero while a capability value performs an operation it already validated (`std::cap`).
    pub(crate) bypass: u32,
}

impl CapGate {
    pub fn new(grant: Grant, mode: CapMode) -> CapGate {
        let console_open = grant.check(&EffectKind::Console, None).is_ok()
            && grant.scope(&EffectKind::Console).is_none_or(|s| matches!(s, crate::ir::effect_facts::Scope::Any));
        CapGate { grant, deps: BTreeMap::new(), default_dep: None, mode, warned: HashSet::new(), seen: BTreeSet::new(), console_open, console_seen: false, bypass: 0 }
    }

    /// Per-package grants: each dependency's code is checked against its own grant (lexical attribution).
    pub fn with_packages(grants: PackageGrants, mode: CapMode) -> Option<CapGate> {
        let app = grants.app.clone()?;
        let mut g = CapGate::new(app, mode);
        if grants.enforce_deps {
            g.deps = grants.deps.clone();
            g.default_dep = Some(grants.default_dep.clone());
        }
        Some(g)
    }

    fn grant_for(&self, package: Option<&str>) -> &Grant {
        match (package, &self.default_dep) {
            (Some(p), Some(default)) => self.deps.get(p).unwrap_or(default),
            _ => &self.grant,
        }
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
/// * `--sandbox` / `--allow` / a `[capabilities]` table: exactly that grant, enforced (dependencies get their own
///   grants from `[capabilities.dependencies.NAME]` and `.wj-capabilities.lock`);
/// * otherwise the local profile (project directory + `time`/`random`/`console`) in [`LOCAL_PROFILE_MODE`]
///   (`--caps enforce|warn|off` / `WJ_CAPS` override it).
pub fn gate_for_run(entry: &Path, sandbox: bool, allow: &[String], allow_network: bool, caps_mode: &str) -> Result<Option<CapGate>, String> {
    if caps_mode == "off" && !sandbox && allow.is_empty() {
        return Ok(None);
    }
    let dir = manifest_dir(entry);
    let root = dir.clone().unwrap_or_else(|| entry.parent().map(Path::to_path_buf).unwrap_or_default());
    let root = if root.as_os_str().is_empty() { std::env::current_dir().unwrap_or_default() } else { root };
    let packages = match &dir {
        Some(_) if sandbox => None,
        Some(d) => {
            let text = std::fs::read_to_string(d.join("wj.toml")).map_err(|e| format!("cannot read wj.toml: {e}"))?;
            let lock = CapLock::load(d)?;
            let g = PackageGrants::from_manifest_text(&text, d, lock.as_ref())?;
            g.app.is_some().then_some(g)
        }
        None => None,
    };
    let explicit = sandbox || packages.is_some() || !allow.is_empty() || allow_network;
    let mut grants = packages.unwrap_or_else(|| PackageGrants {
        app: Some(if sandbox { Grant::sandbox(&root) } else { Grant::local_profile(&root) }),
        deps: BTreeMap::new(),
        default_dep: Grant::sandbox(&root),
        enforce_deps: false,
    });
    if let Some(app) = grants.app.as_mut() {
        for a in allow {
            app.allow_flag(a)?;
        }
        if allow_network {
            app.allow_flag("net.connect=*")?;
        }
    }
    // an explicit grant is always enforced; the local profile follows the flag, then the release default
    let mode = match caps_mode {
        _ if explicit => CapMode::Enforce,
        "enforce" => CapMode::Enforce,
        "warn" => CapMode::Warn,
        _ => LOCAL_PROFILE_MODE,
    };
    Ok(CapGate::with_packages(grants, mode))
}

impl Interpreter {
    pub fn set_caps(&mut self, gate: Option<CapGate>) {
        self.caps = gate;
    }

    /// Effect kinds exercised so far (empty without a gate).
    pub fn caps_seen(&self) -> Vec<EffectKind> {
        let Some(g) = self.caps.as_ref() else { return Vec::new() };
        let mut v: Vec<EffectKind> = g.seen.iter().cloned().collect();
        if g.console_seen && !g.seen.contains(&EffectKind::Console) {
            v.push(EffectKind::Console);
        }
        v
    }

    /// True when the grant lets the program talk to the network (the http port must then be live).
    pub fn caps_grant_network(&self) -> bool {
        let net = EffectKind::NetConnect;
        self.caps.as_ref().is_some_and(|g| g.grant.scope(&net).is_some() || g.deps.values().any(|d| d.scope(&net).is_some()))
    }

    /// The dependency crate whose code is running, if any: the child of the root module that contains the
    /// current module (closures are attributed to where they are written, since a closure keeps its module).
    fn cap_package(&self) -> Option<String> {
        if self.dep_crates.is_empty() {
            return None;
        }
        let mut m = self.scope.module;
        while let Some(parent) = self.modules[m].parent {
            if parent == 0 {
                let name = &self.modules[m].name;
                return self.dep_crates.iter().find(|c| *c == name).cloned();
            }
            m = parent;
        }
        None
    }

    /// Mint the grant behind a capability value: the running package's scope for `kind`, optionally narrowed.
    /// Without a gate (library use) everything is available.
    pub(crate) fn cap_mint(&mut self, kind: &EffectKind, patterns: Option<&[String]>) -> Result<Grant, crate::ir::effect_facts::Denial> {
        let package = self.cap_package();
        let base = match self.caps.as_ref() {
            Some(g) => g.grant_for(package.as_deref()).clone(),
            None => Grant::unrestricted(&std::env::current_dir().unwrap_or_default()),
        };
        base.derive_scope(kind, patterns)
    }

    /// Run `f` with the ambient gate bypassed: a capability value already checked the operation against its own scope.
    pub(crate) fn cap_bypass<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        if let Some(g) = self.caps.as_mut() {
            g.bypass += 1;
        }
        let r = f(self);
        if let Some(g) = self.caps.as_mut() {
            g.bypass -= 1;
        }
        r
    }

    /// Gate a call into a host-bridge `extern fn` (effect `ffi`, resource = the function name).
    pub(crate) fn cap_gate_ffi(&mut self, name: &str) -> R<()> {
        self.cap_decide(&EffectKind::Ffi, Some(name))
    }

    /// Gate console output (`print`, `println`, `eprint`, `eprintln`). Free when `console` is open.
    pub(crate) fn cap_gate_console(&mut self) -> R<()> {
        match self.caps.as_mut() {
            None => Ok(()),
            Some(g) if g.console_open => {
                g.console_seen = true;
                Ok(())
            }
            Some(_) => self.cap_decide(&EffectKind::Console, None),
        }
    }

    /// Gate one builtin call. Cheap for non-sinks: a prefix test, no hashing.
    pub(crate) fn cap_gate(&mut self, path: &str, args: &[Value]) -> R<()> {
        if !sinks::may_be_sink(path) {
            return Ok(());
        }
        let Some(uses) = sinks::lookup(path) else { return Ok(()) };
        for u in uses {
            let resource: Option<String> = match u.res {
                // minting a capability value: `cap_mint` checks it (against the minting package's scope)
                Res::Scope => {
                    if let Some(g) = self.caps.as_mut() {
                        g.seen.insert(u.effect.clone());
                    }
                    continue;
                }
                Res::None => None,
                Res::Fixed(s) => Some(s.to_string()),
                Res::Arg(n) => Some(arg_text(args.get(n))),
                Res::CmdArg(n) => Some(crate::ir::effect_facts::scope_match::parse_command(&arg_text(args.get(n))).0),
            };
            self.cap_decide(&u.effect, resource.as_deref())?;
        }
        Ok(())
    }

    /// Check one effect use against the grant of the running package; stop (enforce) or warn once (local profile).
    pub(crate) fn cap_decide(&mut self, effect: &EffectKind, resource: Option<&str>) -> R<()> {
        if self.caps.is_none() {
            return Ok(());
        }
        let package = self.cap_package();
        let Some(gate) = self.caps.as_mut() else { return Ok(()) };
        if !gate.seen.contains(effect) {
            gate.seen.insert(effect.clone());
        }
        if gate.bypass > 0 {
            return Ok(());
        }
        if let Err(d) = gate.grant_for(package.as_deref()).check(effect, resource) {
            let who = package.as_deref().map(|p| format!(" (dependency `{p}`)")).unwrap_or_default();
            let text = format!("capability denied: {}{who}\n  = grant in force: {}\n  = to allow: {}", d.subject(), d.grant, d.fix);
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

    /// The release that reaches 0.51.0 must enforce the local profile (owner decision D1).
    #[test]
    fn local_profile_is_enforced_from_0_51() {
        let v: Vec<u32> = env!("CARGO_PKG_VERSION").split(['.', '-']).take(2).filter_map(|p| p.parse().ok()).collect();
        let past = v.first().is_some_and(|major| *major > 0) || v.get(1).is_some_and(|minor| *minor >= 51);
        if past {
            assert_eq!(LOCAL_PROFILE_MODE, CapMode::Enforce, "version {} must enforce the local profile", env!("CARGO_PKG_VERSION"));
        }
    }

    fn gate(manifest: &str) -> CapGate {
        let g = crate::ir::effect_facts::grant_from_manifest_text(manifest, &std::env::current_dir().unwrap()).unwrap().unwrap();
        CapGate::new(g, CapMode::Enforce)
    }

    /// `cap_gate_ffi` is the only guard in front of host-bridge externs: a grant for one name must not cover another.
    #[test]
    fn ffi_calls_need_a_grant_by_name() {
        let mut it = Interpreter::new_capturing();
        it.set_caps(Some(gate("[capabilities]\nffi = [\"native_open\"]\n")));
        assert!(it.cap_gate_ffi("native_open").is_ok());
        let text = format!("{:?}", it.cap_gate_ffi("native_exec").unwrap_err());
        assert!(text.contains("capability denied: ffi(native_exec)"), "{text}");
        assert!(text.contains("add `ffi = [\\\"native_exec\\\"]` under [capabilities]"), "the fix line names the exact ffi entry: {text}");
        let mut none = Interpreter::new_capturing();
        none.set_caps(Some(gate("[capabilities]\n")));
        assert!(none.cap_gate_ffi("native_open").is_err(), "no ffi grant: denied");
        let mut open = Interpreter::new_capturing();
        open.set_caps(Some(gate("[capabilities]\nffi = true\n")));
        assert!(open.cap_gate_ffi("anything").is_ok());
    }

    #[test]
    fn console_gate_is_free_when_open_and_denies_when_revoked() {
        let mut it = Interpreter::new_capturing();
        it.set_caps(Some(gate("[capabilities]\n")));
        assert!(it.cap_gate_console().is_ok());
        assert!(it.caps_seen().contains(&EffectKind::Console), "printing is recorded for the soundness gate");
        let mut sandbox = Interpreter::new_capturing();
        sandbox.set_caps(Some(gate("[capabilities]\ndeny = [\"console\"]\n")));
        assert!(sandbox.cap_gate_console().is_err());
    }
}
