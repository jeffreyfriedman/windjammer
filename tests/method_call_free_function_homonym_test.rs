#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! A method call never resolves to a free function of the same name. `list.wj` declares `fn len(c: List) -> int`;
//! `wrap.wj` calls `cur.len()` on a local declared `int` (no `len` method). Neither numeric inference nor the IR
//! may type the call from the free function, so the literal compared with it stays unsuffixed (not `0_i64`).
//! The program is ill-typed on purpose (the receiver's declared type has no such method), so the check reads the
//! emitted text; `tests/project_corpus/method_call_never_names_a_free_function` is the well-typed case under rustc.

use std::fs;
use std::path::Path;
use std::process::Command;

const LIST: &str = "enum List {\n    Cons(int, Box<List>),\n    Nil,\n}\n\nfn len(c: List) -> int {\n    match c {\n        \
List::Cons(_, rest) => 1 + len(*rest),\n        List::Nil => 0,\n    }\n}\n\nfn main() {\n    println(\"${len(List::Cons(1, Box::new(List::Nil)))}\")\n}\n";

const WRAP: &str = "use list\n\nfn wrap(text: string, width: int) -> Vec<string> {\n    let mut lines: Vec<string> = []\n    \
let mut cur: int = \"\"\n    for w in text.split_whitespace() {\n        if cur.len() > 0 && (cur.len() + 1 + w.len()) as int > width {\n            \
lines.push(cur)\n            cur = w.to_string()\n        } else if cur.len() == 0 {\n            cur = w.to_string()\n        } else {\n            \
cur = cur + \" \" + w\n        }\n    }\n    if cur.len() > 0 { lines.push(cur) }\n    lines\n}\n\nfn main() {\n    \
for l in wrap(\"alpha beta gamma\", 12) {\n        println(l)\n    }\n}\n";

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

#[test]
fn method_call_is_not_typed_from_a_free_function_of_the_same_name() {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    write(t, "wj.toml", "[package]\nname = \"bare_name_method\"\nversion = \"0.1.0\"\n");
    write(t, "src/list.wj", LIST);
    write(t, "src/wrap.wj", WRAP);
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "--no-cargo", "--no-lint", "-o", "out", "."])
        .env("WJ_CARGO_TARGET_ROOT", t.join(".cacheroot"))
        .current_dir(t)
        .output()
        .unwrap();
    let wrap_rs = fs::read_to_string(t.join("out/src/wrap.rs")).unwrap_or_else(|_| {
        panic!("no out/src/wrap.rs:\n{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
    });
    assert!(wrap_rs.contains("cur.len() > 0"), "cur.len() compared with a literal:\n{wrap_rs}");
    assert!(!wrap_rs.contains("cur.len() > 0_i64"), "cur.len() typed from the free fn len(c: List) -> int:\n{wrap_rs}");
}
