//! `SUPPORT.toml`: explicit, ratcheted list of known-unsupported (program, backend) pairs.

use super::backends::Backend;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Support {
    entries: BTreeMap<(String, String), String>,
}

impl Support {
    pub fn parse(text: &str) -> Support {
        let v: toml::Value = text.parse().expect("SUPPORT.toml must be valid TOML");
        let mut entries = BTreeMap::new();
        if let Some(arr) = v.get("unsupported").and_then(|a| a.as_array()) {
            for e in arr {
                let get = |k: &str| e.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
                let (program, backend, reason) = (get("program"), get("backend"), get("reason"));
                assert!(!reason.is_empty(), "SUPPORT.toml entry {program}/{backend} needs a reason");
                entries.insert((program, backend), reason);
            }
        }
        Support { entries }
    }

    pub fn load(path: &Path) -> Support {
        Support::parse(&std::fs::read_to_string(path).unwrap_or_default())
    }

    pub fn reason(&self, program: &str, b: Backend) -> Option<&str> {
        self.entries.get(&(program.to_string(), b.name().to_string())).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &(String, String)> {
        self.entries.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let s = Support::parse("[[unsupported]]\nprogram=\"p\"\nbackend=\"go\"\nreason=\"r\"\n");
        assert_eq!(s.reason("p", Backend::Go), Some("r"));
        assert_eq!(s.reason("p", Backend::Js), None);
    }
}
