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
