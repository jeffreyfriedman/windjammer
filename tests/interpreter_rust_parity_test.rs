#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Interpreter vs hand-written Rust: each case pairs a Windjammer program with the equivalent Rust
//! program, compiled by `rustc` (an oracle that shares no code with the interpreter or with
//! Windjammer's code generator). stdout, exit status and the panic message must agree.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Out {
    stdout: String,
    code: i32,
    panic: Option<String>,
}

fn panic_line(stderr: &str) -> Option<String> {
    let mut lines = stderr.lines();
    while let Some(l) = lines.next() {
        if l.starts_with("thread '") && l.contains("panicked at") {
            return lines.next().map(|m| m.trim().to_string());
        }
    }
    None
}

fn interp(src: &str) -> Out {
    let d = TempDir::new().unwrap();
    let f = d.path().join("p.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret"]).arg(&f).output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    Out { stdout: String::from_utf8_lossy(&o.stdout).to_string(), code: o.status.code().unwrap_or(-1), panic: panic_line(&err) }
}

fn rust(src: &str) -> Option<Out> {
    let d = TempDir::new().unwrap();
    let f = d.path().join("p.rs");
    fs::write(&f, src).unwrap();
    let bin = d.path().join("p");
    let c = Command::new("rustc").arg(&f).arg("-o").arg(&bin).args(["--edition", "2021", "-A", "warnings"]).output().ok()?;
    assert!(c.status.success(), "oracle program does not compile:\n{}", String::from_utf8_lossy(&c.stderr));
    let o = Command::new(&bin).output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    Some(Out { stdout: String::from_utf8_lossy(&o.stdout).to_string(), code: o.status.code().unwrap_or(-1), panic: panic_line(&err) })
}

fn parity(wj: &str, rs: &str) {
    let Some(want) = rust(rs) else {
        eprintln!("SKIPPED: rustc unavailable");
        return;
    };
    let got = interp(wj);
    assert_eq!(got.stdout, want.stdout, "stdout differs");
    assert_eq!(got.code, want.code, "exit status differs");
    assert_eq!(got.panic, want.panic, "panic message differs");
}

#[test]
fn rc_make_mut_get_mut_and_cycles_leak() {
    parity(
        r#"
use std::rc::{Rc, Weak}
use std::cell::RefCell
struct Node { next: RefCell<Option<Rc<Node>>>, tag: int }
fn main() {
    let mut a = Rc::new(5)
    let b = Rc::clone(&a)
    let m = Rc::make_mut(&mut a)
    *m += 1
    println("${a} ${b} ${Rc::ptr_eq(&a, &b)}")
    let mut c = Rc::new(10)
    match Rc::get_mut(&mut c) { Some(x) => { *x += 1 } None => {} }
    let c2 = Rc::clone(&c)
    let none = Rc::get_mut(&mut c).is_none()
    println("${c} ${none}")
    let mut w: Weak<Node> = Weak::new()
    {
        let n1 = Rc::new(Node { next: RefCell::new(None), tag: 1 })
        let n2 = Rc::new(Node { next: RefCell::new(Some(Rc::clone(&n1))), tag: 2 })
        {
            let mut slot = n1.next.borrow_mut();
            *slot = Some(Rc::clone(&n2))
        }
        println("strong ${Rc::strong_count(&n1)} ${Rc::strong_count(&n2)}")
        w = Rc::downgrade(&n1)
    }
    println("leaked ${w.upgrade().is_some()}")
}
"#,
        r#"
use std::rc::{Rc, Weak};
use std::cell::RefCell;
struct Node { next: RefCell<Option<Rc<Node>>>, tag: i64 }
fn main() {
    let mut a = Rc::new(5);
    let b = Rc::clone(&a);
    let m = Rc::make_mut(&mut a);
    *m += 1;
    println!("{} {} {}", a, b, Rc::ptr_eq(&a, &b));
    let mut c = Rc::new(10);
    match Rc::get_mut(&mut c) { Some(x) => { *x += 1 } None => {} }
    let c2 = Rc::clone(&c);
    let none = Rc::get_mut(&mut c).is_none();
    println!("{} {}", c, none);
    let mut w: Weak<Node> = Weak::new();
    {
        let n1 = Rc::new(Node { next: RefCell::new(None), tag: 1 });
        let n2 = Rc::new(Node { next: RefCell::new(Some(Rc::clone(&n1))), tag: 2 });
        {
            let mut slot = n1.next.borrow_mut();
            *slot = Some(Rc::clone(&n2));
        }
        println!("strong {} {}", Rc::strong_count(&n1), Rc::strong_count(&n2));
        w = Rc::downgrade(&n1);
    }
    println!("leaked {}", w.upgrade().is_some());
}
"#,
    );
}

#[test]
fn drop_runs_when_ownership_ends() {
    parity(
        r#"
struct G { n: int }
impl Drop for G { fn drop(&mut self) { println("drop ${self.n}") } }
fn consume(g: G) {
    drop(g)
    println("consumed")
}
fn main() {
    let a = G { n: 1 }
    consume(a)
    let mut v: Vec<G> = Vec::new()
    v.push(G { n: 2 })
    v.push(G { n: 3 })
    v.clear()
    println("cleared")
    let b = G { n: 4 }
    let c = G { n: 5 }
    println("end")
}
"#,
        r#"
struct G { n: i64 }
impl Drop for G { fn drop(&mut self) { println!("drop {}", self.n) } }
fn consume(g: G) {
    drop(g);
    println!("consumed");
}
fn main() {
    let a = G { n: 1 };
    consume(a);
    let mut v: Vec<G> = Vec::new();
    v.push(G { n: 2 });
    v.push(G { n: 3 });
    v.clear();
    println!("cleared");
    let b = G { n: 4 };
    let c = G { n: 5 };
    println!("end");
}
"#,
    );
}

#[test]
fn slicing_panics_use_rusts_messages() {
    for (wj_expr, rs_expr) in [
        ("v[1..9]", "&v[1..9]"),
        ("v[3..2]", "&v[3..2]"),
        ("v[7..]", "&v[7..]"),
        ("s[0..9]", "&s[0..9]"),
        ("s[1..2]", "&s[1..2]"),
        ("s[3..1]", "&s[3..1]"),
        ("s[9..]", "&s[9..]"),
        ("s[2..]", "&s[2..]"),
    ] {
        let wj = format!(
            "fn main() {{\n    let v: Vec<int> = [1, 2, 3, 4, 5]\n    let s = \"héllo\"\n    println(\"start\")\n    let x = {wj_expr}\n    println(\"unreachable\")\n}}\n"
        );
        let rs = format!(
            "fn main() {{\n    let v: Vec<i64> = vec![1, 2, 3, 4, 5];\n    let s = \"héllo\";\n    println!(\"start\");\n    let x = {rs_expr};\n    println!(\"unreachable\");\n}}\n"
        );
        parity(&wj, &rs);
    }
}

#[test]
fn char_case_mapping_displays_like_rust() {
    parity(
        "fn main() {\n    let c = 'Σ'\n    println(\"${c.to_lowercase()} ${'ß'.to_uppercase()} ${'a'.to_uppercase()}\")\n}\n",
        "fn main() {\n    let c = 'Σ';\n    println!(\"{} {} {}\", c.to_lowercase(), 'ß'.to_uppercase(), 'a'.to_uppercase());\n}\n",
    );
}
