//! Which types a generated crate declares with `#[derive(Serialize)]`: the contract another crate's derive inference
//! needs (a struct holding a dependency's type may derive `Serialize` only when that type implements it). Read from the
//! generated Rust itself, the declarations the dependency publishes.

use std::path::Path;

/// `(type name, derives Serialize)` for every `struct` / `enum` item declared in `src`.
pub fn type_serde_derives_in_source(src: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut pending_serialize = false;
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with("#[") {
            if t.starts_with("#[derive(") && t.contains("Serialize") {
                pending_serialize = true;
            }
            continue;
        }
        let item = t.strip_prefix("pub(crate) ").or_else(|| t.strip_prefix("pub ")).unwrap_or(t);
        let name = item.strip_prefix("struct ").or_else(|| item.strip_prefix("enum ")).map(|rest| {
            rest.split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("").to_string()
        });
        match name {
            Some(n) if !n.is_empty() => out.push((n, pending_serialize)),
            _ => {}
        }
        if !t.is_empty() && !t.starts_with("//") {
            pending_serialize = false;
        }
    }
    out
}

/// [`type_serde_derives_in_source`] over every `.rs` file under `dir` (sorted, recursive).
pub fn type_serde_derives_in_dir(dir: &Path) -> Vec<(String, bool)> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        crate::metadata::scan_stats::dir_listed();
        let Ok(entries) = std::fs::read_dir(&d) else { continue };
        for p in entries.flatten().map(|e| e.path()) {
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "rs") {
                files.push(p);
            }
        }
    }
    files.sort();
    files.iter().filter_map(|f| std::fs::read_to_string(f).ok()).flat_map(|s| type_serde_derives_in_source(&s)).collect()
}

/// The names among `derives` that derive `Serialize`, sorted and deduplicated (the `metadata.json` field).
pub fn serde_type_names(derives: &[(String, bool)]) -> Vec<String> {
    let mut names: Vec<String> = derives.iter().filter(|(_, s)| *s).map(|(n, _)| n.clone()).collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_derives_of_structs_and_enums() {
        let src = "#[derive(Debug, Clone, Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct A {\n}\n\
                   #[derive(Debug, Clone)]\n#[repr(C)]\npub struct B {\n}\nenum C { X }\n#[derive(Serialize)]\npub enum D { Y }\n";
        assert_eq!(
            type_serde_derives_in_source(src),
            vec![("A".into(), true), ("B".into(), false), ("C".into(), false), ("D".into(), true)]
        );
    }
}
