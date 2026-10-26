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
    interp_with(src, &[])
}

fn interp_with(src: &str, flags: &[&str]) -> Out {
    let d = TempDir::new().unwrap();
    let f = d.path().join("p.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).args(flags).arg(&f).output().unwrap();
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

#[test]
fn u128_above_i128_max() {
    parity(
        r#"
fn main() {
    let m = u128::MAX
    println("${m}")
    let one: u128 = 1
    let x = one << 127
    println("${x + 5} ${x > 5} ${x / 3}")
    println("${m.wrapping_add(2)} ${m.checked_add(1).is_none()} ${m.count_ones()}")
    let back = "340282366920938463463374607431768211455".parse::<u128>().unwrap_or(0)
    println("${back == m} ${(x as f64) > 1.0e38} ${(m as u64)}")
    let mut v: Vec<u128> = [x, 3, m]
    v.sort()
    println("${v:?}")
    let y = x * 2
}
"#,
        r#"
fn main() {
    let m = u128::MAX;
    println!("{}", m);
    let one: u128 = 1;
    let x = one << 127;
    println!("{} {} {}", x + 5, x > 5, x / 3);
    println!("{} {} {}", m.wrapping_add(2), m.checked_add(1).is_none(), m.count_ones());
    let back = "340282366920938463463374607431768211455".parse::<u128>().unwrap_or(0);
    println!("{} {} {}", back == m, (x as f64) > 1.0e38, (m as u64));
    let mut v: Vec<u128> = vec![x, 3, m];
    v.sort();
    println!("{:?}", v);
    let y = x * std::hint::black_box(2);
}
"#,
    );
}

#[test]
fn mutex_poisoning() {
    parity(
        r#"
use std::sync::{Arc, Mutex}
use std::thread
fn main() {
    let m = Arc::new(Mutex::new(5))
    let m2 = m.clone()
    let h = thread::spawn(move || {
        let mut g = m2.lock().unwrap()
        *g += 1
        panic("boom")
    })
    let r = h.join()
    println("joined err ${r.is_err()} poisoned ${m.is_poisoned()}")
    match m.lock() {
        Ok(g) => println("ok ${*g}"),
        Err(e) => {
            let g = e.into_inner()
            println("poisoned, value ${*g}")
        }
    }
}
"#,
        r#"
use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let m = Arc::new(Mutex::new(5));
    let m2 = m.clone();
    let h = thread::spawn(move || {
        let mut g = m2.lock().unwrap();
        *g += 1;
        panic!("boom");
    });
    let r = h.join();
    println!("joined err {} poisoned {}", r.is_err(), m.is_poisoned());
    match m.lock() {
        Ok(g) => println!("ok {}", *g),
        Err(e) => {
            let g = e.into_inner();
            println!("poisoned, value {}", *g);
        }
    };
}
"#,
    );
}

#[test]
fn subprocess_line_pipes() {
    let src = r#"
use std::subprocess
fn main() {
    let args: Vec<string> = []
    match subprocess::spawn("cat", args) {
        Ok(h) => {
            subprocess::write_line(h, "ping")
            subprocess::write_line(h, "@AGENT OBS state=1")
            println(subprocess::read_line(h).unwrap_or("?"))
            println(subprocess::read_agent_obs_n(h, 1).unwrap_or("?"))
            subprocess::kill(h)
            subprocess::close(h)
            println(subprocess::is_valid(h))
        }
        Err(e) => println("spawn failed ${e}"),
    }
    println(subprocess::frames_in_agent_command("frames=30;forward=1"))
    match subprocess::spawn("definitely-not-a-program-xyz", []) {
        Ok(_) => println("unexpected"),
        Err(e) => println(e.starts_with("spawn definitely-not-a-program-xyz:")),
    }
}
"#;
    let got = interp_with(src, &["--allow", "process=cat", "--allow", "process=definitely-not-a-program-xyz"]);
    assert_eq!(got.stdout, "ping\nstate=1\ntrue\n30\ntrue\n");
}

#[test]
fn cli_parsing_matches_clap() {
    let src = r#"
use std::cli
fn main() {
    let m = cli::app("tool", "1.2.3", "A tool")
        .arg("name", Some('n'), Some("name"), "who", true)
        .flag("verbose", Some('v'), Some("verbose"), "chatty")
        .parse()
    println("${m.get(\"name\").unwrap_or(\"-\")} ${m.is_present(\"verbose\")} ${m.is_present(\"name\")}")
}
"#;
    let d = tempfile::TempDir::new().unwrap();
    let f = d.path().join("p.wj");
    fs::write(&f, src.replace("\\\"", "'").replace("m.get('name')", "m.get(\"name\")")).unwrap();
    // `"` inside `${..}` is a lexer limitation; read the values into locals instead
    let src2 = r#"
use std::cli
fn main() {
    let m = cli::app("tool", "1.2.3", "A tool")
        .arg("name", Some('n'), Some("name"), "who", true)
        .flag("verbose", Some('v'), Some("verbose"), "chatty")
        .parse()
    let n = m.get("name").unwrap_or("-")
    let v = m.is_present("verbose")
    println("${n} ${v}")
}
"#;
    fs::write(&f, src2).unwrap();
    let run = |args: &[&str]| {
        // program arguments follow `--` (otherwise `wj` parses them as its own options)
        let o = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).arg(&f).arg("--").args(args).output().unwrap();
        (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code())
    };
    let (out, err, code) = run(&["--name", "bob", "-v"]);
    assert_eq!((out.as_str(), code), ("bob true\n", Some(0)), "{err}");
    let (out, _, code) = run(&["-n", "amy"]);
    assert_eq!((out.as_str(), code), ("amy false\n", Some(0)));
    // clap: a missing required argument is an error on stderr with exit status 2; --version prints and exits 0
    let (_, err, code) = run(&[]);
    assert_eq!(code, Some(2));
    assert!(err.contains("--name"), "{err}");
    let (out, _, code) = run(&["--version"]);
    assert_eq!((out.as_str(), code), ("tool 1.2.3\n", Some(0)));
}

