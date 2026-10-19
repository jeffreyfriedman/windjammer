//! `"${f(\"x\")}"`: an interpolation whose expression contains escaped quotes. Today the lexer
//! ends the string at the inner `\"` ("unterminated string literal"); the fix belongs to the
//! lexer/parser stream. This test pins the expected behaviour (interpreter output) and is ignored
//! until that fix lands; remove the `#[ignore]` then.

use std::process::Command;

#[test]
#[ignore = "lexer: escaped quotes inside `${..}` (parser stream owns the fix)"]
fn escaped_quotes_inside_interpolation() {
    let dir = tempfile::TempDir::new().unwrap();
    let f = dir.path().join("p.wj");
    std::fs::write(&f, "fn shout(s: string) -> string { s.to_uppercase() }\nfn main() {\n    println(\"${shout(\\\"hi\\\")}!\")\n}\n").unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(&f).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "HI!\n", "{}", String::from_utf8_lossy(&o.stderr));
}
