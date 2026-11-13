//! Programs the compiled build rejects or fails must not pass interpreted (the compiled program is the oracle,
//! ADR-0038 ladder 2). Each case is a reduced repro from ecosystem work; the expected behaviour is the compiled one.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn run(name: &str, src: &str) -> (i32, String, String) {
    let dir = std::env::temp_dir().join(format!("wj-agree-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("main.wj");
    std::fs::write(&p, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).arg(&p).stdin(std::process::Stdio::null()).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into())
}

/// `text.chars().len()`: Rust's `Chars` has no `len` (E0599 in the compiled build), so the interpreted run fails
/// too, naming `.count()`; the same through a size-keeping adapter.
#[test]
fn len_of_chars_is_refused_like_the_compiled_build() {
    for (name, expr) in [("chars", "\"héllo\".chars().len()"), ("char_indices", "\"héllo\".char_indices().len()"), ("mapped", "\"abc\".chars().map(|c| c).len()")] {
        let (code, out, err) = run(name, &format!("fn main() {{\n    let n = {expr}\n    println(\"${{n}}\")\n}}\n"));
        assert_ne!(code, 0, "{name}: interpreted run passed: {out}");
        assert!(err.contains("no method named `len`") && err.contains(".count()"), "{name}: {err}");
    }
}

/// What the compiled build accepts still runs: `count()` on `Chars`, `len()` on exact-size iterators.
#[test]
fn exact_size_iterators_keep_len_and_chars_keeps_count() {
    let (code, out, err) = run(
        "ok",
        "fn main() {\n    let v = vec![1, 2, 3]\n    let a = \"héllo\".chars().count()\n    let b = v.iter().len()\n    let c = v.iter().map(|x| x + 1).len()\n    let d = \"héllo\".bytes().len()\n    println(\"${a} ${b} ${c} ${d}\")\n}\n",
    );
    assert_eq!((code, out.as_str()), (0, "5 3 3 6\n"), "{err}");
}
