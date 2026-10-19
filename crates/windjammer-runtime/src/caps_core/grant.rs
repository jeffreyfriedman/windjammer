//! Grants: what a package or a sandboxed run may do (`wj.toml [capabilities]`, `--sandbox`, `--allow`).
//!
//! A grant maps each effect to a scope: `Any`, or a list of resource patterns (paths, hosts, names).
//! `time`, `random` and `console` are granted unless denied (decision D5). `deny` always wins.

use super::kind::EffectKind;
use super::scope_match as m;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Any,
    Only(Vec<String>),
}

/// Why a check failed (shown in the `capability denied` message and the WJ0040/WJ0041 diagnostics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Denial {
    pub effect: EffectKind,
    /// Display form of the resource (`./secret.txt`, `other.example`), when the effect has one.
    pub resource: Option<String>,
    /// True when the effect is granted for other resources (WJ0041) rather than not at all (WJ0040).
    pub outside_scope: bool,
    /// The grant in force, e.g. `net.connect = [api.example.com]` or `net.connect not granted`.
    pub grant: String,
    /// The `wj.toml` line or `--allow` flag that would allow it.
    pub fix: String,
}

impl Denial {
    /// The `[capabilities]` line that would allow this use (`net.connect = ["api.example.com"]`).
    pub fn entry(&self) -> String {
        match &self.resource {
            Some(r) => format!("{} = [\"{r}\"]", self.effect),
            None => format!("{} = true", self.effect),
        }
    }

    /// The `--allow` argument that would allow this use (`net.connect=api.example.com`).
    pub fn flag(&self) -> String {
        match &self.resource {
            Some(r) => format!("{}={r}", self.effect),
            None => self.effect.name(),
        }
    }

    /// The manifest part of the fix, without the interpreter-only `--allow` flag (compiled programs have no such flag).
    pub fn manifest_fix(&self) -> &str {
        self.fix.split(", or run with").next().unwrap_or(&self.fix)
    }

    pub fn subject(&self) -> String {
        match &self.resource {
            Some(r) => format!("{}({})", self.effect, r),
            None => self.effect.name(),
        }
    }
}

#[derive(Debug)]
pub struct Grant {
    allow: BTreeMap<EffectKind, Scope>,
    deny: BTreeSet<EffectKind>,
    /// Directory that relative patterns are anchored to (the project directory).
    root: PathBuf,
    /// Canonical form of each pattern's literal prefix (patterns do not change, so symlink resolution is done once).
    prefix_cache: std::sync::Mutex<std::collections::HashMap<String, PathBuf>>,
}

impl Clone for Grant {
    fn clone(&self) -> Grant {
        Grant { allow: self.allow.clone(), deny: self.deny.clone(), root: self.root.clone(), prefix_cache: Default::default() }
    }
}

impl Grant {
    /// Nothing but the default-granted determinism effects.
    pub fn sandbox(root: &Path) -> Grant {
        Grant { allow: BTreeMap::new(), deny: BTreeSet::new(), root: m::lexical_abs(".", root_base(root).as_path()), prefix_cache: Default::default() }
    }

    /// Everything allowed (a trusted run).
    pub fn unrestricted(root: &Path) -> Grant {
        let mut g = Grant::sandbox(root);
        for k in EffectKind::BUILTIN {
            g.allow.insert(k, Scope::Any);
        }
        g
    }

