//! String literals inside `${..}` (`"${s.replace(",", ";")}"`, `"${"Hello".to_uppercase()}"`) lex, run and
//! format consistently: the compiler lexer and the formatter tokenizer share the same `${"` rule, and the
//! formatter must leave the nested literals untouched (it used to insert spaces: `${" Hello "...}`).

use windjammer::formatter::{format_source, FormatOptions};

const SRC: &str = "fn main() {\n    let s = \"a,b\"\n    println(\"${s.replace(\",\", \";\")}\")\n    println(\"${\"Hello\".to_uppercase()} ${s.len()}\")\n}\n";

#[test]
fn formatter_preserves_nested_string_literals() {
    let out = format_source(SRC, &FormatOptions::default()).unwrap();
    assert!(out.contains("${\"Hello\".to_uppercase()}"), "{out}");
    assert!(out.contains("${s.replace(\",\", \";\")}"), "{out}");
    assert_eq!(format_source(&out, &FormatOptions::default()).unwrap(), out, "idempotent");
}

#[test]
fn interpreter_runs_nested_quotes() {
    let dir = std::env::temp_dir().join(format!("wj-nestedq-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("t.wj");
    std::fs::write(&f, SRC).unwrap();
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(&f).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "a;b\nHELLO 3\n");
}
