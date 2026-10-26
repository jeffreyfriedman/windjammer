//! Static lint: iteration over `HashMap` / `HashSet` in code that can feed emitted output.
//! Iteration order of std hash containers is randomized per process, so any such loop whose
//! result reaches generated text is a nondeterminism bug. Text heuristic (no type checker):
//! names declared with a hash type in the same file (fields, `let`, params) and then iterated
//! (`for .. in [&[mut ]]name`, `.iter()`, `.keys()`, `.values()`, `.drain()`, `.into_iter()`).
//! Sorting right after (`sorted`, `.sort`) is not recognized: switch to BTreeMap/IndexMap instead.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const DIRS: [&str; 3] = ["src/codegen", "src/ir", "src/analyzer"];
const RATCHET: &str = "tests/determinism_hash_iteration.toml";

fn ident_before(s: &str) -> Option<String> {
    let id: String = s.chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<Vec<_>>().into_iter().rev().collect();
    (!id.is_empty()).then_some(id)
}

/// Names declared with a hash container type: `name: [&][mut ]HashMap<..`, `let [mut] name = HashSet::new()`.
fn hash_names(src: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in src.lines() {
        let Some(pos) = ["HashMap", "HashSet"].iter().filter_map(|t| line.find(t)).min() else { continue };
        let head = &line[..pos];
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("let ") {
            let rest = rest.trim_start_matches("mut ");
            let id: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !id.is_empty() {
                names.insert(id);
                continue;
            }
        }
        let head = head.trim_end().trim_end_matches("mut").trim_end().trim_end_matches('&').trim_end();
        if let Some(b) = head.strip_suffix(':') {
            if let Some(id) = ident_before(b.trim_end()) {
                names.insert(id);
            }
        }
    }
    names.retain(|n| n != "self" && n != "mut");
    names
}

/// Struct fields with a hash type (`    name: HashMap<..`, at field indentation).
fn field_names(src: &str) -> BTreeSet<String> {
    src.lines()
        .filter(|l| l.starts_with("    ") && !l.trim_start().starts_with("let ") && (l.contains(": HashMap<") || l.contains(": HashSet<")))
        .filter_map(|l| l.trim_start().trim_start_matches("pub(crate) ").trim_start_matches("pub ").split(':').next().map(|n| n.trim().to_string()))
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_') && n.len() > 3)
        .collect()
}

/// `(line number, line)` of every hash-container iteration (names declared in this file).
pub fn scan(src: &str) -> Vec<(usize, String)> {
    scan_with(src, &hash_names(src))
}

/// Same, with extra hash-typed names (struct fields declared in another file of the module).
pub fn scan_with(src: &str, extra: &BTreeSet<String>) -> Vec<(usize, String)> {
    let mut names = hash_names(src);
    names.extend(extra.iter().cloned());
    let mut hits = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let t = line.trim();
        if t.starts_with("//") {
            continue;
        }
        let words: BTreeSet<&str> = line.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect();
        let hit = names.iter().filter(|n| words.contains(n.as_str())).any(|n| {
            let iter_call = [".iter()", ".iter_mut()", ".keys()", ".values()", ".values_mut()", ".drain()", ".into_iter()"]
                .iter()
                .any(|m| line.contains(&format!(".{n}{m}")) || line.contains(&format!(" {n}{m}")) || line.contains(&format!("({n}{m}")) || line.contains(&format!("&{n}{m}")));
            let for_loop = t.starts_with("for ")
                && t.split(" in ").nth(1).is_some_and(|it| {
                    let it = it.split('{').next().unwrap_or("").trim().trim_start_matches('&').trim_start_matches("mut ").trim_start_matches("self.");
                    it == n || it.ends_with(&format!(".{n}"))
                });
            iter_call || for_loop
        });
        if hit {
            hits.push((i + 1, t.to_string()));
        }
    }
    hits
}

fn walk(d: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out)
        } else if p.extension().is_some_and(|x| x == "rs") && !p.to_string_lossy().contains("test") {
            out.push(p)
        }
    }
}

pub fn measure(root: &Path) -> BTreeMap<String, usize> {
    let mut files = Vec::new();
    for d in DIRS {
        walk(&root.join(d), &mut files);
    }
    // Fields are often declared in `mod.rs` and iterated in a sibling file: share `self.<field>` names, but only
    // within one directory. Sharing across the whole tree flagged every `Vec` that happens to be called `fields`,
    // `params` or `enums` in an unrelated module as soon as any file declared a hash container with that name.
    let mut by_dir: BTreeMap<std::path::PathBuf, BTreeSet<String>> = BTreeMap::new();
    for f in &files {
        by_dir.entry(f.parent().unwrap().to_path_buf()).or_default().extend(field_names(&std::fs::read_to_string(f).unwrap()));
    }
    let mut out = BTreeMap::new();
    for f in &files {
        let n = scan_with(&std::fs::read_to_string(f).unwrap(), &by_dir[f.parent().unwrap()]).len();
        if n > 0 {
            out.insert(f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), n);
        }
    }
    out
}

pub fn check_ratchet(root: &Path) {
    let got = measure(root);
    let text = std::fs::read_to_string(root.join(RATCHET)).unwrap_or_default();
    let allowed: BTreeMap<String, usize> = toml::from_str::<BTreeMap<String, BTreeMap<String, usize>>>(&text)
        .ok()
        .and_then(|mut t| t.remove("hash_iteration"))
        .unwrap_or_default();
    let grew: Vec<_> = got.iter().filter(|(f, n)| **n > allowed.get(*f).copied().unwrap_or(0)).collect();
    let shrank = allowed.iter().any(|(f, n)| got.get(f).copied().unwrap_or(0) < *n);
    let bootstrap = !root.join(RATCHET).exists();
    if (bootstrap || grew.is_empty() && shrank) && std::env::var("DETERMINISM_BLESS").is_ok() {
        let mut s = String::from("# Hash-container iteration sites in src/codegen, src/ir, src/analyzer (tests/codegen_determinism_test.rs).\n# Counts may only go down; tighten with DETERMINISM_BLESS=1.\n\n[hash_iteration]\n");
        for (f, n) in &got {
            s.push_str(&format!("{f:?} = {n}\n"));
        }
        std::fs::write(root.join(RATCHET), s).unwrap();
        return;
    }
    let detail: Vec<String> = grew
        .iter()
        .map(|(f, n)| {
            let sites = scan(&std::fs::read_to_string(root.join(f)).unwrap()).into_iter().map(|(l, t)| format!("    {f}:{l}: {t}")).collect::<Vec<_>>().join("\n");
            format!("{f}: {n} (allowed {}):\n{sites}", allowed.get(*f).copied().unwrap_or(0))
        })
        .collect();
    assert!(detail.is_empty(), "new HashMap/HashSet iteration in codegen paths (use BTreeMap/IndexMap or sort):\n{}", detail.join("\n"));
    assert!(!shrank, "hash-iteration counts went down: tighten with DETERMINISM_BLESS=1");
}