    /// Decision D1: files below the project directory, plus the default-granted effects.
    pub fn local_profile(root: &Path) -> Grant {
        let mut g = Grant::sandbox(root);
        g.allow(EffectKind::FsRead, "./**");
        g.allow(EffectKind::FsWrite, "./**");
        g
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn allow(&mut self, kind: EffectKind, pattern: &str) {
        if pattern == "*" || (pattern == "**" && matches!(kind, EffectKind::FsRead | EffectKind::FsWrite)) {
            self.allow.insert(kind, Scope::Any);
            return;
        }
        match self.allow.entry(kind).or_insert_with(|| Scope::Only(Vec::new())) {
            Scope::Any => {}
            Scope::Only(v) => {
                if !v.iter().any(|p| p == pattern) {
                    v.push(pattern.to_string());
                }
            }
        }
    }

    /// Grant exactly what a lock-file reference names: `kind` (no resource), `kind(resource)`, `kind(*)` (any resource).
    pub fn allow_ref(&mut self, r: &str) {
        let (name, res) = match r.split_once('(') {
            Some((n, rest)) => (n, Some(rest.trim_end_matches(')'))),
            None => (r, None),
        };
        let kind = EffectKind::parse_builtin(name).unwrap_or_else(|| EffectKind::User(name.to_string()));
        match res {
            Some("*") => self.allow_all(kind),
            Some(lit) => self.allow(kind, lit),
            None => self.allow_all(kind),
        }
    }

    /// Mention `kind` with an empty pattern list: declared, but nothing is allowed yet (`fs.read = []`).
    pub fn declare_empty(&mut self, kind: EffectKind) {
        self.allow.entry(kind).or_insert_with(|| Scope::Only(Vec::new()));
    }

    pub fn allow_all(&mut self, kind: EffectKind) {
        self.allow.insert(kind, Scope::Any);
    }

    pub fn deny(&mut self, kind: EffectKind) {
        self.deny.insert(kind);
    }

    pub fn scope(&self, kind: &EffectKind) -> Option<&Scope> {
        self.allow.get(kind)
    }

    /// A capability value's grant (`std::cap`): only `kind`, restricted to `patterns`, each of which this grant already
    /// covers. `patterns` empty = the whole of this grant's scope for `kind` (`cap::net()`); anything this grant does
    /// not cover is refused, so narrowing can never widen.
    pub fn derive_scope(&self, kind: &EffectKind, patterns: Option<&[String]>) -> Result<Grant, Denial> {
        let mut out = Grant { allow: BTreeMap::new(), deny: BTreeSet::new(), root: self.root.clone(), prefix_cache: Default::default() };
        let Some(scope) = self.allow.get(kind) else {
            return match self.check(kind, None) {
                Err(d) => Err(d),
                Ok(()) => Ok(out), // a default-granted effect (time, random, console): unscoped
            };
        };
        match (patterns, scope) {
            (None, s) => {
                out.allow.insert(kind.clone(), s.clone());
            }
            (Some(ps), scope) => {
                out.allow.insert(kind.clone(), Scope::Only(Vec::new()));
                for p in ps {
                    if !self.covers_pattern(kind, scope, p) {
                        return Err(self.denial(kind, Some(p), true, format!("{kind} = [{}]", match scope {
                            Scope::Any => "*".to_string(),
                            Scope::Only(v) => v.join(", "),
                        })));
                    }
                    out.allow(kind.clone(), p);
                }
            }
        }
        Ok(out)
    }

    /// The grant as `(effect name, patterns)` pairs; `Any` is the single pattern `"*"` (for the compiler to render into a
    /// compiled program's `caps::install`).
    pub fn entries(&self) -> Vec<(String, Vec<String>)> {
        self.allow
            .iter()
            .map(|(k, s)| match s {
                Scope::Any => (k.name(), vec!["*".to_string()]),
                Scope::Only(p) => (k.name(), p.clone()),
            })
            .collect()
    }

    /// Names of the denied effects.
    pub fn denied(&self) -> Vec<String> {
        self.deny.iter().map(|k| k.name()).collect()
    }

    /// Forget everything this grant says about `kind` (a read-only value drops `fs.write`).
    pub fn drop_scope(&mut self, kind: &EffectKind) {
        self.allow.remove(kind);
    }

    /// Add the scopes of `other` (used to assemble a value that carries both `fs.read` and `fs.write`).
    pub fn absorb(&mut self, other: Grant) {
        for (k, s) in other.allow {
            self.allow.entry(k).or_insert(s);
        }
    }

    fn covers_pattern(&self, kind: &EffectKind, scope: &Scope, pattern: &str) -> bool {
        let pats = match scope {
            Scope::Any => return true,
            Scope::Only(p) => p,
        };
        if pats.iter().any(|q| q == pattern) {
            return true;
        }
        match pattern.strip_suffix("/**") {
            // a directory subtree: its root must be inside the grant
            Some(dir) if matches!(kind, EffectKind::FsRead | EffectKind::FsWrite) => self.check(kind, Some(dir)).is_ok(),
            _ if pattern.contains('*') => false,
            _ => self.check(kind, Some(pattern)).is_ok(),
        }
    }

    /// `--allow net.connect=api.example.com` (repeatable); `effects=ledger.post` grants a user effect.
    pub fn allow_flag(&mut self, spec: &str) -> Result<(), String> {
        let (name, value) = spec.split_once('=').map(|(a, b)| (a.trim(), Some(b.trim()))).unwrap_or((spec.trim(), None));
        if name == "effects" {
            let v = value.ok_or("`--allow effects=NAME` needs a value")?;
            self.allow_all(EffectKind::User(v.to_string()));
            return Ok(());
        }
        let kind = EffectKind::parse_builtin(name).ok_or_else(|| format!("unknown effect `{name}` (known: {})", known_effect_names()))?;
        match value {
            None | Some("*") | Some("") => self.allow_all(kind),
            Some(v) => self.allow(kind, v),
        }
        Ok(())
    }

    /// Is `kind` granted for at least some resource? (Static check of a use whose resource is computed:
    /// only a runtime guard can decide the rest.)
    pub fn granted_at_all(&self, kind: &EffectKind) -> Result<(), Denial> {
        if self.deny.contains(kind) {
            return Err(self.denial(kind, None, false, format!("{kind} is denied")));
        }
        match self.allow.get(kind) {
            Some(Scope::Only(p)) if p.is_empty() => Err(self.denial(kind, None, false, format!("{kind} not granted"))),
            Some(_) => Ok(()),
            None if kind.default_granted() => Ok(()),
            None => Err(self.denial(kind, None, false, format!("{kind} not granted"))),
        }
    }

    /// Check one use of `kind` on `resource` (the raw argument: a path, URL/address, variable name, program).
    pub fn check(&self, kind: &EffectKind, resource: Option<&str>) -> Result<(), Denial> {
        // standard devices are the console / the entropy source, not files (`fs::read_to_string("/dev/stdin")`)
        if let (EffectKind::FsRead | EffectKind::FsWrite, Some(res)) = (kind, resource) {
            match device_effect(kind, res) {
                Some(None) => return Ok(()),
                Some(Some(effect)) => return self.check(&effect, None),
                None => {}
            }
        }
        if self.deny.contains(kind) {
            return Err(self.denial(kind, resource, false, format!("{kind} is denied")));
        }
        let scope = match self.allow.get(kind) {
            Some(s) => s,
            None if kind.default_granted() => return Ok(()),
            None => return Err(self.denial(kind, resource, false, format!("{kind} not granted"))),
        };
        let pats = match scope {
            Scope::Any => return Ok(()),
            Scope::Only(p) => p,
        };
        let Some(res) = resource else {
            // a resource-refined effect used with an unknown resource cannot be shown to be inside a narrowed scope
            let needs_resource = matches!(
                kind,
                EffectKind::FsRead | EffectKind::FsWrite | EffectKind::NetConnect | EffectKind::NetListen | EffectKind::EnvRead | EffectKind::EnvWrite | EffectKind::Process | EffectKind::Ffi
            );
            return if needs_resource {
                Err(self.denial(kind, None, true, format!("{kind} = [{}]", pats.join(", "))))
            } else if pats.is_empty() {
                Err(self.denial(kind, None, false, format!("{kind} not granted")))
            } else {
                Ok(())
            };
        };
        let ok = match kind {
            EffectKind::FsRead | EffectKind::FsWrite => {
                let p = m::canonical_abs(res, &Self::cwd());
                let inside = |pat: &String| {
                    m::path_matches_cached(pat, &p, &self.root, &mut |prefix, base| {
                        self.prefix_cache.lock().unwrap_or_else(|e| e.into_inner()).entry(prefix.to_string()).or_insert_with(|| m::canonical_abs(prefix, base)).clone()
                    })
                };
                if *kind == EffectKind::FsWrite && is_protected(&p) {
                    // the manifest, the lock file and program sources can be written only when a pattern names them exactly
                    pats.iter().any(|pat| !pat.contains('*') && inside(pat))
                } else {
                    pats.iter().any(inside)
                }
            }
            EffectKind::NetConnect | EffectKind::NetListen => match m::host_port(res) {
                Some((h, port)) => pats.iter().any(|pat| m::host_matches(pat, &h, port)),
                None => false,
            },
            EffectKind::EnvRead | EffectKind::EnvWrite => pats.iter().any(|pat| m::name_matches(pat, res)),
            EffectKind::Process => {
                // a bare name is looked up on PATH and matched by name; a path (./echo, /bin/ls) only by that exact path,
                // so a local binary cannot borrow the name of a granted program
                let prog = res.split_whitespace().next().unwrap_or("");
                if prog.contains('/') {
                    pats.iter().any(|pat| pat == prog)
                } else {
                    pats.iter().any(|pat| m::name_matches(pat, prog))
                }
            }
            EffectKind::Ffi => pats.iter().any(|pat| m::name_matches(pat, res)),
            _ => true,
        };
        if ok {
            Ok(())
        } else {
            Err(self.denial(kind, Some(res), true, format!("{kind} = [{}]", pats.join(", "))))
        }
    }

    fn cwd() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    }

