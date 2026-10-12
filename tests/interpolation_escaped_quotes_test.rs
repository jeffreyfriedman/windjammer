//! `"${f(\"x\")}"`: an interpolation whose expression contains escaped quotes. The lexer
//! unescapes `\"` inside the expression; the formatter tokenizer must skip it the same way.

use std::process::Command;

#[test]
fn escaped_quotes_inside_interpolation() {
    let dir = tempfile::TempDir::new().unwrap();
    let f = dir.path().join("p.wj");
    std::fs::write(&f, "fn shout(s: string) -> string { s.to_uppercase() }\nfn main() {\n    println(\"${shout(\\\"hi\\\")}!\")\n}\n").unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(&f).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "HI!\n", "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn formatter_keeps_escaped_quotes_inside_interpolation() {
    use windjammer::formatter::{format_source, FormatOptions};
    let src = "fn main() {\n    println(\"${shout(\\\"hi\\\")}!\")\n}\n";
    let out = format_source(src, &FormatOptions::default()).unwrap();
    assert_eq!(out, src);
}
