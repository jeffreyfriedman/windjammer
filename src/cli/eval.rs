//! `wj eval '<expr>'` and `wj run -`: a program that is not a file yet.
//!
//! Both write the program to a scratch file and run it like any other: inside a project (`wj.toml` above the
//! current directory) under the project's `.wj-cache/eval/` so its modules, dependencies and capabilities
//! apply; elsewhere in a temporary directory. Never beside the sources. The scratch directory is removed after
//! the run.

use std::path::{Path, PathBuf};

/// The binding `wj eval` gives the final expression's value before printing it.
const VALUE: &str = "__wj_eval_value";

/// The standard modules an `eval` expression can use without importing them.
pub const PRELUDE: &[&str] = &[
    "use std::strings",
    "use std::math",
    "use std::json",
    "use std::time",
    "use std::fs",
    "use std::env",
    "use std::collections::HashMap",
    "use std::collections::HashSet",
];

/// A fresh scratch directory for one program (removed by [`remove`]).
pub fn scratch_dir(cwd: &Path) -> std::io::Result<PathBuf> {
    let base = match crate::module_system::project::find_manifest_dir(cwd) {
        Some(project) => project.join(".wj-cache").join("eval"),
        None => std::env::temp_dir().join("wj-eval"),
    };
    let dir = base.join(format!("{}-{}", std::process::id(), nanos()));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn nanos() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}

/// Write `source` as `main.wj` in a fresh scratch directory; the path of the file.
pub fn write_script(cwd: &Path, source: &str) -> std::io::Result<PathBuf> {
    let dir = scratch_dir(cwd)?;
    let file = dir.join("main.wj");
    std::fs::write(&file, source)?;
    Ok(file)
}

/// Remove the scratch directory of `file` (best effort).
pub fn remove(file: &Path) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// `wj run -`: when `path` is `-`, read the program from stdin into a scratch file and return its path.
pub fn stdin_script(path: Option<&Path>) -> std::io::Result<Option<PathBuf>> {
    if path.is_none_or(|p| p.as_os_str() != "-") {
        return Ok(None);
    }
    let mut text = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)?;
    write_script(&std::env::current_dir()?, &text).map(Some)
}

/// Exit with `status` when it is not 0.
pub fn exit_on_failure(status: i32) {
    if status != 0 {
        std::process::exit(status);
    }
}

/// `wj eval CODE`: run [`program`]`(code)` interpreted, without lint noise; the exit status.
pub fn run(code: &str) -> std::io::Result<i32> {
    use super::run_dev::{execute, Mode, RunOptions};
    let file = write_script(&std::env::current_dir()?, &program(code))?;
    let interp = crate::interpreter::runner::RunOptions { engine_resolved: true, ..Default::default() };
    let status = execute(&RunOptions {
        path: Some(file.clone()),
        bin: None,
        args: Vec::new(),
        mode: Mode::Interpret,
        target: "rust".into(),
        hot: false,
        interp,
        lint: None,
        no_lint: true,
    });
    remove(&file);
    Ok(status)
}

/// The program `wj eval` runs for `text`: the prelude, then `text` in `main`. Statements may be separated by
/// newlines or `;`. A final expression is printed (`${..}`); a final statement is not.
pub fn program(text: &str) -> String {
    let parts: Vec<&str> = text.split(['\n', ';']).map(str::trim).filter(|s| !s.is_empty()).collect();
    let mut body = String::new();
    for (i, p) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        if last && is_expression(p) {
            // the value is bound first and printed by name: the expression is never spliced into a string, so
            // one that is itself a string with `${..}` (or quotes) stays as written
            body.push_str(&format!("    let {VALUE} = {p}\n    println(\"${{{VALUE}}}\")\n"));
        } else {
            body.push_str(&format!("    {p}\n"));
        }
    }
    format!("{}\n\nfn main() {{\n{body}}}\n", PRELUDE.join("\n"))
}

/// A line that yields a value to print (not a binding, a loop, an assignment or a print).
fn is_expression(s: &str) -> bool {
    const STATEMENTS: &[&str] = &["let ", "for ", "while ", "loop ", "if ", "match ", "return", "print", "eprint", "use "];
    let assignment = s.split_once('=').is_some_and(|(l, r)| {
        !r.starts_with('=') && !l.ends_with(['!', '<', '>', '=']) && !l.contains('(') && !l.contains('"')
    });
    !STATEMENTS.iter().any(|k| s.starts_with(k)) && !s.ends_with('}') && !assignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_final_expression_is_printed_and_statements_are_kept() {
        let p = program("let x = 6; x * 7");
        assert!(p.contains("    let x = 6\n    let __wj_eval_value = x * 7\n    println(\"${__wj_eval_value}\")\n"), "{p}");
        let s = program("let mut n = 1\nn += 2\nprintln(\"${n}\")");
        assert!(s.ends_with("    let mut n = 1\n    n += 2\n    println(\"${n}\")\n}\n"), "{s}");
        assert!(program("x == 3").contains("let __wj_eval_value = x == 3\n"));
        // a string literal with `${..}` is bound as written, not spliced into another interpolation
        assert!(program("\"${6 * 7} ok\"").contains("let __wj_eval_value = \"${6 * 7} ok\"\n"));
    }
}
