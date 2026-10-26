//! `std::cap` for compiled programs (ADR-0021 section 6): capability values.
//!
//! A value is an unforgeable handle (no public constructor except the `net()`/`fs()`/... mints below) holding a
//! narrowed [`Grant`]. An operation through it is checked against that grant and then performed with the ambient
//! checks bypassed, so a library that is *handed* a `Net` needs no ambient `net.connect` of its own. Minting is
//! checked against the program's installed grant; narrowing can only shrink. A denial exits with status 126.

use crate::caps;
use crate::caps_core::grant::{Denial, Grant};
use crate::caps_core::kind::EffectKind;
use crate::http::Response;
use crate::{env, fs, http, process, random, time};
use std::path::Path;

fn fault(d: Denial) -> ! {
    eprintln!(
        "error: capability denied: {}\n  = grant in force: the capability value you were given ({})\n  = to allow: mint a wider value where the ambient grant allows it, or pass a different one",
        d.subject(),
        d.grant
    );
    std::process::exit(126)
}

fn base() -> Grant {
    match caps::installed() {
        Some(g) => g.clone(),
        None => Grant::unrestricted(&std::env::current_dir().unwrap_or_default()),
    }
}

fn mint(kind: EffectKind) -> Grant {
    base().derive_scope(&kind, None).unwrap_or_else(|d| fault(d))
}

fn narrow(g: &Grant, kind: EffectKind, patterns: &[String]) -> Grant {
    g.derive_scope(&kind, Some(patterns)).unwrap_or_else(|d| fault(d))
}

fn through<T>(g: &Grant, kind: EffectKind, resource: Option<&str>, f: impl FnOnce() -> T) -> T {
    if let Err(d) = g.check(&kind, resource) {
        fault(d);
    }
    caps::bypass(f)
}

#[derive(Clone, Debug)]
pub struct Net(Grant);
#[derive(Clone, Debug)]
pub struct Fs(Grant);
#[derive(Clone, Debug)]
pub struct Env(Grant);
#[derive(Clone, Debug)]
pub struct Process(Grant);
#[derive(Clone, Debug)]
pub struct Clock(Grant);
#[derive(Clone, Debug)]
pub struct Rng(Grant);

pub fn net() -> Net {
    Net(mint(EffectKind::NetConnect))
}
pub fn fs() -> Fs {
    let mut g = mint(EffectKind::FsRead);
    if let Ok(w) = base().derive_scope(&EffectKind::FsWrite, None) {
        g.absorb(w);
    }
    Fs(g)
}
pub fn env() -> Env {
    Env(mint(EffectKind::EnvRead))
}
pub fn process() -> Process {
    Process(mint(EffectKind::Process))
}
pub fn clock() -> Clock {
    Clock(mint(EffectKind::Time))
}
pub fn rng() -> Rng {
    Rng(mint(EffectKind::Random))
}

impl Net {
    pub fn only(&self, hosts: Vec<String>) -> Net {
        Net(narrow(&self.0, EffectKind::NetConnect, &hosts))
    }
    pub fn get(&self, url: &str) -> Result<Response, String> {
        through(&self.0, EffectKind::NetConnect, Some(url), || http::get(url))
    }
    pub fn post(&self, url: &str, body: &str) -> Result<Response, String> {
        through(&self.0, EffectKind::NetConnect, Some(url), || http::post(url, body))
    }
    pub fn put(&self, url: &str, body: &str) -> Result<Response, String> {
        through(&self.0, EffectKind::NetConnect, Some(url), || http::put(url, body))
    }
    pub fn delete(&self, url: &str) -> Result<Response, String> {
        through(&self.0, EffectKind::NetConnect, Some(url), || http::delete(url))
    }
    pub fn head(&self, url: &str) -> Result<Response, String> {
        through(&self.0, EffectKind::NetConnect, Some(url), || http::head(url))
    }
}

impl Fs {
    /// Restrict to the subtree below `dir`; a direction the directory is not inside of is dropped.
    pub fn under(&self, dir: &str) -> Fs {
        let abs = crate::caps_core::scope_match::canonical_abs(dir, &std::env::current_dir().unwrap_or_default());
        let pat = vec![format!("{}/**", abs.display())];
        let mut out = Grant::sandbox(self.0.root());
        let mut first_err = None;
        let mut any = false;
        for k in [EffectKind::FsRead, EffectKind::FsWrite] {
            if self.0.scope(&k).is_none() {
                continue;
            }
            match self.0.derive_scope(&k, Some(&pat)) {
                Ok(d) => {
                    out.absorb(d);
                    any = true;
                }
                Err(e) => first_err = first_err.or(Some(e)),
            }
        }
        if !any {
            fault(first_err.expect("a value always has a scope"));
        }
        Fs(out)
    }
    pub fn read_only(&self) -> Fs {
        let mut g = self.0.clone();
        g.drop_scope(&EffectKind::FsWrite);
        Fs(g)
    }
    pub fn read_to_string(&self, path: &str) -> Result<String, String> {
        through(&self.0, EffectKind::FsRead, Some(path), || fs::read_to_string(path))
    }
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        through(&self.0, EffectKind::FsRead, Some(path), || fs::read(path))
    }
    pub fn read_dir(&self, path: &str) -> Result<Vec<fs::DirEntry>, String> {
        through(&self.0, EffectKind::FsRead, Some(path), || fs::read_dir(path))
    }
    pub fn exists(&self, path: &str) -> bool {
        through(&self.0, EffectKind::FsRead, Some(path), || fs::exists(path))
    }
    pub fn write(&self, path: &str, contents: &str) -> Result<(), String> {
        through(&self.0, EffectKind::FsWrite, Some(path), || fs::write(path, contents))
    }
    pub fn create_dir_all(&self, path: &str) -> Result<(), String> {
        through(&self.0, EffectKind::FsWrite, Some(path), || fs::create_dir_all(Path::new(path)))
    }
    pub fn remove_file(&self, path: &str) -> Result<(), String> {
        through(&self.0, EffectKind::FsWrite, Some(path), || fs::remove_file(path))
    }
}

impl Env {
    pub fn only(&self, names: Vec<String>) -> Env {
        Env(narrow(&self.0, EffectKind::EnvRead, &names))
    }
    pub fn var(&self, name: &str) -> Result<String, String> {
        through(&self.0, EffectKind::EnvRead, Some(name), || env::var(name))
    }
}

impl Process {
    pub fn only(&self, programs: Vec<String>) -> Process {
        Process(narrow(&self.0, EffectKind::Process, &programs))
    }
    pub fn run(&self, program: &str, args: &[String]) -> Result<String, String> {
        through(&self.0, EffectKind::Process, Some(program), || process::run(program, args))
    }
}

impl Clock {
    pub fn now_millis(&self) -> i64 {
        through(&self.0, EffectKind::Time, None, time::now_millis)
    }
}

impl Rng {
    pub fn range(&self, lo: i64, hi: i64) -> i64 {
        through(&self.0, EffectKind::Random, None, || random::int_range(lo, hi))
    }
}
