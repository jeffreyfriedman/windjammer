//! A numeric-inference conflict inside a `${..}` interpolation names the column the user reads in the source, not the
//! interpolation key column (`source_map::INTERP_COLUMN_BASE + ..`, millions) the solver keys the expression by.

use assert_cmd::Command;

#[test]
fn conflict_in_interpolation_reports_the_source_column() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("t.wj");
    std::fs::write(&f, "fn main() {\n    let sorted: u64 = 7\n    let wide: u32 = sorted\n    println(\"${wide}\")\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).arg("build").arg(&f).arg("--output").arg(dir.path().join("o")).arg("--no-cargo").output().unwrap();
    let text = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Type conflict: must be U32"), "the conflict is reported:\n{text}");
    for line in text.lines().filter(|l| l.contains("Type conflict")) {
        let loc = line.split(": Type conflict").next().unwrap();
        let mut parts = loc.rsplit(':');
        let col: usize = parts.next().unwrap().trim().parse().unwrap();
        let ln: usize = parts.next().unwrap().trim().parse().unwrap();
        assert!(col < 1000 && (1..=5).contains(&ln), "a sane line:column, got {ln}:{col} in {line}");
    }
    assert!(!text.contains("panicked"), "no panic:\n{text}");
}

/// Build `src` and return the (line, column) of the first diagnostic mentioning `needle`.
fn first_position(src: &str, needle: &str) -> (usize, usize) {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("t.wj");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).arg("build").arg(&f).arg("--output").arg(dir.path().join("o")).arg("--no-cargo").output().unwrap();
    let text = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains("panicked"), "no panic:\n{text}");
    let line = text.lines().find(|l| l.contains(needle)).unwrap_or_else(|| panic!("no `{needle}` in:\n{text}"));
    let at = line.find(needle).and_then(|n| line[..n].rfind("t.wj:")).unwrap_or_else(|| panic!("no position in {line}")) + 5;
    let mut it = line[at..].split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse::<usize>().unwrap());
    (it.next().unwrap(), it.next().unwrap())
}

#[test]
fn pipe_error_inside_interpolation_is_at_the_true_position() {
    // `|>` with a non-function right side; the stage `b` is at line 3, column 23 (after a multibyte char and an escape)
    let src = "fn main() {\n    let st = [1]\n    println(\"é\\t${st.len|>1}\")\n}\n";
    let (l, c) = first_position(src, "right-hand side");
    // the right-hand side `1` of `|>`: after a multibyte char and an escape, on line 3
    assert_eq!((l, c), (3, 27));
}

#[test]
fn conflict_position_is_exact_in_a_second_interpolation() {
    let src = "fn main() {\n    let v: u64 = 1\n    let wide: u32 = v\n    println(\"a ${wide} b ${wide}\")\n}\n";
    let (l, c) = first_position(src, "identifier wide type");
    assert_eq!((l, c), (4, 19), "first `wide` inside the string");
}

#[test]
fn unknown_function_inside_interpolation_is_at_the_true_position() {
    let src = "fn main() {\n    let n = 1\n    println(\"é ${n} ${nosuchfn(n)}\")\n}\n";
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("t.wj");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wj")).arg("check").arg(&f).output().unwrap();
    let text = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains("panicked"), "no panic:\n{text}");
    assert!(text.contains("nosuchfn"), "reported:\n{text}");
    let l = text.lines().find(|l| l.trim_start().starts_with("-->")).unwrap_or_else(|| panic!("no position in:\n{text}"));
    let at = l.rfind("t.wj:").unwrap() + 5;
    let nums: Vec<usize> = l[at..].split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).take(2).map(|s| s.parse().unwrap()).collect();
    assert_eq!(nums, vec![3, 23], "the `n` of `nosuchfn` after a multibyte char and an earlier an earlier interpolation: {l}");
}
