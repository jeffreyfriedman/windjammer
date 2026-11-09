//! Multi-file projects in the book: a `sh,verify` fence that writes a `wj.toml` and sources with `cat > PATH <<'EOF'`
//! heredocs and then runs `cd DIR && wj run` is also a project sample. `extract` writes its files under
//! `projects/ID/` with `.root` (DIR) and `.expect` (the lines the fence expects), and the runner
//! (`scripts/book/book_samples.wj`) runs it interpreted and compiled and requires the outputs to agree.

use super::{book, md, Ctx};
use windjammer::doctest::fence::parse_info;

pub struct Project {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub root: String,
    pub files: Vec<(String, String)>,
    pub expect: Vec<String>,
}

fn heredoc_target(cmd: &str) -> Option<(String, String)> {
    let rest = cmd.strip_prefix("cat > ")?;
    let (path, tail) = rest.split_once(" <<")?;
    let term = tail.trim().trim_matches(['\'', '"', '-']).to_string();
    Some((path.trim().to_string(), term))
}

/// Files written by heredocs, plus the `cd DIR && wj run` command (DIR and expected output), if the fence is a project.
pub fn parse(body: &str) -> Option<(Vec<(String, String)>, String, Vec<String>)> {
    let mut files: Vec<(String, String)> = Vec::new();
    let mut run: Option<(String, Vec<String>)> = None;
    let mut collecting = false;
    let mut lines = body.lines();
    while let Some(l) = lines.next() {
        if let Some(cmd) = l.trim_start().strip_prefix("$ ") {
            collecting = false;
            if let Some((path, term)) = heredoc_target(cmd) {
                let mut text = String::new();
                for h in lines.by_ref() {
                    if h.trim() == term {
                        break;
                    }
                    text.push_str(h);
                    text.push('\n');
                }
                files.push((path, text));
            } else if let Some(dir) = cmd.trim().strip_prefix("cd ").and_then(|c| c.strip_suffix(" && wj run")) {
                if run.is_none() {
                    run = Some((dir.trim().to_string(), vec![]));
                    collecting = true;
                }
            }
        } else if collecting && !l.trim().is_empty() {
            if let Some(r) = run.as_mut() {
                r.1.push(l.trim_end().to_string());
            }
        }
    }
    let (root, expect) = run?;
    files.iter().any(|(p, _)| *p == format!("{root}/wj.toml")).then_some((files, root, expect))
}

pub fn projects(ctx: &Ctx) -> Vec<Project> {
    let mut out = Vec::new();
    for (file, text) in book::pages(ctx) {
        for f in md::fences(&text) {
            let a = parse_info(&f.info);
            if !a.sh_verify || a.ignore {
                continue;
            }
            if let Some((files, root, expect)) = parse(&f.body) {
                let stem = file.trim_start_matches(&format!("{}/", book::SRC)).trim_end_matches(".md").replace('/', "_");
                out.push(Project { id: format!("{stem}_L{}", f.line), file: file.clone(), line: f.line, root, files, expect });
            }
        }
    }
    out
}

/// Write every project under `dir/projects/ID/`, and an `INDEX.tsv` mapping ids to `page.md:line`.
pub fn extract(ctx: &Ctx, dir: &std::path::Path) -> std::io::Result<usize> {
    let ps = projects(ctx);
    let mut index = String::new();
    for p in &ps {
        let base = dir.join("projects").join(&p.id);
        for (path, text) in &p.files {
            let full = base.join(path);
            std::fs::create_dir_all(full.parent().unwrap())?;
            std::fs::write(full, text)?;
        }
        std::fs::write(base.join(".root"), &p.root)?;
        std::fs::write(base.join(".expect"), p.expect.join("\n") + "\n")?;
        index.push_str(&format!("{}\t{}:{}\n", p.id, p.file, p.line));
    }
    std::fs::create_dir_all(dir.join("projects"))?;
    std::fs::write(dir.join("projects").join("INDEX.tsv"), index)?;
    Ok(ps.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fence_with_a_manifest_and_a_run_is_a_project() {
        let body = "$ mkdir -p p/src\n$ cat > p/wj.toml <<'EOF'\n[package]\nname = \"p\"\nEOF\n$ cat > p/src/main.wj <<'EOF'\nfn main() {\n}\nEOF\n$ cd p && wj run\nhello\n";
        let (files, root, expect) = parse(body).unwrap();
        assert_eq!((files.len(), root.as_str(), expect), (2, "p", vec!["hello".to_string()]));
        assert!(parse("$ cat > a.wj <<'EOF'\nx\nEOF\n$ wj run a.wj\n").is_none());
    }
}
