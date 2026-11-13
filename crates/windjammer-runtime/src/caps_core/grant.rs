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

/// Whole-tree directory operations decided descriptor by descriptor (`Grant::dir_all_op`).
#[derive(Debug, Clone, Copy)]
pub enum DirAllOp {
    Create,
    Remove,
}

/// One directory-entry operation decided on its parent directory (`Grant::entry_op`).
#[derive(Debug, Clone, Copy)]
pub enum EntryOp {
    MakeDir,
    RemoveFile,
    RemoveDir,
}

/// Why `Grant::open_read` did not return a file.
#[derive(Debug)]
pub enum OpenError {
    Denied(Denial),
    Io(std::io::Error),
}

#[cfg(unix)]
use super::fdio;

/// The canonical path of an open file, from its descriptor; none where the platform cannot say.
fn path_of_open_file(f: &std::fs::File) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        fdio::path_of_open_file(f)
    }
    #[cfg(not(unix))]
    {
        let _ = f;
        None
    }
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
    /// Resources whose `fs.read` was allowed, valid while no operation that can change what a path means has run (see `ReadCache`).
    read_cache: std::sync::Mutex<ReadCache>,
}

/// Decisions for `fs.read` of a resource, so a loop that reads the same path does not resolve its symlinks on every call.
///
/// A cached decision is only as good as the file system it was made against. It is dropped when the program does anything that
/// could change what a path resolves to: every granted `fs.write` and `env.write` (a rename, a new link, `set_current_dir`)
/// bumps `GENERATION`, and every granted `process` or `ffi` use (code outside the gate that may change files at any time) or a
/// second thread using the gate turns the cache off for the rest of the run. Denials are never cached.
#[derive(Debug, Default)]
struct ReadCache {
    generation: u64,
    allowed: std::collections::HashSet<String>,
    /// The thread that uses this grant; another thread using it turns its cache off for good.
    owner: u64,
    disabled: bool,
    /// When the cached decisions were last known to be fresh; older than `CACHE_TTL` they are dropped, which bounds how long a
    /// change made behind the program's back (another process swapping a link) can go unnoticed.
    stamp: Option<std::time::Instant>,
}

const CACHE_TTL: std::time::Duration = std::time::Duration::from_micros(100);

/// How long a cached `fs.read` decision may be served (the bound on the staleness of names that are only checked).
pub fn decision_ttl() -> std::time::Duration {
    CACHE_TTL
}

/// The working directory, looked up once per generation (`set_current_dir` is an `env.write`, which starts a new one).
static CWD: std::sync::Mutex<Option<(u64, PathBuf)>> = std::sync::Mutex::new(None);

static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static CACHE_OFF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static NEXT_TID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