    fn display_resource(&self, kind: &EffectKind, res: &str) -> String {
        match kind {
            EffectKind::FsRead | EffectKind::FsWrite => {
                let p = m::lexical_abs(res, &Self::cwd());
                match p.strip_prefix(&self.root) {
                    Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
                    Ok(rel) => format!("./{}", rel.display()),
                    Err(_) => p.display().to_string(),
                }
            }
            EffectKind::NetConnect | EffectKind::NetListen => match m::host_port(res) {
                Some((h, _)) => h,
                None => res.to_string(),
            },
            EffectKind::Process => res.split_whitespace().next().unwrap_or(res).to_string(),
            _ => res.to_string(),
        }
    }

    fn denial(&self, kind: &EffectKind, resource: Option<&str>, outside_scope: bool, grant: String) -> Denial {
        let shown = resource.map(|r| self.display_resource(kind, r));
        let entry = match &shown {
            Some(r) => format!("{kind} = [\"{r}\"]"),
            None => format!("{kind} = true"),
        };
        let flag_val = shown.as_deref().map(|r| format!("{kind}={r}")).unwrap_or_else(|| kind.name());
        Denial {
            effect: kind.clone(),
            resource: shown,
            outside_scope,
            grant,
            fix: format!("add `{entry}` under [capabilities] in wj.toml, or run with `--allow {flag_val}`"),
        }
    }
}

