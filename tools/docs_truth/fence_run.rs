//! Executing a single fence: parse/analyze/lint/interpret Windjammer, or run a shell script in a sandbox.

use std::path::Path;

pub use windjammer::doctest::check::check_wj;
pub use windjammer::doctest::fence::expects;

/// Run a `sh,verify` fence: lines starting `$ ` are commands (run with `sh -c` in a fresh temp dir
/// whose PATH starts with the directory of `wj_bin`); other lines are expected output lines, each
/// of which must appear, in order, in the combined output of the preceding command. A command with a
/// `<<'EOF'` here-document continues until its terminator line (so fences can create their input files).
pub fn run_sh(body: &str, wj_bin: &Path) -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = format!("{}:{}", wj_bin.parent().unwrap().display(), std::env::var("PATH").unwrap_or_default());
    let mut cmds: Vec<(String, Vec<String>)> = Vec::new();
    let mut heredoc: Option<String> = None; // terminator of an open `<<'EOF'` here-document
    for l in body.lines() {
        if let Some(end) = &heredoc {
            let last = cmds.last_mut().expect("heredoc belongs to a command");
            last.0.push('\n');
            last.0.push_str(l);
            if l.trim() == end {
                heredoc = None;
            }
        } else if let Some(c) = l.trim_start().strip_prefix("$ ") {
            heredoc = c.split_once("<<").map(|(_, r)| r.trim().trim_matches(['\'', '"', '-']).split_whitespace().next().unwrap_or("EOF").trim_matches(['\'', '"']).to_string());
            cmds.push((c.to_string(), vec![]));
        } else if let Some(last) = cmds.last_mut() {
            if !l.trim().is_empty() {
                last.1.push(l.trim_end().to_string());
            }
        }
    }
    for (cmd, expected) in cmds {
        let out = std::process::Command::new("sh").arg("-c").arg(&cmd).current_dir(dir.path()).env("PATH", &path).output().map_err(|e| e.to_string())?;
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        if !out.status.success() {
            return Err(format!("`{cmd}` exited {:?}: {}", out.status.code(), text.trim()));
        }
        let mut from = 0;
        for e in expected {
            match text[from..].find(e.trim()) {
                Some(i) => from += i + e.trim().len(),
                None => return Err(format!("`{cmd}`: expected line {e:?} not found in output {:?}", text.trim())),
            }
        }
    }
    Ok(())
}
