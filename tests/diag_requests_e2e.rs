//! End-to-end gate for the agent-pack diagnostic requests R0-R3
//! (docs/design/diagnostic-requests-from-agent-pack.md): `wj run --compiled` on a program that the
//! generated Rust rejects must print a Windjammer diagnostic: the stable code, a `.wj` span on the
//! right line, the fix hint, and NO rustc text (`main.rs`, `E0xxx`, `rustc --explain`).
//!
//! (The dropped-branch wording of R2 is covered by diag_hints_suite against real rustc output: no `.wj` program
//! that reaches that rustc error is known today, the compiler accepts the shapes tried.)
//!
//! The expected line is the line of the offending expression in the `.wj` source (the oracle).
//! Run: `CARGO_BUILD_JOBS=2 cargo test --release --test diag_requests_e2e`.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::process::Command;

struct Case {
    name: &'static str,
    src: &'static str,
    code: &'static str,
    line: usize,
    hint: &'static [&'static str],
}

const CASES: &[Case] = &[
    // Shapes rustc still rejects. The solver now lowers `len()` into an int slot (plain or in a tuple), `let mut k = 0` widened by a later
    // `parse::<int>()`, and `collect()` of `split(..)`/`split_whitespace()` into `Vec<string>`; those compile and run correctly.
    // `nth(r)` with an int `r` now compiles (the usize slot converts it), so R1 uses a closure body: `len()` there is
    // still `usize` beside an `int` operand (tests/inference_corpus/closure_len_width.wj, in the rustc_rejects ratchet).
    Case {
        name: "r1_len_in_closure_body",
        src: "fn main() {\n    let v = [1, 2, 3]\n    let f = |x: int| x + v.len()\n    println(\"${f(1)}\")\n}\n",
        code: "WJ0003",
        line: 3,
        hint: &["expected `int`, found `uint`", "as int"],
    },
    Case {
        name: "r2_char_for_string",
        src: "fn digit_char(d: int) -> string {\n    if d < 10 { (d + 48) as u8 as char } else { (d + 55) as u8 as char }\n}\nfn main() {\n    println(digit_char(3))\n}\n",
        code: "WJ0003",
        line: 2,
        hint: &["c.to_string()", "s.chars()"],
    },
    Case {
        name: "r3_collect_turbofish",
        src: "fn main() {\n    let line = \" a b  c \"\n    let rows = line.lines().map(|l| l.trim()).collect::<Vec<string>>()\n    println(\"${rows.len()}\")\n}\n",
        code: "WJ0029",
        line: 3,
        hint: &["add `.map(|s| s.to_string())` before `.collect()`", "cannot collect `&str` items into `Vec<string>`"],
    },
];

fn run_case(c: &Case) -> (String, i32) {
    let dir = Scratch::new("e2e");
    let f = dir.join("main.wj");
    std::fs::write(&f, c.src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["run", "--compiled"])
        .arg(&f)
        .env("WJ_CARGO_TARGET_ROOT", scratch::shared_cache_dir("e2e-target"))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (String::from_utf8_lossy(&out.stderr).to_string(), out.status.code().unwrap_or(-1))
}

#[test]
fn rustc_rejections_reach_the_user_as_windjammer_diagnostics() {
    let mut bad = Vec::new();
    for c in CASES {
        let (err, code) = run_case(c);
        let mut why = Vec::new();
        if code == 0 {
            why.push("program unexpectedly compiled".to_string());
        }
        if !err.contains(&format!("[{}]", c.code)) {
            why.push(format!("missing code {}", c.code));
        }
        if !err.contains(&format!("main.wj:{}:", c.line)) {
            why.push(format!("no `.wj` span on line {}", c.line));
        }
        for leak in ["main.rs", "rustc --explain", "error[E0", "cargo", "could not compile"] {
            if err.contains(leak) {
                why.push(format!("rustc text leaked: `{leak}`"));
            }
        }
        for h in c.hint {
            if !err.contains(h) {
                why.push(format!("hint missing `{h}`"));
            }
        }
        if !why.is_empty() {
            bad.push(format!("{}: {}\n---- stderr ----\n{err}", c.name, why.join("; ")));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
