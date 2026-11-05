//! Run-time capability checks for compiled programs (ADR-0021 stage E2).
//!
//! The compiler installs the program's grant once, at the start of `main`, and only when it cannot prove every
//! sink call inside the grant at compile time (a computed path, host or name). Without an installed grant every
//! check is one atomic load and returns. A denial stops the process with exit status 126 and cannot be caught.
//!
//! Matching is `caps_core`, the same source the interpreter and `wj check` use.

use crate::caps_core::grant::{Denial, Grant};
use crate::caps_core::kind::EffectKind;
use std::cell::Cell;
use std::path::Path;
use std::sync::OnceLock;

static GRANT: OnceLock<Grant> = OnceLock::new();

thread_local! {
    /// Nonzero while a capability value (`cap::Net`, ...) performs an operation it already validated.
    static BYPASS: Cell<u32> = const { Cell::new(0) };
}

/// The grant a program was compiled with: `allow` is `(effect, patterns)`; a single `"*"` pattern means any resource.
pub struct Spec<'a> {
    pub root: &'a str,
    pub allow: &'a [(&'a str, &'a [&'a str])],
    pub deny: &'a [&'a str],
}

/// Install the grant. The first call wins; later calls are ignored.
pub fn install(spec: &Spec<'_>) {
    let mut g = Grant::sandbox(Path::new(spec.root));
    for (name, patterns) in spec.allow {
        let kind = EffectKind::parse_builtin(name).unwrap_or_else(|| EffectKind::User((*name).to_string()));
        if patterns.is_empty() {
            g.declare_empty(kind.clone());
        }
        for p in *patterns {
            g.allow(kind.clone(), p);
        }
    }
    for name in spec.deny {
        if let Some(k) = EffectKind::parse_builtin(name) {
            g.deny(k);
        }
    }
    let _ = GRANT.set(g);
}

/// The installed grant, if any (capability values mint from it).
pub fn installed() -> Option<&'static Grant> {
    GRANT.get()
}

fn fault(d: Denial) -> ! {
    eprintln!("error: capability denied: {}\n  = grant in force: {}\n  = to allow: {}", d.subject(), d.grant, d.manifest_fix());
    std::process::exit(126)
}

/// Check one use of `kind` on `resource` against the installed grant (no-op when none is installed).
pub fn check(kind: &EffectKind, resource: Option<&str>) {
    if BYPASS.with(|b| b.get()) > 0 {
        return;
    }
    if let Some(g) = GRANT.get() {
        if let Err(d) = g.check(kind, resource) {
            fault(d);
        }
    }
}

/// Run `f` with the ambient checks bypassed (a capability value validated the operation itself).
pub fn bypass<T>(f: impl FnOnce() -> T) -> T {
    BYPASS.with(|b| b.set(b.get() + 1));
    let r = f();
    BYPASS.with(|b| b.set(b.get() - 1));
    r
}

/// Open a file for reading and decide on the path of the open file, not on the name (a name can be re-pointed between a check and
/// an open, an open file cannot). Read from the returned handle. Without an installed grant this is `File::open`.
pub fn open_read(p: impl AsRef<Path>) -> std::io::Result<std::fs::File> {
    let name = p.as_ref();
    let grant = if BYPASS.with(|b| b.get()) > 0 { None } else { GRANT.get() };
    let Some(g) = grant else { return std::fs::File::open(name) };
    match g.open_read(&name.to_string_lossy()) {
        Ok(f) => Ok(f),
        Err(crate::caps_core::grant::OpenError::Io(e)) => Err(e),
        Err(crate::caps_core::grant::OpenError::Denied(d)) => fault(d),
    }
}

/// Open a file for writing (create and truncate, or append) and decide on its parent directory's descriptor, not on the name
/// (see `Grant::open_write`). Without an installed grant this opens by name.
pub fn open_write(p: impl AsRef<Path>, append: bool) -> std::io::Result<std::fs::File> {
    let name = p.as_ref();
    let by_name = || {
        let mut o = std::fs::OpenOptions::new();
        o.write(true).create(true);
        if append {
            o.append(true);
        } else {
            o.truncate(true);
        }
        o.open(name)
    };
    let grant = if BYPASS.with(|b| b.get()) > 0 { None } else { GRANT.get() };
    let Some(g) = grant else { return by_name() };
    match g.open_write(&name.to_string_lossy(), append) {
        Ok(Some(f)) => Ok(f),
        Ok(None) => by_name(),
        Err(crate::caps_core::grant::OpenError::Io(e)) => Err(e),
        Err(crate::caps_core::grant::OpenError::Denied(d)) => fault(d),
    }
}