const MOVES_WJ: &str = r#"
struct G { n: int }
impl Drop for G { fn drop(&mut self) { println("drop ${self.n}") } }
struct Pair { left: G, right: G }
struct Wrap { g: G }
fn keep(g: G) {
    let mut v: Vec<G> = Vec::new()
    v.push(g)
    println("in keep")
}
fn take<T>(t: T) {
    println("took")
}
fn show(g: G) {
    println("show ${g.n}")
}
fn main() {
    let a = G { n: 1 }
    let b = G { n: 2 }
    {
        let mut v: Vec<G> = Vec::new()
        v.push(a)
        println("inner end")
    }
    println("after block")
    keep(G { n: 3 })
    let c = G { n: 4 }
    keep(c)
    println("after keep")
    let d = G { n: 5 }
    take(d)
    println("after take")
    let p = Pair { left: G { n: 6 }, right: G { n: 7 } }
    {
        let l = p.left
        println("partial ${l.n}")
    }
    println("after partial")
    let e = G { n: 8 }
    {
        let w = Wrap { g: e }
        println("wrapped ${w.g.n}")
    }
    let f = G { n: 9 }
    {
        let f2 = f
        println("rebound ${f2.n}")
    }
    let s = G { n: 10 }
    show(s)
    println("end ${b.n}")
}
"#;

const MOVES_RS: &str = r#"
struct G { n: i64 }
impl Drop for G { fn drop(&mut self) { println!("drop {}", self.n) } }
struct Pair { left: G, right: G }
struct Wrap { g: G }
fn keep(g: G) {
    let mut v: Vec<G> = Vec::new();
    v.push(g);
    println!("in keep");
}
fn take<T>(_t: T) {
    println!("took");
}
fn show(g: &G) {
    println!("show {}", g.n);
}
fn main() {
    let a = G { n: 1 };
    let b = G { n: 2 };
    {
        let mut v: Vec<G> = Vec::new();
        v.push(a);
        println!("inner end");
    }
    println!("after block");
    keep(G { n: 3 });
    let c = G { n: 4 };
    keep(c);
    println!("after keep");
    let d = G { n: 5 };
    take(d);
    println!("after take");
    let p = Pair { left: G { n: 6 }, right: G { n: 7 } };
    {
        let l = p.left;
        println!("partial {}", l.n);
    }
    println!("after partial");
    let e = G { n: 8 };
    {
        let w = Wrap { g: e };
        println!("wrapped {}", w.g.n);
    }
    let f = G { n: 9 };
    {
        let f2 = f;
        println!("rebound {}", f2.n);
    }
    let s = G { n: 10 };
    show(&s);
    println!("end {}", b.n);
}
"#;

#[test]
fn moves_decide_where_destructors_run() {
    parity(MOVES_WJ, MOVES_RS);
}
