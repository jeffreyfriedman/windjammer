//! `wj test` doctests (ADR-0018 feature 6, spec area DOC).
//!
//! Every ```` ```wj ```` / ```` ```windjammer ```` fence in a `///` or `//!` doc comment of a `.wj` file is a test.
//! The fence runs with its module in scope: the program is the module source (its own `fn main`, if any,
//! renamed away) followed by the fence (wrapped in `fn main` when it is a statement fragment). `// expect:` lines
//! are the expected stdout; without them the example must run without panicking. Annotations are the
//! docs-truth ones (`ignore`, `should_fail[=CODE]`, `check`).
//! Interpreter by default; `compiled` also builds and runs each doctest through the Rust backend
//! (`wj run`), and both outputs must match the expectation.

use super::check::check_with_expects;
use super::fence::{expects, fences, parse_info, rustdoc_as_markdown, Fence};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DocTest {
    pub file: PathBuf,
    /// 1-based line of the opening fence in the `.wj` file.
    pub line: usize,
    /// The item the doc comment documents (`fn parse`), for reports.
    pub item: String,
    pub fence: Fence,
}

#[derive(Debug)]
pub struct DocResult {
    pub test: DocTest,
    /// `None` = ignored.
    pub outcome: Option<Result<(), String>>,
}

impl DocTest {
    pub fn name(&self) -> String {
        format!("{}:{} ({})", self.file.display(), self.line, self.item)
    }
}

/// `.wj` files under `root` (or `root` itself), skipping build output and hidden directories.
fn wj_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if !name.starts_with('.')
                    && !matches!(name.as_str(), "target" | "build" | "node_modules")
                {
                    stack.push(p);
                }
            } else if name.ends_with(".wj") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The documented item: the first non-doc, non-attribute line after the fence's comment block.
fn documented_item(lines: &[&str], fence_line: usize) -> String {
    lines
        .iter()
        .skip(fence_line)
        .map(|l| l.trim())
        .find(|l| {
            !l.is_empty() && !l.starts_with("///") && !l.starts_with("//!") && !l.starts_with('@')
        })
        .map(|l| {
            l.split(['(', '{', '<'])
                .next()
                .unwrap_or(l)
                .trim()
                .to_string()
        })
        .unwrap_or_else(|| "module".to_string())
}

/// Doctests of one source text.
pub fn doctests_in(file: &Path, src: &str) -> Vec<DocTest> {
    let lines: Vec<&str> = src.lines().collect();
    fences(&rustdoc_as_markdown(src))
        .into_iter()
        .filter(|f| parse_info(&f.info).wj)
        .map(|fence| DocTest {
            file: file.to_path_buf(),
            line: fence.line,
            item: documented_item(&lines, fence.line),
            fence,
        })
        .collect()
}

pub fn discover(root: &Path) -> Vec<(DocTest, String)> {
    let mut out = Vec::new();
    for f in wj_files(root) {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        for t in doctests_in(&f, &src) {
            out.push((t, src.clone()));
        }
    }
    out
}

/// The program a doctest runs: module source with `fn main` renamed, then the fence.
pub fn compose(module_src: &str, fence_body: &str) -> String {
    let module = module_src.replace("fn main(", "fn __wj_doctest_module_main(");
    let has_main = fence_body
        .lines()
        .any(|l| l.trim_start().starts_with("fn main("));
    if has_main {
        format!("{module}\n{fence_body}\n")
    } else {
        format!("{module}\nfn main() {{\n{fence_body}\n}}\n")
    }
}

