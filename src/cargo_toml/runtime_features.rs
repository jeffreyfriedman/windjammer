//! Which `windjammer-runtime` feature groups a generated program needs.
//!
//! The runtime gates its heavy modules (`http` pulls tokio, axum and reqwest; `cli` pulls clap; ...)
//! behind cargo features. The module-to-feature map is read from the runtime's own `src/lib.rs`
//! (`#[cfg(feature = "net")] pub mod http;`), so the runtime is the single source of truth. A
//! program enables only the groups whose modules its generated Rust names; anything the scanner does
//! not understand enables everything (`full`), so a miss can cost compile time but never correctness.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

/// Module (or re-exported item) name -> the feature that gates it (`None` = always compiled).
#[derive(Debug, Default, Clone)]
pub(crate) struct FeatureMap {
    items: HashMap<String, Option<String>>,
    native: HashMap<String, Option<String>>,
}

fn single_feature(line: &str) -> Option<String> {
    let l = line.trim();
    let inner = l.strip_prefix("#[cfg(feature = \"")?.strip_suffix("\")]")?;
    (!inner.contains('"')).then(|| inner.to_string())
}

/// Parse `#[cfg(feature = "f")] pub mod m;` and `pub use m::{A, B};` lines.
fn parse_gates(src: &str) -> HashMap<String, Option<String>> {
    let mut out = HashMap::new();
    let mut pending: Option<String> = None;
    for line in src.lines() {
        let t = line.trim();
        if let Some(f) = single_feature(t) {
            pending = Some(f);
            continue;
        }
        if t.starts_with("#[") || t.starts_with("//") {
            continue;
        }
        if let Some(rest) = t.strip_prefix("pub mod ").and_then(|r| r.strip_suffix(';')) {
            out.insert(rest.trim().to_string(), pending.take());
        } else if let Some(rest) = t.strip_prefix("pub use ").and_then(|r| r.strip_suffix(';')) {
            let names = match rest.split_once("::") {
                Some((_, tail)) => tail.trim_matches(|c| c == '{' || c == '}').to_string(),
                None => rest.to_string(),
            };
            for n in names.split(',') {
                let n = n.trim();
                if !n.is_empty() && n != "*" {
                    out.entry(n.to_string()).or_insert_with(|| pending.clone());
                }
            }
            pending = None;
        } else {
            pending = None;
        }
    }
    out
}

impl FeatureMap {
    pub(crate) fn from_sources(lib_rs: &str, native_mod_rs: &str) -> Self {
        Self { items: parse_gates(lib_rs), native: parse_gates(native_mod_rs) }
    }

    /// Read the maps from a runtime crate directory; `None` when it cannot be read.
    pub(crate) fn load(runtime_dir: &Path) -> Option<Self> {
        let lib = std::fs::read_to_string(runtime_dir.join("src/lib.rs")).ok()?;
        let native = std::fs::read_to_string(runtime_dir.join("src/platform/native/mod.rs")).unwrap_or_default();
        Some(Self::from_sources(&lib, &native))
    }

    /// Features needed by one generated source file. `None` means "cannot tell: enable everything".
    pub(crate) fn features_for(&self, src: &str) -> Option<BTreeSet<String>> {
        let mut need = BTreeSet::new();
        let mut rest = src;
        while let Some(at) = rest.find("windjammer_runtime") {
            rest = &rest[at + "windjammer_runtime".len()..];
            let after = rest.strip_prefix("::")?;
            if let Some(list) = after.strip_prefix('{') {
                let end = list.find('}')?;
                for part in list[..end].split(',') {
                    let first = part.trim().split("::").next().unwrap_or("").trim();
                    self.add(first, part.trim(), &mut need)?;
                }
            } else {
                let mut segs = after.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'));
                let path = segs.next().unwrap_or("");
                let first = path.split("::").next().unwrap_or("");
                self.add(first, path, &mut need)?;
            }
        }
        Some(need)
    }