/// `root` as an absolute, lexically normalized directory (a relative root is taken against the current directory).
fn root_base(root: &Path) -> PathBuf {
    if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")).join(root)
    }
}

/// `/dev/stdin` (read) and `/dev/stdout`, `/dev/stderr` (write) are the console; `/dev/urandom`, `/dev/random` (read) are
/// randomness; `/dev/null` is nothing. `Some(None)` = always allowed, `Some(Some(e))` = checked as effect `e`.
fn device_effect(kind: &EffectKind, res: &str) -> Option<Option<EffectKind>> {
    let reading = *kind == EffectKind::FsRead;
    match res {
        "/dev/null" => Some(None),
        "/dev/stdin" if reading => Some(Some(EffectKind::Console)),
        "/dev/stdout" | "/dev/stderr" if !reading => Some(Some(EffectKind::Console)),
        "/dev/urandom" | "/dev/random" if reading => Some(Some(EffectKind::Random)),
        _ => None,
    }
}

/// Files that define what a program may do or is: never reachable through a glob grant.
fn is_protected(p: &Path) -> bool {
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    name == "wj.toml" || name.ends_with(".wj-capabilities.lock") || name.ends_with(".wj")
}

pub fn known_effect_names() -> String {
    EffectKind::BUILTIN.iter().map(|k| k.name()).collect::<Vec<_>>().join(", ")
}
