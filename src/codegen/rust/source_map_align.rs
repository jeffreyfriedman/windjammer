//! `.wj` -> `.rs` line map built by aligning the Windjammer source with the Rust it produced.
//!
//! The generator emits strings, so it does not know which Rust line a statement lands on. This
//! module recovers the correspondence from the two texts: functions are paired by name and
//! occurrence order, then the statements of each body are paired level by level. A level is paired
//! only when both sides have the same number of statements with the same keywords (`let`, `if`,
//! `for`, ...); an uncertain level maps nothing but its opener, so a lookup lands on the enclosing
//! function or block rather than on a wrong statement.
//!
//! Consumers: `wj build --source-maps` writes the map as `<out>/<file>.rs.map`, and
//! `wj build --check` uses it to translate rustc errors back to `.wj` lines.

use crate::source_map::SourceMap;
use std::collections::BTreeMap;
use std::path::Path;

/// Build the map for one file. `rust_file` is the path the map is keyed by (as the error mapper
/// looks it up); lines are 1-based on both sides.
pub fn build_source_map(
    wj_file: &Path,
    wj_src: &str,
    rust_file: &Path,
    rust_src: &str,
) -> SourceMap {
    let mut map = SourceMap::new();
    if let Ok(cwd) = std::env::current_dir() {
        map.set_workspace_root(cwd);
    }
    let wj: Vec<&str> = wj_src.lines().collect();
    let rs: Vec<&str> = rust_src.lines().collect();
    let wj_fns = fn_lines(&wj);
    let rs_fns = fn_lines(&rs);
    for (name, wj_decls) in &wj_fns {
        let Some(rs_decls) = rs_fns.get(name) else {
            continue;
        };
        if wj_decls.len() != rs_decls.len() {
            continue; // generated or dropped duplicates: do not guess
        }
        for (&w, &r) in wj_decls.iter().zip(rs_decls) {
            let mut pairs = vec![(w, r)];
            if let (Some(we), Some(re)) = (block_end(&wj, w), block_end(&rs, r)) {
                align(&wj, &rs, (w + 1, we), (r + 1, re), &mut pairs);
            }
            for (wl, rl) in pairs {
                map.add_mapping(rust_file, rl + 1, 0, wj_file, wl + 1, 0);
            }
        }
    }
    map
}

/// Write `<rust_file>.map` next to the generated Rust when source maps were requested
/// (`CompilerConfig::emit_source_maps`). A failed write is a warning, not a build failure.
pub fn write_if_enabled(
    enabled: bool,
    wj_file: &Path,
    wj_src: &str,
    rust_file: &Path,
    rust_src: &str,
) {
    if !enabled {
        return;
    }
    let map = build_source_map(wj_file, wj_src, rust_file, rust_src);
    if let Err(e) = map.save_to_file(rust_file.with_extension("rs.map")) {
        eprintln!("Warning: Failed to save source map: {}", e);
    }
}

/// `name -> 0-based lines` of every `fn name` declaration, in file order.
fn fn_lines(lines: &[&str]) -> BTreeMap<String, Vec<usize>> {
    let mut out: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if t.starts_with("//") {
            continue;
        }
        let mut words = t.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '('));
        while let Some(w) = words.next() {
            if w == "fn" {
                if let Some(name) = words.next() {
                    if !name.is_empty() && !name.starts_with('(') {
                        out.entry(name.to_string()).or_default().push(i);
                    }
                }
                break;
            }
            if !matches!(
                w,
                "pub" | "async" | "unsafe" | "extern" | "pub(crate)" | "pub(super)" | "const" | ""
            ) && !w.starts_with("pub(")
            {
                break;
            }
        }
    }
    out
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}

/// Line of the `}` closing the block opened on line `open` (first later line at the opener's
/// indent that starts with `}`), if the opener ends with `{`.
fn block_end(lines: &[&str], open: usize) -> Option<usize> {
    let opener = lines[open];
    if !opener.trim_end().ends_with('{') {
        return None;
    }
    let ind = indent(opener);
    (open + 1..lines.len())
        .find(|&j| indent(lines[j]) == ind && lines[j].trim_start().starts_with('}'))
}

/// Keyword class of a statement line, the same in both languages for the pairs we trust.
fn key(line: &str) -> &'static str {
    let t = line.trim_start();
    let w = t
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()
        .unwrap_or("");
    match w {
        "let" => "let",
        "if" => "if",
        "for" => "for",
        "while" => "while",
        "loop" => "loop",
        "match" => "match",
        "return" => "return",
        "break" => "break",
        "continue" => "continue",
        _ => "expr",
    }
}