thread_local! {
    static TID: u64 = NEXT_TID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// Forget that a process or foreign call has run, and every cached decision (the escape tests start each case from here).
#[doc(hidden)]
pub fn reset_decision_cache() {
    use std::sync::atomic::Ordering::Relaxed;
    CACHE_OFF.store(false, Relaxed);
    GENERATION.fetch_add(1, Relaxed);
}

impl Clone for Grant {
    fn clone(&self) -> Grant {
        Grant { allow: self.allow.clone(), deny: self.deny.clone(), root: self.root.clone(), prefix_cache: Default::default(), read_cache: Default::default() }
    }
}

impl Grant {
    /// Nothing but the default-granted determinism effects.
    pub fn sandbox(root: &Path) -> Grant {
        Grant { allow: BTreeMap::new(), deny: BTreeSet::new(), root: m::lexical_abs(".", root_base(root).as_path()), prefix_cache: Default::default(), read_cache: Default::default() }
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
        self.forget_decisions();
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
        self.forget_decisions();
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
        self.forget_decisions();
        self.allow.entry(kind).or_insert_with(|| Scope::Only(Vec::new()));
    }

    pub fn allow_all(&mut self, kind: EffectKind) {
        self.forget_decisions();
        self.allow.insert(kind, Scope::Any);
    }

    pub fn deny(&mut self, kind: EffectKind) {
        self.forget_decisions();
        self.deny.insert(kind);
    }

    pub fn scope(&self, kind: &EffectKind) -> Option<&Scope> {
        self.allow.get(kind)
    }

    /// A capability value's grant (`std::cap`): only `kind`, restricted to `patterns`, each of which this grant already
    /// covers. `patterns` empty = the whole of this grant's scope for `kind` (`cap::net()`); anything this grant does
    /// not cover is refused, so narrowing can never widen.
    pub fn derive_scope(&self, kind: &EffectKind, patterns: Option<&[String]>) -> Result<Grant, Denial> {
        let mut out = Grant { allow: BTreeMap::new(), deny: BTreeSet::new(), root: self.root.clone(), prefix_cache: Default::default(), read_cache: Default::default() };
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
        self.forget_decisions();
        self.allow.remove(kind);
    }

    /// Add the scopes of `other` (used to assemble a value that carries both `fs.read` and `fs.write`).
    pub fn absorb(&mut self, other: Grant) {
        self.forget_decisions();
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
        use std::sync::atomic::Ordering::Relaxed;
        // no decision cache on wasm32: it is stamped with a clock that platform does not have (the browser playground,
        // ADR-0034, has no file system to cache decisions about anyway)
        let cacheable_read = matches!((kind, resource), (EffectKind::FsRead, Some(_))) && !CACHE_OFF.load(Relaxed) && !cfg!(target_arch = "wasm32");
        if cacheable_read {
            let res = resource.unwrap_or("");
            let me = TID.with(|t| *t);
            let mut c = self.read_cache.lock().unwrap_or_else(|e| e.into_inner());
            if c.owner == 0 {
                c.owner = me;
            } else if c.owner != me {
                c.disabled = true;
            }
            if !c.disabled {
                let now = GENERATION.load(Relaxed);
                let fresh = c.stamp.is_some_and(|t| t.elapsed() < CACHE_TTL);
                if c.generation != now || !fresh {
                    c.allowed.clear();
                    c.generation = now;
                    c.stamp = if cfg!(target_arch = "wasm32") { None } else { Some(std::time::Instant::now()) };
                }
                if c.allowed.contains(res) {
                    return Ok(());
                }
            }
        }
        let decision = self.check_with(kind, resource, None);
        if decision.is_ok() {
            match kind {
                // the action that follows this check may change what a path resolves to
                EffectKind::FsWrite | EffectKind::EnvWrite => {
                    GENERATION.fetch_add(1, Relaxed);
                }
                EffectKind::Process | EffectKind::Ffi => CACHE_OFF.store(true, Relaxed),
                EffectKind::FsRead if cacheable_read => {
                    let mut c = self.read_cache.lock().unwrap_or_else(|e| e.into_inner());
                    if !c.disabled && c.generation == GENERATION.load(Relaxed) {
                        c.allowed.insert(resource.unwrap_or("").to_string());
                    }
                }
                _ => {}
            }
        }
        decision
    }

    /// Open `res` for reading and decide on the path of **the file that was opened**, not on the name: a name can be re-pointed
    /// (a link swapped by another process) between a check and an open, an open file cannot. The returned handle is the one
    /// that was checked; read from it, do not open the name again. Denials are made before any byte is read, and opening a
    /// file for reading changes nothing on disk.
    ///
    /// When `res` cannot be opened the name decides (an existing file outside the grant and a missing one both give the denial,
    /// so the answer is no oracle for existence); allowed, the error is the caller's I/O error.
    pub fn open_read(&self, res: &str) -> Result<std::fs::File, OpenError> {
        if device_effect(&EffectKind::FsRead, res).is_some() {
            self.check(&EffectKind::FsRead, Some(res)).map_err(OpenError::Denied)?;
            return std::fs::File::open(res).map_err(OpenError::Io);
        }
        let file = match std::fs::File::open(res) {
            Ok(f) => f,
            Err(e) => {
                self.check(&EffectKind::FsRead, Some(res)).map_err(OpenError::Denied)?;
                return Err(OpenError::Io(e));
            }
        };
        match path_of_open_file(&file) {
            Some(real) => self.check_with(&EffectKind::FsRead, Some(res), Some(real)).map_err(OpenError::Denied)?,
            // a platform that cannot name an open file: the name decides (the old, racy order)
            None => self.check(&EffectKind::FsRead, Some(res)).map_err(OpenError::Denied)?,
        }
        Ok(file)
    }

    /// Open `res` for writing (create and truncate, or append) after deciding on the **parent directory** it lives in: the
    /// directory is opened first, its canonical path comes from the descriptor, `parent/name` is checked, and the file is opened
    /// relative to that same descriptor with `O_NOFOLLOW` (see `fdio`). A name that is a link is followed by hand and checked
    /// again. `Ok(None)` means the caller writes by name (a device such as `/dev/stdout`, or a platform without descriptor paths).
    /// An `Io` error is the operation's own error; nothing was written.
    pub fn open_write(&self, res: &str, append: bool) -> Result<Option<std::fs::File>, OpenError> {
        #[cfg(unix)]
        {
            self.checked_parent(res, true, &mut |p| fdio::open_for_write(p, append))
        }
        #[cfg(not(unix))]
        {
            let _ = append;
            self.check(&EffectKind::FsWrite, Some(res)).map_err(OpenError::Denied)?;
            Ok(None)
        }
    }

    /// `mkdir`, `unlink` and `rmdir` of one name, decided on the parent directory like `open_write`. `Ok(None)`: do it by name.
    pub fn entry_op(&self, res: &str, op: EntryOp) -> Result<Option<()>, OpenError> {
        #[cfg(unix)]
        {
            self.checked_parent(res, false, &mut |p| match op {
                EntryOp::MakeDir => fdio::make_dir(p),
                EntryOp::RemoveFile => fdio::unlink(p, false),
                EntryOp::RemoveDir => fdio::unlink(p, true),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = op;
            self.check(&EffectKind::FsWrite, Some(res)).map_err(OpenError::Denied)?;
            Ok(None)
        }
    }

    /// `rename` decided on both parent directories (two descriptors): each canonical parent comes from its descriptor, both
    /// `parent/name` are checked, and the move is one `renameat` between those same descriptors. `Ok(None)`: do it by name.
    pub fn rename_op(&self, from: &str, to: &str) -> Result<Option<()>, OpenError> {
        #[cfg(unix)]
        {
            use std::sync::atomic::Ordering::Relaxed;
            if device_effect(&EffectKind::FsWrite, from).is_some() || device_effect(&EffectKind::FsWrite, to).is_some() {
                self.check(&EffectKind::FsWrite, Some(from)).map_err(OpenError::Denied)?;
                self.check(&EffectKind::FsWrite, Some(to)).map_err(OpenError::Denied)?;
                return Ok(None);
            }
            let mut parents = Vec::new();
            for res in [from, to] {
                let lex = m::lexical_abs(res, &Self::base_for(res));
                match fdio::open_parent(&lex) {
                    Ok(p) => {
                        self.check_with(&EffectKind::FsWrite, Some(res), Some(p.canon.join(&p.name))).map_err(OpenError::Denied)?;
                        parents.push(p);
                    }
                    Err(e) => {
                        self.check(&EffectKind::FsWrite, Some(res)).map_err(OpenError::Denied)?;
                        return Err(OpenError::Io(e));
                    }
                }
            }
            GENERATION.fetch_add(1, Relaxed);
            fdio::rename_at(&parents[0], &parents[1]).map(Some).map_err(OpenError::Io)
        }
        #[cfg(not(unix))]
        {
            self.check(&EffectKind::FsWrite, Some(from)).map_err(OpenError::Denied)?;
            self.check(&EffectKind::FsWrite, Some(to)).map_err(OpenError::Denied)?;
            Ok(None)
        }
    }

    /// `copy`: the source is opened and decided on as an open file, the destination as `open_write` does, and the bytes go
    /// from the one handle to the other (permissions follow, as `std::fs::copy` does). `Ok(None)`: do it by name.
    pub fn copy_op(&self, from: &str, to: &str) -> Result<Option<u64>, OpenError> {
        let mut src = self.open_read(from)?;
        let Some(mut dst) = self.open_write(to, false)? else { return Ok(None) };
        let n = std::io::copy(&mut src, &mut dst).map_err(OpenError::Io)?;
        if let Ok(meta) = src.metadata() {
            let _ = dst.set_permissions(meta.permissions());
        }
        Ok(Some(n))
    }

    /// `create_dir_all` and `remove_dir_all`. The whole path is first checked by name (so nothing is created for a path that is
    /// out of scope, as before); then each missing component is created relative to the descriptor of the directory before it,
    /// after checking `canonical(parent)/name` from that descriptor, and the final directory is checked once more. A removal
    /// is decided on the parent directory and never follows a link inside the tree.
    pub fn dir_all_op(&self, res: &str, op: DirAllOp) -> Result<Option<()>, OpenError> {
        #[cfg(unix)]
        {
            use std::sync::atomic::Ordering::Relaxed;
            if let DirAllOp::Remove = op {
                return self.checked_parent(res, false, &mut |p| fdio::remove_tree(p));
            }
            if device_effect(&EffectKind::FsWrite, res).is_some() {
                self.check(&EffectKind::FsWrite, Some(res)).map_err(OpenError::Denied)?;
                return Ok(None);
            }
            self.check_with(&EffectKind::FsWrite, Some(res), None).map_err(OpenError::Denied)?;
            let lex = m::lexical_abs(res, &Self::base_for(res));
            let mut cur = std::fs::File::open("/").map_err(OpenError::Io)?;
            let mut cur_canon = path_of_open_file(&cur).unwrap_or_else(|| PathBuf::from("/"));
            for comp in lex.components() {
                let std::path::Component::Normal(name) = comp else { continue };
                match fdio::open_dir_at(&cur, name) {
                    Ok(next) => cur = next,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        self.check_with(&EffectKind::FsWrite, Some(res), Some(cur_canon.join(name))).map_err(OpenError::Denied)?;
                        GENERATION.fetch_add(1, Relaxed);
                        match fdio::make_dir_at(&cur, name) {
                            Ok(()) => {}
                            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                            Err(e) => return Err(OpenError::Io(e)),
                        }
                        cur = fdio::open_dir_at(&cur, name).map_err(OpenError::Io)?;
                    }
                    Err(e) => return Err(OpenError::Io(e)),
                }
                cur_canon = path_of_open_file(&cur).ok_or_else(|| OpenError::Io(std::io::Error::new(std::io::ErrorKind::Unsupported, "cannot name an open directory")))?;
            }
            self.check_with(&EffectKind::FsWrite, Some(res), Some(cur_canon)).map_err(OpenError::Denied)?;
            Ok(Some(()))
        }
        #[cfg(not(unix))]
        {
            let _ = op;
            self.check(&EffectKind::FsWrite, Some(res)).map_err(OpenError::Denied)?;
            Ok(None)
        }
    }

    #[cfg(unix)]
    fn checked_parent<T>(&self, res: &str, follow_last: bool, op: &mut dyn FnMut(&fdio::Parent) -> std::io::Result<T>) -> Result<Option<T>, OpenError> {
        use std::sync::atomic::Ordering::Relaxed;
        if device_effect(&EffectKind::FsWrite, res).is_some() {
            self.check(&EffectKind::FsWrite, Some(res)).map_err(OpenError::Denied)?;
            return Ok(None);
        }
        let mut lex = m::lexical_abs(res, &Self::base_for(res));
        for _ in 0..40 {
            let parent = match fdio::open_parent(&lex) {
                Ok(p) => p,
                // the parent cannot be opened (missing, not a directory): the name decides, then the caller sees the error
                Err(e) => {
                    self.check(&EffectKind::FsWrite, Some(res)).map_err(OpenError::Denied)?;
                    return Err(OpenError::Io(e));
                }
            };
            let target = parent.canon.join(&parent.name);
            self.check_with(&EffectKind::FsWrite, Some(res), Some(target.clone())).map_err(OpenError::Denied)?;
            // like any granted write: what a path means may change from here on
            GENERATION.fetch_add(1, Relaxed);
            match op(&parent) {
                Ok(v) => return Ok(Some(v)),
                Err(e) if follow_last && fdio::is_symlink_loop(&e) => {
                    let link = std::fs::read_link(&target).map_err(OpenError::Io)?;
                    lex = m::lexical_abs(&link.to_string_lossy(), &parent.canon);
                }
                Err(e) => return Err(OpenError::Io(e)),
            }
        }
        Err(OpenError::Io(std::io::Error::from_raw_os_error(libc::ELOOP)))
    }

    fn forget_decisions(&mut self) {
        if let Ok(c) = self.read_cache.get_mut() {
            c.allowed.clear();
        }
    }

    /// The decision, from scratch. `canonical` is the already-resolved path of a file that is open (see `open_read`); without it the
    /// resource is resolved by name.
    fn check_with(&self, kind: &EffectKind, resource: Option<&str>, canonical: Option<PathBuf>) -> Result<(), Denial> {
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
                let p = canonical.unwrap_or_else(|| m::canonical_abs(res, &Self::base_for(res)));
                let inside = |pat: &String| {
                    m::path_matches_cached(pat, &p, &self.root, &mut |prefix, base| {
                        self.prefix_cache.lock().unwrap_or_else(|e| e.into_inner()).entry(prefix.to_string()).or_insert_with(|| m::canonical_abs(prefix, base)).clone()
                    })
                };
                if *kind == EffectKind::FsWrite && is_protected(&p) {
                    // the manifest, the lock file and program sources can be written only when a pattern names them exactly,
                    // or spells out the protected suffix in its last segment (`./gen/*.wj`, `./tmp/**/wj.toml`) - a deliberate
                    // grant; `./**` is not
                    pats.iter().any(|pat| (!pat.contains('*') || names_protected_suffix(pat, &p)) && inside(pat))
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
        use std::sync::atomic::Ordering::Relaxed;
        let direct = || std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        if CACHE_OFF.load(Relaxed) {
            return direct();
        }
        let now = GENERATION.load(Relaxed);
        let mut slot = CWD.lock().unwrap_or_else(|e| e.into_inner());
        match slot.as_ref() {
            Some((g, p)) if *g == now => p.clone(),
            _ => {
                let p = direct();
                *slot = Some((now, p.clone()));
                p
            }
        }
    }

    /// The base a resource is resolved against: an absolute path needs no working directory.
    fn base_for(res: &str) -> PathBuf {
        if res.starts_with('/') {
            PathBuf::from("/")
        } else {
            Self::cwd()
        }
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

/// A glob whose last segment (which does not itself cross directories) ends in the protected file's own suffix:
/// `*.wj` for a program source, `wj.toml` for a manifest, `.wj-capabilities.lock` for a lock file. The grant says
/// what it reaches out loud.
fn names_protected_suffix(pat: &str, p: &Path) -> bool {
    // names compare case-insensitively: case-insensitive file systems resolve `WJ.TOML` to the manifest
    let last = pat.rsplit('/').next().unwrap_or(pat).to_ascii_lowercase();
    let name = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let suffix = if name.ends_with(".wj-capabilities.lock") {
        ".wj-capabilities.lock"
    } else if name == "wj.toml" {
        "wj.toml"
    } else {
        ".wj"
    };
    last.ends_with(suffix) && !last.contains("**")
}

/// Files that define what a program may do or is: never reachable through a glob grant.
fn is_protected(p: &Path) -> bool {
    let name = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    name == "wj.toml" || name.ends_with(".wj-capabilities.lock") || name.ends_with(".wj")
}

pub fn known_effect_names() -> String {
    EffectKind::BUILTIN.iter().map(|k| k.name()).collect::<Vec<_>>().join(", ")
}
