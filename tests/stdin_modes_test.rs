//! Reading stdin works the same in the interpreter and the compiled path (agent-dx finding):
//! `io::stdin().read_line(line)` must fill `line` (compiled: `&mut line`, never a clone), and the free
//! functions `io::read_line()` / `io::read_to_string(io::stdin())` exist in the interpreter.

use std::io::Write;
use std::process::{Command, Stdio};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("wj-stdin-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_interp(src: &str, name: &str, input: &str) -> String {
    let d = dir(name);
    let f = d.join("t.wj");
    std::fs::write(&f, src).unwrap();
    let mut c = Command::new(WJ).args(["run", "--interpret"]).arg(&f).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    c.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    String::from_utf8_lossy(&c.wait_with_output().unwrap().stdout).trim().to_string()
}

const LOOP: &str = "use std::io\n\nfn main() {\n    let mut line = String::new()\n    let mut total = 0\n    loop {\n        line.clear()\n        let n = match io::stdin().read_line(line) { Ok(k) => k, Err(_) => 0 }\n        if n == 0 {\n            break\n        }\n        total += n\n    }\n    println(\"${total}\")\n}\n";

#[test]
fn read_line_loop_terminates_in_the_interpreter() {
    assert_eq!(run_interp(LOOP, "loop", "ab\ncd\n"), "6");
}

#[test]
fn compiled_read_line_borrows_the_buffer_mutably() {
    let d = dir("compiled");
    let f = d.join("t.wj");
    std::fs::write(&f, LOOP).unwrap();
    let o = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).current_dir(&d).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let rs = std::fs::read_to_string(d.join("build/t.rs")).unwrap();
    assert!(rs.contains("read_line(&mut line)"), "{rs}");
    assert!(!rs.contains("read_line(line.clone())"), "{rs}");
}

#[test]
fn free_stdin_functions_work_in_the_interpreter() {
    let src = "use std::io\n\nfn main() {\n    let a = io::read_line()\n    let rest = io::read_to_string(io::stdin())\n    println(\"${a:?} ${rest:?}\")\n}\n";
    assert_eq!(run_interp(src, "free", "ab\ncd\nef\n"), "Ok(\"ab\") Ok(\"cd\\nef\\n\")");
}
