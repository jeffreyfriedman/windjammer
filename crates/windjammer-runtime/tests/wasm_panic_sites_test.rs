//! std calls that misbehave on wasm32 (plan item C8). On `wasm32-unknown-unknown` they compile and then
//! either PANIC or trap when reached (`Instant::now`, `SystemTime::now`, `env::temp_dir`, `thread::spawn`,
//! `thread::sleep`, `process::exit`, `process::id`, chrono's `Utc::now`/`Local::now`) or return an
//! `Unsupported` error that an `unwrap` turns into a panic (`std::fs`, `Command`, sockets in `std::net`).
//! Nothing but a browser run shows it, so this gate makes each such site a decision.
//!
//! `tests/wasm_panic_sites.txt` lists, per file under `src/`, how many ungated sites it has
//! (`FILE PANIC UNSUPPORTED`); the test fails when a count grows, a file gains a first site, or a count shrinks
//! without the list being lowered (shrink-only). A site is counted once per line and class. It is not counted when
//!   - `target_arch` and `wasm32` appear on the same line or one of the two lines before it (a cfg or `cfg!`),
//!   - the file is under `src/platform/native/` (not compiled on wasm32) or `src/platform/wasm/` (the wasm32
//!     implementation itself; `src/platform/mod.rs`),
//!   - the file is a module `lib.rs` gates behind a cargo feature (`http`, `time`, `cli`, ...): the engine builds
//!     the runtime with `default-features = false`, so those modules are not part of it,
//!   - the line is a comment, or follows a `#[cfg(test)]` line (inline test modules are last in the file).
//!
//! A new site belongs behind a port (a game engine's clock and file-system ports), behind that
//! cfg with a wasm32 alternative, or in a module the browser never reaches; only then may a count be raised.
//! The wasm32 build itself is gated by the engine's own repo (`cargo check --target wasm32-unknown-unknown
//! --no-default-features`, scripts/check_wasm_runtime.wj).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Calls that panic or trap on wasm32-unknown-unknown.
const PANIC_TOKENS: &[&str] = &[
    "Instant::now(",
    "SystemTime",
    "temp_dir(",
    "thread::spawn(",
    "thread::sleep(",
    "std::process::exit(",
    "std::process::id(",
    "Utc::now(",
    "Local::now(",
];

/// Calls that return an `Unsupported` error on wasm32-unknown-unknown.
const UNSUPPORTED_TOKENS: &[&str] = &["std::fs", "fs::", "Command::new(", "TcpListener", "TcpStream", "UdpSocket", "std::net"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// True when `tok` occurs in `line` not preceded by an identifier character (so `wasm_fs::` is not `fs::`).
fn has_token(line: &str, tok: &str) -> bool {
    let b = line.as_bytes();
    line.match_indices(tok).any(|(i, _)| i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
}

/// (panicking sites, unsupported-error sites) in one file's text.
fn ungated_sites(text: &str) -> (usize, usize) {
    let lines: Vec<&str> = text.lines().collect();
    let gated = |l: &str| l.contains("target_arch") && l.contains("wasm32");
    let (mut panics, mut errs) = (0, 0);
    for i in 0..lines.len() {
        let l = lines[i].trim_start();
        if l.starts_with("#[cfg(test)]") {
            break;
        }
        if l.starts_with("//") || (i.saturating_sub(2)..=i).any(|j| gated(lines[j])) {
            continue;
        }
        panics += PANIC_TOKENS.iter().any(|t| has_token(l, t)) as usize;
        errs += UNSUPPORTED_TOKENS.iter().any(|t| has_token(l, t)) as usize;
    }
    (panics, errs)
}

/// Modules `src/lib.rs` declares behind `#[cfg(feature = "..")]` (`foo` for `foo.rs` or `foo/`).
fn feature_gated_modules(lib_rs: &str) -> Vec<String> {
    let lines: Vec<&str> = lib_rs.lines().map(str::trim).collect();
    (1..lines.len())
        .filter(|&i| lines[i - 1].starts_with("#[cfg(feature"))
        .filter_map(|i| lines[i].strip_prefix("pub mod ")?.strip_suffix(';').map(str::to_string))
        .collect()
}

fn measured() -> BTreeMap<String, (usize, usize)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gated_mods = feature_gated_modules(&fs::read_to_string(root.join("src/lib.rs")).unwrap());
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files
        .iter()
        .filter_map(|p| {
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            if rel.starts_with("src/platform/native/") || rel.starts_with("src/platform/wasm/") {
                return None;
            }
            let top = rel.trim_start_matches("src/").split('/').next().unwrap().trim_end_matches(".rs");
            if gated_mods.iter().any(|m| m == top) {
                return None;
            }
            let n = ungated_sites(&fs::read_to_string(p).ok()?);
            (n != (0, 0)).then_some((rel, n))
        })
        .collect()
}

fn recorded() -> BTreeMap<String, (usize, usize)> {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/wasm_panic_sites.txt")).unwrap();
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut it = l.split_whitespace();
            let f = it.next().unwrap().to_string();
            let mut num = || it.next().unwrap().parse().unwrap();
            (f, (num(), num()))
        })
        .collect()
}