    fn add(&self, first: &str, path: &str, need: &mut BTreeSet<String>) -> Option<()> {
        if first == "platform" {
            let mut it = path.split("::");
            it.next();
            if it.next() != Some("native") {
                return None;
            }
            let module = it.next().filter(|m| !m.is_empty())?;
            if let Some(f) = self.native.get(module)?.clone() {
                need.insert(f);
            }
            return Some(());
        }
        if first.is_empty() || first == "self" {
            return None;
        }
        if let Some(f) = self.items.get(first)?.clone() {
            need.insert(f);
        }
        Some(())
    }
}

/// Feature list for a program: the union over `sources`, or `["full"]` when any file is unclear.
pub(crate) fn runtime_features<'a>(map: Option<&FeatureMap>, sources: impl Iterator<Item = &'a str>) -> Vec<String> {
    let Some(map) = map else { return vec!["full".into()] };
    let mut all = BTreeSet::new();
    for s in sources {
        match map.features_for(s) {
            Some(f) => all.extend(f),
            None => return vec!["full".into()],
        }
    }
    all.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIB: &str = "pub use rand;\n#[cfg(feature = \"net\")]\npub mod http;\npub mod json;\n#[cfg(feature = \"cli\")]\npub mod cli;\n#[cfg(feature = \"net\")]\npub use http::{HttpMethod, Router};\n";
    const NATIVE: &str = "#[cfg(feature = \"parallel\")]\npub mod compute;\npub mod fs;\n";

    fn map() -> FeatureMap {
        FeatureMap::from_sources(LIB, NATIVE)
    }

    fn feats(src: &str) -> Vec<String> {
        runtime_features(Some(&map()), std::iter::once(src))
    }

    #[test]
    fn ungated_modules_need_nothing() {
        assert!(feats("use windjammer_runtime::json::parse;").is_empty());
    }

    #[test]
    fn gated_module_enables_its_feature() {
        assert_eq!(feats("let r = windjammer_runtime::http::get(u);"), vec!["net"]);
        assert_eq!(feats("use windjammer_runtime::{json, cli::Args};"), vec!["cli"]);
    }

    #[test]
    fn root_reexports_follow_their_gate() {
        assert_eq!(feats("use windjammer_runtime::Router;"), vec!["net"]);
    }

    #[test]
    fn native_platform_modules_use_their_own_gates() {
        assert_eq!(feats("windjammer_runtime::platform::native::compute::join"), vec!["parallel"]);
        assert!(feats("windjammer_runtime::platform::native::fs::read").is_empty());
    }

    #[test]
    fn anything_unclear_enables_everything() {
        assert_eq!(feats("use windjammer_runtime::*;"), vec!["full"]);
        assert_eq!(feats("use windjammer_runtime::brand_new_module::x;"), vec!["full"]);
        assert_eq!(feats("extern crate windjammer_runtime;"), vec!["full"]);
        assert_eq!(runtime_features(None, std::iter::once("")), vec!["full"]);
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::super::toml_generation::write_cargo_toml;

    fn manifest_for(main_rs: &str) -> String {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("main.rs"), main_rs).unwrap();
        let section = "[[bin]]\nname = \"p\"\npath = \"main.rs\"\n";
        write_cargo_toml(t.path(), t.path(), section, &crate::compiler_config::CompilerConfig::default()).unwrap();
        std::fs::read_to_string(t.path().join("Cargo.toml")).unwrap()
    }

    /// The embedded runtime's real `lib.rs` is the oracle for which module needs which group.
    #[test]
    fn a_program_without_network_code_gets_no_heavy_groups() {
        let m = manifest_for("fn main() { windjammer_runtime::io::println(\"hi\"); }\n");
        assert!(m.contains("default-features = false }"), "{m}");
        assert!(!m.contains("\"net\"") && !m.contains("full"), "{m}");
    }

    #[test]
    fn http_use_enables_net_and_its_dependencies_come_with_it() {
        let m = manifest_for("fn main() { let _ = windjammer_runtime::http::get(\"u\"); }\n");
        assert!(m.contains("features = [\"net\"]"), "{m}");
    }

    #[test]
    fn unknown_runtime_usage_falls_back_to_full() {
        let m = manifest_for("use windjammer_runtime::*;\nfn main() {}\n");
        assert!(m.contains("features = [\"full\"]"), "{m}");
    }
}