/// Statement start lines directly inside `range` (half-open, 0-based): lines at the body indent
/// that are not closers, comments or blanks.
fn statements(lines: &[&str], (from, to): (usize, usize)) -> Vec<usize> {
    let Some(ind) = (from..to)
        .filter(|&i| !lines[i].trim().is_empty())
        .map(|i| indent(lines[i]))
        .min()
    else {
        return Vec::new();
    };
    (from..to)
        .filter(|&i| {
            let t = lines[i].trim_start();
            indent(lines[i]) == ind
                && !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with(['}', ')', ']', '.', '#'])
        })
        .collect()
}

fn align(
    wj: &[&str],
    rs: &[&str],
    wr: (usize, usize),
    rr: (usize, usize),
    out: &mut Vec<(usize, usize)>,
) {
    let (ws, rss) = (statements(wj, wr), statements(rs, rr));
    if ws.is_empty()
        || ws.len() != rss.len()
        || ws.iter().zip(&rss).any(|(&w, &r)| key(wj[w]) != key(rs[r]))
    {
        return;
    }
    for (&w, &r) in ws.iter().zip(&rss) {
        out.push((w, r));
        // Follow `{ ... }` / `} else {` chains pairwise.
        let (mut wo, mut ro) = (w, r);
        while let (Some(we), Some(re)) = (block_end(wj, wo), block_end(rs, ro)) {
            align(wj, rs, (wo + 1, we), (ro + 1, re), out);
            let (wl, rl) = (wj[we].trim_end(), rs[re].trim_end());
            if wl.ends_with('{') && rl.ends_with('{') {
                out.push((we, re));
                (wo, ro) = (we, re);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(wj: &str, rs: &str) -> SourceMap {
        build_source_map(Path::new("/p/m.wj"), wj, Path::new("/p/m.rs"), rs)
    }

    fn wj_line(m: &SourceMap, rust_line: usize) -> Option<usize> {
        m.lookup(Path::new("/p/m.rs"), rust_line).map(|x| x.wj_line)
    }

    const WJ: &str = "fn helper(a: int) -> int {\n    let b = a + 1\n    b\n}\n\nfn main() {\n    let x = helper(1)\n    if x > 1 {\n        println(\"big\")\n    } else {\n        println(\"small\")\n    }\n}\n";
    const RS: &str = "use std::io;\n\nfn helper(a: i64) -> i64 {\n    let b = a + 1;\n    b\n}\n\nfn main() {\n    let x = helper(1);\n    if x > 1 {\n        println!(\"big\");\n    } else {\n        println!(\"small\");\n    }\n}\n";

    #[test]
    fn maps_functions_and_nested_statements() {
        let m = map(WJ, RS);
        assert_eq!(wj_line(&m, 3), Some(1), "helper decl");
        assert_eq!(wj_line(&m, 4), Some(2), "let b");
        assert_eq!(wj_line(&m, 8), Some(6), "main decl");
        assert_eq!(wj_line(&m, 9), Some(7), "let x");
        assert_eq!(wj_line(&m, 10), Some(8), "if");
        assert_eq!(wj_line(&m, 11), Some(9), "println big");
        assert_eq!(wj_line(&m, 12), Some(10), "else");
        assert_eq!(wj_line(&m, 13), Some(11), "println small");
    }

    #[test]
    fn an_uncertain_body_maps_only_the_function_line() {
        // The Rust side has an extra generated statement, so statements are not paired.
        let rs = "fn main() {\n    let tmp = 1;\n    let x = 1;\n    println!(\"{}\", x);\n}\n";
        let wj = "fn main() {\n    let x = 1\n    println(\"${x}\")\n}\n";
        let m = map(wj, rs);
        assert_eq!(wj_line(&m, 1), Some(1));
        assert_eq!(wj_line(&m, 3), None);
    }

    #[test]
    fn duplicate_names_with_different_counts_are_skipped() {
        let wj = "fn new() {\n}\n";
        let rs = "fn new() {\n}\nfn new() {\n}\n";
        assert!(map(wj, rs).is_empty());
    }

    #[test]
    fn comments_and_non_fn_text_are_not_declarations() {
        let m = map(
            "// fn ghost() {\nfn real() {\n}\n",
            "// fn ghost() {\nfn real() {\n}\n",
        );
        assert_eq!(wj_line(&m, 2), Some(2));
        assert_eq!(wj_line(&m, 1), None);
    }
}
