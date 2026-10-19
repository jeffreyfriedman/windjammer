//! File-level passes: comment stripping and dropping whole files (together with the `use`
//! declarations that import them).

use super::ddmin::ddmin;
use super::project::Project;
use super::session::Session;
use super::toks::{groups, render, scan, units, without_ranges};
use crate::formatter::TokKind;

pub fn strip_comments(s: &mut Session) -> bool {
    let strip = |text: &str| -> Option<String> {
        let toks = scan(text)?;
        if !toks.iter().any(|t| t.kind == TokKind::Comment) {
            return None;
        }
        let kept: Vec<_> = toks
            .into_iter()
            .filter(|t| t.kind != TokKind::Comment)
            .collect();
        Some(render(&kept))
    };
    let mut all = s.project.clone();
    let mut any = false;
    for (k, v) in s.project.files.iter() {
        if let Some(t) = strip(v) {
            all.files.insert(k.clone(), t);
            any = true;
        }
    }
    if !any {
        return false;
    }
    if s.adopt_first(vec![all]).is_some() {
        return true;
    }
    let mut changed = false;
    for file in s.file_order() {
        if let Some(t) = s.project.files.get(&file).and_then(|v| strip(v)) {
            let c = s.project.with_file(&file, t);
            changed |= s.adopt_first(vec![c]).is_some();
        }
    }
    changed
}

/// Module name a file defines: its stem, or the directory name for `mod.wj`.
fn module_stem(rel: &str) -> String {
    let p = std::path::Path::new(rel);
    let stem = p.file_stem().and_then(|x| x.to_str()).unwrap_or("");
    if stem == "mod" {
        p.parent()
            .and_then(|d| d.file_name())
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_string()
    } else {
        stem.to_string()
    }
}

/// `p` without `drop`, and without top-level `use`/`mod` units elsewhere that mention a dropped module.
fn drop_with_imports(p: &Project, drop: &[String]) -> Project {
    let stems: Vec<String> = drop
        .iter()
        .map(|d| module_stem(d))
        .filter(|x| !x.is_empty())
        .collect();
    let mut out = p.clone();
    for d in drop {
        out.files.remove(d);
    }
    for (k, v) in p.files.iter().filter(|(k, _)| !drop.contains(k)) {
        let Some(toks) = scan(v) else { continue };
        let root = groups(&toks).into_iter().next().expect("root group");
        let dead: Vec<_> = units(&toks, &root)
            .into_iter()
            .filter(|u| {
                matches!(toks[u.start].text.as_str(), "use" | "mod" | "pub")
                    && toks[u.clone()]
                        .iter()
                        .any(|t| t.kind == TokKind::Word && stems.contains(&t.text))
                    && toks[u.clone()]
                        .iter()
                        .any(|t| matches!(t.text.as_str(), "use" | "mod"))
            })
            .collect();
        if !dead.is_empty() {
            out.files
                .insert(k.clone(), render(&without_ranges(&toks, &dead)));
        }
    }
    out
}

pub fn drop_files(s: &mut Session) -> bool {
    let names = s.file_order();
    if names.len() < 2 {
        return false;
    }
    let before = s.project.files.len();
    let build = |p: &Project, removed: &[usize]| {
        let drop: Vec<String> = removed.iter().map(|&i| names[i].clone()).collect();
        drop_with_imports(p, &drop)
    };
    ddmin(s, names.len(), &build);
    s.project.files.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_a_module_removes_its_imports() {
        let mut p = Project::default();
        p.files
            .insert("main.wj".into(), "use util\nfn main() {\n}\n".into());
        p.files.insert("util.wj".into(), "pub fn f() {\n}\n".into());
        let q = drop_with_imports(&p, &["util.wj".to_string()]);
        assert_eq!(q.files.len(), 1);
        assert!(!q.files["main.wj"].contains("util"));
        assert!(q.files["main.wj"].contains("fn main"));
    }

    #[test]
    fn module_stem_uses_directory_for_mod_files() {
        assert_eq!(module_stem("a/b.wj"), "b");
        assert_eq!(module_stem("a/mod.wj"), "a");
    }
}
