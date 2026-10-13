//! Metamorphic gates through the CLI: `wj fmt` and `wj fix` must preserve behavior.
//!
//! - fmt: for every program P (interpreter corpus at two seeds, verified examples):
//!   run(P) == run(fmt(P)), fmt(fmt(P)) == fmt(P), and `wj fmt --check` accepts fmt(P).
//! - fix: P is "un-fixed" into the deprecated surface (`println!`/`format!` with `{}` templates,
//!   `vec![..]` initialisers), then `wj fix` must give a program with run(fix(unfix(P))) == run(P)
//!   and `wj fix --check` must accept the result.
//! The oracle is the interpreter run of the untouched P (`wj run --interpret`); programs the
//! interpreter cannot run are skipped (counted, must stay a minority).
//! Known failures: tests/fmt_fix_known.txt (`fmt|fix <file>`), a ratchet: unlisted failures and
//! listed passes both fail.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corpus() -> Vec<(String, String)> {
    let mut v = Vec::new();
    let mut add = |dir: &str, seeds: &[&str]| {
        let mut files: Vec<_> = std::fs::read_dir(root().join(dir)).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")).collect();
        files.sort();
        for f in files {
            let t = std::fs::read_to_string(&f).unwrap();
            let rel = f.strip_prefix(root()).unwrap().to_string_lossy().into_owned();
            for s in seeds {
                let name = if seeds.len() > 1 { format!("{rel}@{s}") } else { rel.clone() };
                v.push((name, t.replace("__SEED__", s)));
            }
        }
    };
    add("tests/interpreter_corpus", &["3", "11"]);
    add("examples/verified", &["1"]);
    v
}

struct Out {
    ok: bool,
    stdout: String,
}

fn wj(args: &[&str], file: Option<&Path>, stdin: Option<&str>) -> Out {
    let mut c = Command::new(WJ);
    c.args(args).env("NO_COLOR", "1").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(f) = file {
        c.arg(f);
    }
    let mut child = c.spawn().unwrap();
    {
        use std::io::Write;
        let mut si = child.stdin.take().unwrap();
        if let Some(s) = stdin {
            let _ = si.write_all(s.as_bytes());
        }
    }
    let start = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > std::time::Duration::from_secs(30) {
            let _ = child.kill();
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let o = child.wait_with_output().unwrap();
    Out { ok: o.status.success(), stdout: String::from_utf8_lossy(&o.stdout).into_owned() }
}

fn run_src(dir: &Path, name: &str, src: &str) -> Option<String> {
    let f = dir.join(name);
    std::fs::write(&f, src).unwrap();
    let o = wj(&["run", "--interpret"], Some(&f), None);
    o.ok.then_some(o.stdout)
}

/// Deprecated surface for P: `println("a ${x}")` -> `println!("a {}", x)` (identifier
/// interpolations only), `let v = [..]` -> `let v = vec![..]`. Lines it cannot rewrite stay as is.
pub fn unfix(src: &str) -> String {
    let mut out = String::new();
    for line in src.lines() {
        let mut l = line.to_string();
        // Only the free function `println(..)`, not methods such as `stdout.println(..)`.
        let free = l.find("println(\"").filter(|&i| !l[..i].ends_with(|c: char| c == '.' || c == '_' || c.is_alphanumeric()));
        if let Some(i) = free {
            let rest = &l[i + 9..];
            if let Some(end) = rest.find("\")") {
                let body = &rest[..end];
                let mut tmpl = String::new();
                let mut args = Vec::new();
                let mut ok = true;
                let mut chars = body.char_indices().peekable();
                while let Some((j, c)) = chars.next() {
                    if c == '$' && body[j..].starts_with("${") {
                        let close = body[j..].find('}').map(|k| j + k);
                        match close {
                            Some(k) if body[j + 2..k].chars().all(|c| c.is_alphanumeric() || c == '_') && k > j + 2 => {
                                args.push(body[j + 2..k].to_string());
                                tmpl.push_str("{}");
                                while chars.peek().is_some_and(|(p, _)| *p <= k) {
                                    chars.next();
                                }
                            }
                            _ => ok = false,
                        }
                    } else if c == '{' || c == '}' {
                        // A literal brace in a Rust format string is doubled.
                        tmpl.push(c);
                        tmpl.push(c);
                    } else {
                        tmpl.push(c);
                    }
                }
                if ok && !args.is_empty() {
                    let new = format!("println!(\"{tmpl}\", {})", args.join(", "));
                    l = format!("{}{}{}", &l[..i], new, &rest[end + 2..]);
                } else if ok && !body.contains('$') {
                    l = format!("{}println!(\"{}\"){}", &l[..i], tmpl, &rest[end + 2..]);
                }
            }
        }
        let t = l.trim_start();
        if t.starts_with("let ") && l.contains(" = [") && l.trim_end().ends_with(']') && !l.contains(';') {
            l = l.replacen(" = [", " = vec![", 1);
        }
        out.push_str(&l);
        out.push('\n');
    }
    out
}

fn known() -> Vec<String> {
    std::fs::read_to_string(root().join("tests/fmt_fix_known.txt")).unwrap_or_default().lines().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.split(" | ").next().unwrap().trim().to_string()).collect()
}

