//! Anonymizer property tests over the verified corpus: no user-declared name survives in any
//! file, path or string, and the anonymized program still behaves identically.

#[path = "common/reduce_fixtures.rs"]
mod fx;

use std::collections::HashSet;
use std::path::PathBuf;
use windjammer::reduce::anonymize::{apply, declared_words, plan};
use windjammer::reduce::Project;

fn corpus() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/verified");
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "wj"))
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read_to_string(e.path()).unwrap(),
            )
        })
        .collect();
    v.sort();
    assert!(v.len() > 100, "corpus present");
    v
}

fn words(text: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut cur = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_alphanumeric() || c == '_' {
            cur.push(c);
        } else if !cur.is_empty() {
            out.insert(std::mem::take(&mut cur));
        }
    }
    out
}

/// Words that occur right after `.` or `::` in `text`.
fn member_words(text: &str) -> HashSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = HashSet::new();
    for i in 0..chars.len() {
        if chars[i] == '.' || (chars[i] == ':' && i > 0 && chars[i - 1] == ':') {
            let w: String = chars[i + 1..]
                .iter()
                .take_while(|c| c.is_alphanumeric() || **c == '_')
                .collect();
            if !w.is_empty() {
                out.insert(w);
            }
        }
    }
    out
}

/// Names a source declares as items (`fn x`, `struct X`, ...): the std API surface.
fn declared_items(text: &str) -> HashSet<String> {
    let flat: String = text
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let ws: Vec<&str> = flat.split_whitespace().collect();
    ws.windows(2)
        .filter(|w| ["fn", "struct", "enum", "trait", "type", "mod"].contains(&w[0]))
        .map(|w| w[1].to_string())
        .collect()
}

/// Words the language and std legitimately own: keywords plus every word of `std/*.wj`.
fn language_words() -> HashSet<String> {
    let mut set: HashSet<String> = windjammer::lexer::keywords::KEYWORDS
        .iter()
        .map(|k| k.word.to_string())
        .collect();
    fn walk(dir: &std::path::Path, set: &mut HashSet<String>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, set);
            } else if p.extension().is_some_and(|x| x == "wj") {
                set.extend(declared_items(&std::fs::read_to_string(p).unwrap()));
            }
        }
    }
    walk(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("std"),
        &mut set,
    );
    set
}

#[test]
fn no_user_declared_name_survives_anonymization_over_the_verified_corpus() {
    let lang = language_words();
    let mut checked = 0usize;
    let mut survivors_allowed: HashSet<String> = HashSet::new();
    for (name, text) in corpus() {
        let mut p = Project::default();
        p.files.insert("main.wj".into(), text.clone());
        // a word that is also used as a std member (`.rev()`, `x::len`) legitimately survives there
        let members: HashSet<String> = member_words(&text);
        let declared: Vec<String> = declared_words(&p)
            .into_iter()
            .filter(|w| !w.starts_with(|c: char| c.is_ascii_digit()))
            .collect();
        let pl = plan(&p);
        let q = apply(&p, &pl, &vec![true; pl.items.len()]);
        let out_words: HashSet<String> = q
            .files
            .iter()
            .flat_map(|(k, v)| words(k).into_iter().chain(words(v)))
            .collect();
        for w in &declared {
            // operator / formatting / iteration protocol methods are the language's names
            let protocol = [
                "to_string",
                "fmt",
                "eq",
                "ne",
                "cmp",
                "partial_cmp",
                "add",
                "sub",
                "mul",
                "div",
                "rem",
                "neg",
                "not",
                "index",
                "next",
                "drop",
                "clone",
                "default",
                "from",
                "into",
                "hash",
                "iter",
                "into_iter",
            ];
            if protocol.contains(&w.as_str())
                || members.contains(w)
                || lang.contains(w)
                || [
                    "main", "self", "Self", "_", "int", "string", "float", "bool", "str", "uint",
                    "Vec", "String", "Option", "Result", "HashMap", "Some", "None", "Ok", "Err",
                ]
                .contains(&w.as_str())
            {
                survivors_allowed.insert(w.clone());
                continue;
            }
            assert!(
                !out_words.contains(w),
                "{name}: declared name `{w}` survived anonymization"
            );
            checked += 1;
        }
        // comments never survive
        for line in text.lines().filter(|l| l.trim_start().starts_with("//")) {
            let body = line.trim_start().trim_start_matches('/').trim();
            if body.len() > 12 {
                assert!(
                    !q.files["main.wj"].contains(body),
                    "{name}: comment text survived: {body}"
                );
            }
        }
    }
    assert!(
        checked > 500,
        "property exercised on a real corpus ({checked} names)"
    );
}

#[test]
fn anonymized_programs_behave_like_the_originals() {
    // `// expect:` lines are an oracle independent of the anonymizer
    let mut ran = 0;
    for (name, text) in corpus().into_iter().take(60) {
        let expect: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("// expect:"))
            .map(|s| s.strip_prefix(' ').unwrap_or(s))
            .collect();
        if expect.is_empty() || text.contains("// exit:") {
            continue;
        }
        let mut p = Project::default();
        p.files.insert("main.wj".into(), text.clone());
        let pl = plan(&p);
        // identifiers only: string literals are deliberately replaced, so printed text would differ
        let q = apply(&p, &pl, &pl.words_only());
        let dir = tempfile::tempdir().unwrap();
        q.write_to(dir.path()).unwrap();
        let out = std::process::Command::new(fx::wj())
            .arg("run")
            .arg("--interpret")
            .arg(dir.path().join("main.wj"))
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let got: Vec<&str> = stdout.lines().collect();
        assert_eq!(
            got,
            expect,
            "{name}: anonymized program output differs\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        ran += 1;
    }
    assert!(ran >= 20, "ran {ran}");
}