#[test]
fn wasm_panic_sites_in_src_match_the_recorded_list() {
    let (now, then) = (measured(), recorded());
    if std::env::var("WASM_PANIC_SITES_PRINT").is_ok() {
        for (f, n) in &now {
            println!("{f} {} {}", n.0, n.1);
        }
    }
    let mut problems = Vec::new();
    for (f, &(p, e)) in &now {
        let (rp, re) = then.get(f).copied().unwrap_or((0, 0));
        if p > rp {
            problems.push(format!("{f}: {p} wasm32-panicking sites, {rp} recorded; gate it or use a port (see the module docs)"));
        }
        if e > re {
            problems.push(format!("{f}: {e} unsupported-on-wasm32 sites, {re} recorded; gate it or use a port"));
        }
        if (p < rp || e < re) && p <= rp && e <= re {
            problems.push(format!("{f}: now {p} {e}, recorded {rp} {re}; lower the line in tests/wasm_panic_sites.txt"));
        }
    }
    for f in then.keys().filter(|f| !now.contains_key(*f)) {
        problems.push(format!("{f}: no sites any more; delete its line from tests/wasm_panic_sites.txt"));
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_counting_rule_gates_cfg_comments_test_tails_and_identifier_prefixes() {
    assert_eq!(ungated_sites("fn f() { let t = std::time::Instant::now(); }"), (1, 0));
    assert_eq!(ungated_sites("fn f() { std::thread::spawn(g); }"), (1, 0));
    assert_eq!(ungated_sites("fn f() { std::process::exit(1) }"), (1, 0));
    assert_eq!(ungated_sites("fn f() { std::fs::read(p); }"), (0, 1));
    assert_eq!(ungated_sites("fn f() { let l = TcpListener::bind(a); }"), (0, 1));
    assert_eq!(ungated_sites("fn f() { Command::new(p); std::env::temp_dir(); }"), (1, 1));
    assert_eq!(ungated_sites("#[cfg(not(target_arch = \"wasm32\"))]\nlet t = SystemTime::now();"), (0, 0));
    assert_eq!(ungated_sites("let d = if cfg!(target_arch = \"wasm32\") { 0 } else { Instant::now() };"), (0, 0));
    assert_eq!(ungated_sites("// Instant::now() in a comment"), (0, 0));
    assert_eq!(ungated_sites("fn a() {}\n#[cfg(test)]\nmod t { fn f() { std::fs::read(p); } }"), (0, 0));
    assert_eq!(ungated_sites("let x = wasm_fs::read(p);"), (0, 0));
}

#[test]
fn feature_gated_modules_are_read_from_lib_rs() {
    let lib = "pub mod a;\n#[cfg(feature = \"net\")]\npub mod http;\n#[cfg(feature = \"datetime\")]\npub mod time;\npub mod fs;";
    assert_eq!(feature_gated_modules(lib), vec!["http".to_string(), "time".to_string()]);
}