#[test]
fn fmt_and_fix_preserve_behavior_through_the_cli() {
    let progs = corpus();
    assert!(progs.len() > 150, "corpus too small");
    let results = std::sync::Mutex::new((Vec::<String>::new(), 0usize, 0usize));
    std::thread::scope(|s| {
        for chunk in progs.chunks(progs.len().div_ceil(4)) {
            let results = &results;
            s.spawn(move || {
                for (name, src) in chunk {
                    let tmp = tempfile::TempDir::new().unwrap();
                    let d = tmp.path();
                    let Some(want) = run_src(d, "p.wj", src) else {
                        results.lock().unwrap().1 += 1;
                        continue;
                    };
                    let mut fails = Vec::new();
                    // fmt
                    let f1 = wj(&["fmt", "-"], None, Some(src));
                    if !f1.ok {
                        fails.push(format!("fmt {name}: wj fmt - failed on a program that runs"));
                    } else {
                        let f2 = wj(&["fmt", "-"], None, Some(&f1.stdout));
                        let ran = run_src(d, "f.wj", &f1.stdout);
                        let chk = wj(&["fmt", "--check"], Some(&d.join("f.wj")), None);
                        if ran.as_deref() != Some(want.as_str()) {
                            fails.push(format!("fmt {name}: formatted program behaves differently"));
                        } else if f2.stdout != f1.stdout {
                            fails.push(format!("fmt {name}: not idempotent"));
                        } else if !chk.ok {
                            fails.push(format!("fmt {name}: `wj fmt --check` rejects wj fmt output"));
                        }
                    }
                    // fix
                    let legacy = unfix(src);
                    if legacy != *src {
                        let lf = d.join("legacy.wj");
                        std::fs::write(&lf, &legacy).unwrap();
                        let fx = wj(&["fix"], Some(&lf), None);
                        let fixed = std::fs::read_to_string(&lf).unwrap();
                        let ran = run_src(d, "fixed.wj", &fixed);
                        let chk = wj(&["fix", "--check"], Some(&d.join("fixed.wj")), None);
                        if !fx.ok || ran.as_deref() != Some(want.as_str()) {
                            fails.push(format!("fix {name}: fix(unfix(P)) behaves differently from P"));
                        } else if !chk.ok {
                            fails.push(format!("fix {name}: `wj fix --check` wants to change wj fix output"));
                        }
                        results.lock().unwrap().2 += 1;
                    }
                    results.lock().unwrap().0.extend(fails);
                }
            });
        }
    });
    let (fails, skipped, fix_cases) = results.into_inner().unwrap();
    eprintln!("fmt/fix metamorphic: {} programs, {skipped} not runnable in the interpreter, {fix_cases} fix cases", progs.len());
    assert!(skipped * 3 < progs.len(), "most programs do not run: the oracle is vacuous");
    assert!(fix_cases > 50, "unfix produced only {fix_cases} legacy programs");
    let known = known();
    let keys: Vec<String> = fails.iter().map(|f| f.split(':').next().unwrap().to_string()).collect();
    let new: Vec<_> = fails.iter().zip(&keys).filter(|(_, k)| !known.contains(k)).map(|(f, _)| f.clone()).collect();
    let stale: Vec<_> = known.iter().filter(|k| !keys.contains(k)).collect();
    if !root().join("tests/fmt_fix_known.txt").exists() && std::env::var("FMT_FIX_BLESS").is_ok() {
        let mut body = String::from("# fmt/fix metamorphic failures (tests/fmt_fix_semantics_test.rs). `<fmt|fix> <program> | message`. Only shrinks.\n");
        for f in &fails {
            let (k, msg) = f.split_once(": ").unwrap();
            body.push_str(&format!("{k} | {msg}\n"));
        }
        std::fs::write(root().join("tests/fmt_fix_known.txt"), body).unwrap();
        return;
    }
    assert!(new.is_empty(), "behavior changed by fmt/fix:\n{}", new.join("\n"));
    assert!(stale.is_empty(), "now passing, remove from tests/fmt_fix_known.txt: {stale:?}");
}

#[test]
fn selftest_unfix_produces_the_deprecated_surface() {
    let p = "fn main() {\n    let v = [1, 2]\n    let n = 3\n    println(\"n=${n} v\")\n    println(\"plain\")\n}\n";
    let u = unfix(p);
    assert!(u.contains("let v = vec![1, 2]"), "{u}");
    assert!(u.contains("println!(\"n={} v\", n)"), "{u}");
    assert!(u.contains("println!(\"plain\")"), "{u}");
    assert_eq!(unfix("    stdout.println(\"x ${n}\")\n"), "    stdout.println(\"x ${n}\")\n", "methods are not macros");
    assert_eq!(unfix("let s = \"${a.b}\"\n"), "let s = \"${a.b}\"\n", "non-identifier interpolation is left alone");
    assert!(unfix("    println(\"lit {x} ${n}\")\n").contains("println!(\"lit {{x}} {}\", n)"), "literal braces are escaped");
}
