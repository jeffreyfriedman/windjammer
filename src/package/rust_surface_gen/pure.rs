//! Which generated functions are `@pure`: the curated list in `pure_crates.toml`, nothing else.
//!
//! A crate is pure only when it is listed for the exact version being generated and its source does not
//! mention an I/O module. Effects are never inferred from names; an unlisted crate's functions have unknown
//! authority and stay `ffi(<crate>)` (ADR-0026).

use std::collections::BTreeMap;

const LIST: &str = include_str!("pure_crates.toml");

/// Source text that rules a crate out even when it is listed.
const IO_MARKERS: &[&str] = &["std::fs", "std::net", "std::process", "std::env", "std::time", "extern \"C\"", "libc::"];

/// Version of the curated list (recorded in every generated header).
pub fn list_version() -> i64 {
    toml::from_str::<toml::Value>(LIST).ok().and_then(|v| v.get("version")?.as_integer()).unwrap_or(0)
}

/// The verdict for `crate_name` at `version` whose sources are `files`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pure,
    /// Not listed (or not for this version): unknown authority.
    Unlisted,
    /// Listed, but the source contains an I/O marker (the text says which and where).
    Refused(String),
}

pub fn verdict(crate_name: &str, version: &str, files: &BTreeMap<String, String>) -> Verdict {
    verdict_in(LIST, crate_name, version, files)
}

fn verdict_in(list: &str, crate_name: &str, version: &str, files: &BTreeMap<String, String>) -> Verdict {
    let Ok(doc) = toml::from_str::<toml::Value>(list) else { return Verdict::Unlisted };
    let Some(entry) = doc.get("crates").and_then(|c| c.get(crate_name.replace('_', "-")).or_else(|| c.get(crate_name))) else {
        return Verdict::Unlisted;
    };
    let req = entry.get("versions").and_then(|v| v.as_str()).and_then(|r| semver::VersionReq::parse(r).ok());
    let ver = semver::Version::parse(version).ok();
    match (req, ver) {
        (Some(r), Some(v)) if r.matches(&v) => {}
        _ => return Verdict::Unlisted,
    }
    for (file, text) in files {
        for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
            if let Some(m) = IO_MARKERS.iter().find(|m| line.contains(*m)) {
                return Verdict::Refused(format!("`{m}` appears in {file}"));
            }
        }
    }
    Verdict::Pure
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(text: &str) -> BTreeMap<String, String> {
        [("src/lib.rs".to_string(), text.to_string())].into_iter().collect()
    }

    #[test]
    fn listed_versions_only() {
        let clean = files("pub fn f(a: &str) -> usize { a.len() }\n");
        assert_eq!(verdict("strsim", "0.11.1", &clean), Verdict::Pure);
        assert_eq!(verdict("strsim", "0.12.0", &clean), Verdict::Unlisted);
        assert_eq!(verdict("reqwest", "0.12.0", &clean), Verdict::Unlisted);
        // names never decide: a crate called `pure_math` is unlisted
        assert_eq!(verdict("pure_math", "1.0.0", &clean), Verdict::Unlisted);
        assert!(list_version() >= 1);
    }

    #[test]
    fn io_in_the_source_refuses_a_listed_crate() {
        let io = files("use std::fs::read;\n// std::net in a comment is fine\n");
        assert!(matches!(verdict("strsim", "0.11.1", &io), Verdict::Refused(w) if w.contains("std::fs")));
        let comment_only = files("// std::net::TcpStream is not used\n");
        assert_eq!(verdict("strsim", "0.11.1", &comment_only), Verdict::Pure);
    }

    #[test]
    fn dashed_names_match() {
        let list = "version = 1\n[crates.unicode-width]\nversions = \"0.1\"\nreviewed = \"x\"\n";
        assert_eq!(verdict_in(list, "unicode_width", "0.1.5", &BTreeMap::new()), Verdict::Pure);
    }
}