/// Build and run `program` through the Rust backend with `wj run` (`wj_bin`), returning stdout.
fn run_compiled(program: &str, wj_bin: &Path) -> Result<String, String> {
    let dir = std::env::temp_dir().join(format!("wj-doctest-{}-{}", std::process::id(), unique()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let main = dir.join("main.wj");
    std::fs::write(&main, program).map_err(|e| e.to_string())?;
    let out = std::process::Command::new(wj_bin)
        .arg("run")
        .arg(&main)
        .current_dir(&dir)
        .output()
        .map_err(|e| e.to_string());
    let _ = std::fs::remove_dir_all(&dir);
    let out = out?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "compiled run failed: {}",
            err.lines()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    // `wj run` announces itself on stdout: drop exactly that banner line.
    let banner = format!("Running {} (target: rust)", main.display());
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text.lines().filter(|l| *l != banner).map(|l| format!("{l}\n")).collect())
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

/// Run one doctest. `compiled`: also through the Rust backend with this `wj` binary.
pub fn run_one(
    t: &DocTest,
    module_src: &str,
    compiled: Option<&Path>,
) -> Option<Result<(), String>> {
    let mut a = parse_info(&t.fence.info);
    if a.ignore {
        return None;
    }
    let program = compose(module_src, &t.fence.body);
    let want = expects(&t.fence.body);
    if a.should_fail.is_none() {
        a.run = true; // doctests always execute (docs-truth fences only parse unless they have expectations)
    }
    let interpreted =
        check_with_expects(&program, &want, &a).map_err(|e| format!("interpreter: {e}"));
    if interpreted.is_err() || a.should_fail.is_some() {
        return Some(interpreted);
    }
    if let Some(bin) = compiled {
        let got = match run_compiled(&program, bin) {
            Ok(out) => out,
            Err(e) => return Some(Err(e)),
        };
        let got: Vec<String> = got.lines().map(|l| l.trim_end().to_string()).collect();
        if !want.is_empty() && got != want {
            return Some(Err(format!(
                "compiled: output {got:?} != expected {want:?}"
            )));
        }
    }
    Some(Ok(()))
}

pub fn run_doctests(root: &Path, filter: Option<&str>, compiled: Option<&Path>) -> Vec<DocResult> {
    discover(root)
        .into_iter()
        .filter(|(t, _)| filter.is_none_or(|f| t.name().contains(f)))
        .map(|(t, src)| {
            let outcome = run_one(&t, &src, compiled);
            DocResult { test: t, outcome }
        })
        .collect()
}

/// Print one line per doctest and a summary; returns the number of failures.
pub fn report(results: &[DocResult]) -> usize {
    let mut failed = 0;
    for r in results {
        match &r.outcome {
            None => println!("doctest {} ... ignored", r.test.name()),
            Some(Ok(())) => println!("doctest {} ... ok", r.test.name()),
            Some(Err(e)) => {
                failed += 1;
                println!(
                    "doctest {} ... FAILED\n    {}",
                    r.test.name(),
                    e.replace('\n', "\n    ")
                );
            }
        }
    }
    let ran = results.iter().filter(|r| r.outcome.is_some()).count();
    println!(
        "doctests: {} passed, {} failed, {} ignored",
        ran - failed,
        failed,
        results.len() - ran
    );
    failed
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "/// Doubles.\n///\n/// ```wj\n/// println(\"${double(21)}\")\n/// // expect: 42\n/// ```\nfn double(x: int) -> int {\n    x * 2\n}\n\nfn main() {\n    println(\"module main\")\n}\n";

    #[test]
    fn finds_the_fence_and_its_item() {
        let ts = doctests_in(Path::new("m.wj"), SRC);
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].line, 3);
        assert_eq!(ts[0].item, "fn double");
        assert_eq!(expects(&ts[0].fence.body), vec!["42"]);
    }

    #[test]
    fn composed_program_renames_the_module_main() {
        let p = compose(SRC, "println(\"x\")");
        assert!(p.contains("fn __wj_doctest_module_main("));
        assert_eq!(p.matches("fn main(").count(), 1);
    }

    #[test]
    fn passing_and_failing_expectations() {
        let t = &doctests_in(Path::new("m.wj"), SRC)[0];
        assert_eq!(run_one(t, SRC, None), Some(Ok(())));
        let wrong = SRC.replace("expect: 42", "expect: 43");
        let t = &doctests_in(Path::new("m.wj"), &wrong)[0];
        assert!(matches!(run_one(t, &wrong, None), Some(Err(e)) if e.contains("43")));
    }
}