/// `mkdir`, `unlink` or `rmdir` of one name, decided on its parent directory's descriptor (see `Grant::entry_op`).
pub fn entry_op(p: impl AsRef<Path>, op: crate::caps_core::grant::EntryOp) -> std::io::Result<()> {
    use crate::caps_core::grant::EntryOp;
    let name = p.as_ref();
    let by_name = || match op {
        EntryOp::MakeDir => std::fs::create_dir(name),
        EntryOp::RemoveFile => std::fs::remove_file(name),
        EntryOp::RemoveDir => std::fs::remove_dir(name),
    };
    let grant = if BYPASS.with(|b| b.get()) > 0 { None } else { GRANT.get() };
    let Some(g) = grant else { return by_name() };
    match g.entry_op(&name.to_string_lossy(), op) {
        Ok(Some(())) => Ok(()),
        Ok(None) => by_name(),
        Err(crate::caps_core::grant::OpenError::Io(e)) => Err(e),
        Err(crate::caps_core::grant::OpenError::Denied(d)) => fault(d),
    }
}

/// `rename` decided on both parent directories (see `Grant::rename_op`).
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<()> {
    let (a, b) = (from.as_ref(), to.as_ref());
    let grant = if BYPASS.with(|x| x.get()) > 0 { None } else { GRANT.get() };
    let Some(g) = grant else { return std::fs::rename(a, b) };
    match g.rename_op(&a.to_string_lossy(), &b.to_string_lossy()) {
        Ok(Some(())) => Ok(()),
        Ok(None) => std::fs::rename(a, b),
        Err(crate::caps_core::grant::OpenError::Io(e)) => Err(e),
        Err(crate::caps_core::grant::OpenError::Denied(d)) => fault(d),
    }
}

/// `copy` between two checked handles (see `Grant::copy_op`).
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<u64> {
    let (a, b) = (from.as_ref(), to.as_ref());
    let grant = if BYPASS.with(|x| x.get()) > 0 { None } else { GRANT.get() };
    let Some(g) = grant else { return std::fs::copy(a, b) };
    match g.copy_op(&a.to_string_lossy(), &b.to_string_lossy()) {
        Ok(Some(n)) => Ok(n),
        Ok(None) => std::fs::copy(a, b),
        Err(crate::caps_core::grant::OpenError::Io(e)) => Err(e),
        Err(crate::caps_core::grant::OpenError::Denied(d)) => fault(d),
    }
}

/// `create_dir_all` / `remove_dir_all` decided descriptor by descriptor (see `Grant::dir_all_op`).
pub fn dir_all(p: impl AsRef<Path>, op: crate::caps_core::grant::DirAllOp) -> std::io::Result<()> {
    use crate::caps_core::grant::DirAllOp;
    let name = p.as_ref();
    let by_name = || match op {
        DirAllOp::Create => std::fs::create_dir_all(name),
        DirAllOp::Remove => std::fs::remove_dir_all(name),
    };
    let grant = if BYPASS.with(|x| x.get()) > 0 { None } else { GRANT.get() };
    let Some(g) = grant else { return by_name() };
    match g.dir_all_op(&name.to_string_lossy(), op) {
        Ok(Some(())) => Ok(()),
        Ok(None) => by_name(),
        Err(crate::caps_core::grant::OpenError::Io(e)) => Err(e),
        Err(crate::caps_core::grant::OpenError::Denied(d)) => fault(d),
    }
}

pub fn fs_read(p: impl AsRef<Path>) {
    check(&EffectKind::FsRead, Some(&p.as_ref().to_string_lossy()));
}
pub fn fs_write(p: impl AsRef<Path>) {
    check(&EffectKind::FsWrite, Some(&p.as_ref().to_string_lossy()));
}
pub fn net_connect(url: &str) {
    check(&EffectKind::NetConnect, Some(url));
}
pub fn net_listen(addr: &str) {
    check(&EffectKind::NetListen, Some(addr));
}
pub fn env_read(name: &str) {
    check(&EffectKind::EnvRead, Some(name));
}
pub fn env_write(name: &str) {
    check(&EffectKind::EnvWrite, Some(name));
}
pub fn process(program: &str) {
    check(&EffectKind::Process, Some(program));
}
/// Enumerating the environment needs `env.read` for every name.
pub fn env_read_all() {
    check(&EffectKind::EnvRead, Some("*"));
}
/// A database connection string: a `postgres://` URL is a network connection, anything else a file.
pub fn db_open(url: &str) {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        net_connect(url);
    } else if url != ":memory:" && url != "sqlite::memory:" {
        fs_write(url.strip_prefix("sqlite:").unwrap_or(url));
    }
}
