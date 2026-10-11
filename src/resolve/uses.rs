//! Normalisation of parser `use` items into one [`UseDecl`] per bound name.

use crate::source_map::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// `use a::b` / `use a::b as c`: binds this name.
    Named(String),
    /// `use a::*`
    Glob,
}

/// One import edge: `segments` is the target path; `binding` says what it introduces.
#[derive(Debug, Clone)]
pub struct UseDecl {
    pub segments: Vec<String>,
    pub binding: Binding,
    pub is_pub: bool,
    pub location: Option<Location>,
}

/// Expand a parser `use` (`path`, `alias`) into normalised declarations.
pub fn expand_use(
    path: &[String],
    alias: &Option<String>,
    is_pub: bool,
    location: &Option<Location>,
) -> Vec<UseDecl> {
    let mk = |segments: Vec<String>, alias: Option<&String>| {
        make_decl(segments, alias, is_pub, location)
    };
    if let [single] = path {
        if let Some((prefix, names)) = split_braced(single) {
            return names
                .into_iter()
                .map(|n| {
                    let mut segs = split_path(&prefix);
                    segs.push(n);
                    mk(segs, None)
                })
                .collect();
        }
    }
    let segs = normalize_relative(path);
    vec![mk(segs, alias.as_ref())]
}

fn make_decl(
    mut segments: Vec<String>,
    alias: Option<&String>,
    is_pub: bool,
    location: &Option<Location>,
) -> UseDecl {
    let binding = if segments.last().map(String::as_str) == Some("*") {
        segments.pop();
        Binding::Glob
    } else {
        if segments.last().map(String::as_str) == Some("self") && segments.len() > 1 {
            segments.pop();
        }
        let name = alias.cloned().or_else(|| segments.last().cloned());
        Binding::Named(name.unwrap_or_default())
    };
    UseDecl {
        segments,
        binding,
        is_pub,
        location: location.clone(),
    }
}

fn split_path(p: &str) -> Vec<String> {
    p.split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// `"a::b::{C, D}"` -> (`"a::b"`, `["C", "D"]`).
fn split_braced(s: &str) -> Option<(String, Vec<String>)> {
    let (prefix, rest) = s.split_once("::{")?;
    let inner = rest.strip_suffix('}')?;
    let names = inner
        .split(',')
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .collect();
    Some((prefix.to_string(), names))
}

/// `./m` -> `self::m`; `../m` -> `super::m`.
fn normalize_relative(path: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, seg) in path.iter().enumerate() {
        if i == 0 {
            if let Some(rest) = seg.strip_prefix("./") {
                out.push("self".to_string());
                out.push(rest.to_string());
                continue;
            }
            let ups = seg.matches("../").count();
            if ups > 0 {
                for _ in 0..ups {
                    out.push("super".to_string());
                }
                let rest = seg.trim_start_matches("../");
                if !rest.is_empty() {
                    out.push(rest.to_string());
                }
                continue;
            }
        }
        out.push(seg.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn braced_expands_to_one_decl_per_name() {
        let d = expand_use(&s(&["a::b::{C, D}"]), &None, false, &None);
        assert_eq!(d.len(), 2);
        assert_eq!(d[1].segments, s(&["a", "b", "D"]));
        assert_eq!(d[1].binding, Binding::Named("D".into()));
    }

    #[test]
    fn glob_and_alias_and_relative() {
        let g = expand_use(&s(&["a", "*"]), &None, false, &None);
        assert_eq!(g[0].binding, Binding::Glob);
        assert_eq!(g[0].segments, s(&["a"]));
        let a = expand_use(&s(&["a", "b"]), &Some("c".into()), true, &None);
        assert_eq!(a[0].binding, Binding::Named("c".into()));
        let r = expand_use(&s(&["../m", "x"]), &None, false, &None);
        assert_eq!(r[0].segments, s(&["super", "m", "x"]));
    }
}
