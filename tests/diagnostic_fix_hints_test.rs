//! Every diagnostic an agent commonly meets carries a concrete one-line `help:` and, where the fix is mechanical, a
//! machine-applicable edit: applying it (`wj check --fix`) leaves a program that `wj check` accepts. The corpus is the set
//! of diagnostics seen in the pilot-7 haiku failures (private-docs/pilot7-analysis.md).

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

use std::process::Command;
use tempfile::TempDir;

struct Case {
    name: &'static str,
    code: &'static str,
    /// `wj check --fix` makes the program check clean.
    auto_fix: bool,
    src: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "local changed without mut",
        code: "WJ0051",
        auto_fix: true,
        src: "struct Cursor {\n    pos: int,\n}\n\nfn advance(c: Cursor) -> int {\n    c.pos += 1\n    c.pos\n}\n\nfn main() {\n    let cur = Cursor { pos: 0 }\n    println(\"${advance(cur)}\")\n}\n",
    },
    Case {
        name: "string literal to_string",
        code: "W0006",
        auto_fix: true,
        src: "fn main() {\n    let s = \"x\".to_string()\n    println(\"${s}\")\n}\n",
    },
    Case {
        name: "explicit iter",
        code: "W0003",
        auto_fix: true,
        src: "fn main() {\n    let v = [1, 2]\n    for x in v.iter() {\n        println(\"${x}\")\n    }\n}\n",
    },
    Case {
        name: "iterator indexed",
        code: "WJ0035",
        auto_fix: true,
        src: "fn main() {\n    let parts = \"a b\".split_whitespace()\n    println(\"${parts[1]}\")\n}\n",
    },
    Case {
        name: "list method on an iterator",
        code: "WJ0035",
        auto_fix: true,
        src: "fn main() {\n    let parts = \"a b\".split(\" \")\n    println(\"${parts.contains(\"a\")}\")\n}\n",
    },
    Case {
        name: "iterator given to a declared list",
        code: "WJ0035",
        auto_fix: true,
        src: "fn main() {\n    let line = \"hello world\"\n    let parts: Vec<string> = line.split(\" \")\n    println(\"${parts.len()}\")\n}\n",
    },
    Case {
        name: "bodyless trait receiver",
        code: "W0016",
        auto_fix: false,
        src: "trait Rule {\n    fn apply(self, x: int) -> int\n}\n\nfn main() {\n    println(\"ok\")\n}\n",
    },
    Case {
        name: "explicit unwrap",
        code: "W0002",
        auto_fix: false,
        src: "fn main() {\n    let n = \"5\".parse::<int>().unwrap()\n    println(\"${n}\")\n}\n",
    },
];

fn wj(args: &[&str], dir: &std::path::Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(args)
        .current_dir(dir)
        .env("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1")
        .output()
        .unwrap();
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn common_diagnostics_carry_a_help_and_a_working_fix() {
    let mut failures = Vec::new();
    for c in CASES {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("main.wj"), c.src).unwrap();
        let text = wj(&["check", "main.wj"], dir.path());
        if !text.contains(c.code) {
            failures.push(format!("{}: `{}` not reported:\n{text}", c.name, c.code));
            continue;
        }
        if !text.contains("= help:") {
            failures.push(format!("{}: `{}` has no help line:\n{text}", c.name, c.code));
        }
        if c.auto_fix {
            if !text.contains("= fix (auto-fixable)") {
                failures.push(format!("{}: `{}` has no machine-applicable fix:\n{text}", c.name, c.code));
                continue;
            }
            wj(&["check", "--fix", "main.wj"], dir.path());
            let after = wj(&["check", "main.wj"], dir.path());
            if !after.contains("0 error(s)") {
                failures.push(format!("{}: still failing after `wj check --fix`:\n{after}", c.name));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n---\n"));
}
