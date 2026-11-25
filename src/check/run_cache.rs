//! The check before an interpreted run, cached by content (`wj run` of a program that already passed).
//!
//! `wj run` checks the program exactly as `wj check` does before interpreting it, and that check is nearly all
//! of a small program's start-up. Its outcome is a function of: the compiler build (by content, [`compiler_identity`]),
//! the stdlib and runtime trees baked into it, every input of the check (`inputs`: the reachable `.wj` sources with
//! their paths, the manifest, lock file, dependency sources and Rust crate surfaces), the flags that change what is
//! reported (`--no-lint`, the error format), where it runs from (diagnostic paths are printed relative to the working
//! directory, and `.wj.meta` sidecars are found from it), how the entry is spelled, and the `WJ_*` / `WINDJAMMER_*`
//! environment. The key hashes all of that; the `.wj.meta` sidecars the numeric inference may read are digested
//! after the check and verified on every load (`ambient.rs`). Only a clean outcome is stored, with the exact text it
//! printed (width warnings), so a hit prints the same bytes; an outcome with errors always runs the check again.
//! A build of unknown origin (no content key) caches nothing, and anything unexpected is a miss.
//!
//! The same store keeps facts the interpreter computes about one file with the compiler's analysis (the solver's
//! literal widths, `interpreter::compiled_facts`): keyed by the file's name, text and renames in the same context,
//! with the sidecars its `use`s can reach verified on load ([`fact_key`], [`encode_fact`], [`decode`]).

use super::ambient::{self, part};
use sha2::{Digest, Sha256};
use std::path::Path;

const FORMAT: u32 = 2;

/// Identity of this compiler build by content, the one place the run cache asks for it.
fn compiler_identity() -> Option<String> {
    super::shared_cache::compiler_key()
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Entry {
    format: u32,
    /// `ambient::use_depth` of the checked sources and the sidecar digest at that depth.
    use_depth: ambient::UseScope,
    ambient: String,
    /// What the clean check printed on stderr (for a file fact: the fact's text).
    stderr: String,
}

/// The key of the check of `path` with these flags, or `None` when this run must not be cached.
pub fn key(path: &Path, lints_block: bool, format: &str) -> Option<String> {
    let (_, keyed, _) = super::inputs(path, false).ok()?;
    let mut h = context("wj-run-check")?;
    part(&mut h, &[lints_block as u8]);
    part(&mut h, format.as_bytes());
    part(&mut h, path.to_string_lossy().as_bytes());
    part(&mut h, &(keyed.len() as u64).to_le_bytes());
    for (f, t) in &keyed {
        part(&mut h, f.as_bytes());
        part(&mut h, t.as_bytes());
    }
    Some(format!("{:x}", h.finalize()))
}

/// A hasher over what every cached result here depends on besides its own inputs: the entry kind and format, the
/// compiler build by content, the baked stdlib/runtime trees, the working directory and the `WJ_*` /
/// `WINDJAMMER_*` environment. `None` for a build of unknown origin.
fn context(kind: &str) -> Option<Sha256> {
    let compiler = compiler_identity()?;
    let cwd = super::ambient::cwd().ok()?;
    let mut h = Sha256::new();
    part(&mut h, format!("{kind}\n{FORMAT}").as_bytes());
    part(&mut h, compiler.as_bytes());
    part(&mut h, crate::embedded_assets::scan_inputs_fingerprint().as_bytes());
    part(&mut h, cwd.to_string_lossy().as_bytes());
    let env = crate::debug_env::snapshot_with_prefixes(&["WJ_", "WINDJAMMER_"]);
    part(&mut h, &(env.len() as u64).to_le_bytes());
    for (k, v) in &env {
        part(&mut h, k.as_bytes());
        part(&mut h, v.as_bytes());
    }
    Some(h)
}

/// The key of a fact about one parsed file (`file_key`: the interpreter's key of the file's name, text and
/// renames, `interpreter::compiled_facts`), with the [`context`] it is computed in.
pub fn fact_key(file_key: &str) -> Option<String> {
    let mut h = context("wj-file-fact")?;
    part(&mut h, file_key.as_bytes());
    Some(format!("{:x}", h.finalize()))
}

/// The entry for a fact about `program`: its text and the `.wj.meta` sidecars its `use`s can reach (`None`: they
/// cannot be digested, so it is not stored).
pub fn encode_fact(program: &crate::parser::Program<'static>, text: &str) -> Option<String> {
    let use_depth = ambient::use_depth([program]);
    let ambient = ambient::fingerprint(&use_depth)?;
    serde_json::to_string(&Entry { format: FORMAT, use_depth, ambient, stderr: text.to_string() }).ok()
}

/// The stderr text of a stored clean check (`text`, the entry [`encode`] made), when its sidecars are unchanged.
pub fn decode(text: &str) -> Option<String> {
    let e: Entry = serde_json::from_str(text).ok()?;
    (e.format == FORMAT && ambient::fingerprint(&e.use_depth).as_deref() == Some(e.ambient.as_str())).then_some(e.stderr)
}

/// The entry for a clean check of the program whose sources are `sources` that printed `stderr` (`None`: the
/// sidecars cannot be digested, so it is not stored). The store itself is the driver's (`cli::check`).
pub fn encode(sources: &[(String, String)], stderr: &str) -> Option<String> {
    let mut sink = Vec::new();
    let parsed: Vec<super::front::Parsed> = sources.iter().filter_map(|(f, t)| super::front::parse(f, t, &mut sink)).collect();
    let use_depth = ambient::use_depth(parsed.iter().map(|p| &p.program));
    let ambient = ambient::fingerprint(&use_depth)?;
    serde_json::to_string(&Entry { format: FORMAT, use_depth, ambient, stderr: stderr.to_string() }).ok()
}
