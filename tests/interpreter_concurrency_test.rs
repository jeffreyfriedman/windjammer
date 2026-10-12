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

//! Interpreter concurrency: the deterministic cooperative scheduler.
//!
//! Expected outputs are derived independently of the interpreter: Python reference models
//! (`interpreter_corpus/*.ref.py`, vectors), or the Rust standard library's documented behaviour
//! worked out by hand (comments). Every test runs the real `wj run --interpret`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Out {
    stdout: String,
    stderr: String,
    exit: i32,
}

fn run(src: &str, flags: &[&str]) -> Out {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret"]).args(flags).arg(&f).output().unwrap();
    Out {
        stdout: String::from_utf8_lossy(&o.stdout).to_string(),
        stderr: String::from_utf8_lossy(&o.stderr).to_string(),
        exit: o.status.code().unwrap_or(-1),
    }
}

fn corpus(name: &str, seed: u64) -> (String, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/interpreter_corpus");
    let src = fs::read_to_string(dir.join(format!("{name}.wj"))).unwrap().replace("__SEED__", &seed.to_string());
    let want = fs::read_to_string(dir.join(format!("{name}.vectors/{seed}.out"))).unwrap();
    (src, want)
}

#[test]
fn synchronised_program_gives_the_oracle_output_under_every_schedule() {
    for seed in [3u64, 21] {
        let (src, want) = corpus("threads_sync", seed);
        for sched in 0..10u64 {
            let got = run(&src, &["--sched-seed", &sched.to_string()]);
            assert_eq!(got.stdout, want, "threads_sync data seed {seed}, schedule {sched}: {}", got.stderr);
            assert_eq!(got.exit, 0);
        }
    }
}

const RACE: &str = r#"
use std::sync::{Arc, Mutex}
use std::thread

fn bump(c: Arc<Mutex<i64>>) -> i64 {
    let seen = match c.lock() { Ok(g) => *g, Err(_) => 0 }
    match c.lock() {
        Ok(mut g) => { *g = seen + 1 }
        Err(_) => {}
    }
    0
}

fn main() {
    let c = Arc::new(Mutex::new(0))
    let a = c.clone()
    let b = c.clone()
    let h1 = thread::spawn(move || { return bump(a) })
    let h2 = thread::spawn(move || { return bump(b) })
    h1.join()
    h2.join()
    let v = match c.lock() { Ok(g) => *g, Err(_) => -1 }
    println("final {}", v)
}
"#;

#[test]
fn schedule_exploration_finds_the_lost_update_and_is_reproducible() {
    // read-then-write across two lock acquisitions: the only outcomes a real machine can produce are
    // 1 (both read 0) and 2 (serialised); never 0 and never 3.
    let mut seen = BTreeSet::new();
    for sched in 0..24u64 {
        let a = run(RACE, &["--sched-seed", &sched.to_string()]);
        let b = run(RACE, &["--sched-seed", &sched.to_string()]);
        assert_eq!(a.stdout, b.stdout, "schedule {sched} must be reproducible");
        seen.insert(a.stdout.trim().to_string());
    }
    let want: BTreeSet<String> = ["final 1", "final 2"].iter().map(|s| s.to_string()).collect();
    assert_eq!(seen, want, "exploration must reach both the racy and the serialised outcome");
    // the default schedule never preempts: spawned threads run to completion one after the other
    assert_eq!(run(RACE, &[]).stdout, "final 2\n");
}

#[test]
fn sleeping_threads_wake_in_deadline_order_on_the_virtual_clock() {
    let src = r#"
use std::sync::mpsc
use std::thread
use std::time::{Duration, Instant}

fn main() {
    let start = Instant::now()
    let (tx, rx) = mpsc::channel()
    let delays: Vec<u64> = [300, 100, 200]
    for d in delays {
        let t = tx.clone()
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(d))
            t.send(d)
            return 0
        })
    }
    drop(tx)
    for v in rx { println("woke {}", v) }
    println("slept {}", start.elapsed() >= Duration::from_millis(300))
}
"#;
    let t0 = std::time::Instant::now();
    let o = run(src, &[]);
    assert_eq!(o.stdout, "woke 100\nwoke 200\nwoke 300\nslept true\n", "{}", o.stderr);
    assert!(t0.elapsed() < std::time::Duration::from_secs(5), "virtual sleep must not wait in real time");
}

#[test]
fn deterministic_time_makes_the_clock_reproducible() {
    let src = r#"
use std::thread
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH}

fn main() {
    let a = Instant::now()
    thread::sleep(Duration::from_millis(1500))
    println("{:?}", a.elapsed())
    let s = match SystemTime::now().duration_since(UNIX_EPOCH) { Ok(d) => d.as_secs(), Err(_) => 0 }
    println("{}", s)
}
"#;
    // 1_700_000_000 s fixed origin + 1.5 s of virtual sleep, Duration's Debug is `1.5s`
    let o = run(src, &["--deterministic-time"]);
    assert_eq!(o.stdout, "1.5s\n1700000001\n", "{}", o.stderr);
}

#[test]
fn deadlock_is_a_located_error_not_a_hang() {
    let src = r#"
use std::sync::mpsc
use std::thread
fn main() {
    let (tx, rx) = mpsc::channel()
    let h = thread::spawn(move || {
        let v = match rx.recv() { Ok(x) => x, Err(_) => -1 }
        return v
    })
    let r = h.join()
    println("unreachable {}", tx.send(1).is_ok())
}
"#;
    let o = run(src, &[]);
    assert_eq!(o.exit, 1);
    assert!(o.stderr.contains("deadlock"), "{}", o.stderr);
    assert_eq!(o.stdout, "");
}

#[test]
fn refcell_double_borrow_panics_with_rusts_message() {
    let src = "use std::cell::RefCell\nfn main() {\n    let c = RefCell::new(5)\n    let a = c.borrow_mut()\n    println(\"one\")\n    let b = c.borrow_mut()\n    println(\"two\")\n}\n";
    let o = run(src, &[]);
    assert_eq!(o.exit, 101);
    assert_eq!(o.stdout, "one\n");
    assert!(o.stderr.contains("already borrowed: BorrowMutError"), "{}", o.stderr);
    // a shared borrow while mutably borrowed
    let src2 = "use std::cell::RefCell\nfn main() {\n    let c = RefCell::new(5)\n    let a = c.borrow_mut()\n    let b = c.borrow()\n}\n";
    let o2 = run(src2, &[]);
    assert!(o2.stderr.contains("already mutably borrowed: BorrowError"), "{}", o2.stderr);
    // guards released at scope end allow later borrows
    let src3 = "use std::cell::RefCell\nfn main() {\n    let c = RefCell::new(5)\n    {\n        let a = c.borrow_mut()\n    }\n    let b = c.borrow_mut()\n    println(\"ok\")\n}\n";
    assert_eq!(run(src3, &[]).stdout, "ok\n");
}

#[test]
fn panicking_thread_reports_and_join_returns_err() {
    let src = r#"
use std::thread
fn main() {
    let h = thread::spawn(move || {
        panic("boom")
        return 1
    })
    let r = h.join()
    println("joined {}", r.is_err())
}
"#;
    let o = run(src, &[]);
    assert_eq!(o.stdout, "joined true\n");
    assert_eq!(o.exit, 0);
    assert!(o.stderr.contains("thread '<unnamed>' panicked"), "{}", o.stderr);
    assert!(o.stderr.contains("boom"), "{}", o.stderr);
}

#[test]
fn condvar_barrier_and_bounded_channel() {
    let src = r#"
use std::sync::{Arc, Mutex, Condvar, Barrier}
use std::sync::mpsc
use std::thread
use std::thread::JoinHandle

fn main() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()))
    let p2 = pair.clone()
    let h = thread::spawn(move || {
        let (lock, cv) = &*p2
        match lock.lock() {
            Ok(mut g) => { *g = true }
            Err(_) => {}
        }
        cv.notify_one()
        return 1
    })
    let (lock, cv) = &*pair
    let mut ready = match lock.lock() { Ok(g) => g, Err(_) => return }
    while !*ready {
        ready = match cv.wait(ready) { Ok(g) => g, Err(_) => return }
    }
    println("ready {}", *ready)
    h.join()

    let (stx, srx) = mpsc::sync_channel(1)
    let prod = thread::spawn(move || {
        let mut i: i64 = 0
        while i < 4 {
            stx.send(i)
            i += 1
        }
        return 0
    })
    let mut got: Vec<i64> = Vec::new()
    for v in srx { got.push(v) }
    println("bounded {:?}", got)
    prod.join()

    let barrier = Arc::new(Barrier::new(3))
    let log = Arc::new(Mutex::new(Vec::new()))
    let mut hs: Vec<JoinHandle<i64>> = Vec::new()
    let mut i: i64 = 0
    while i < 3 {
        let b = barrier.clone()
        let l = log.clone()
        let id = i
        hs.push(thread::spawn(move || {
            match l.lock() { Ok(mut g) => { g.push(id) } Err(_) => {} }
            b.wait()
            match l.lock() { Ok(mut g) => { g.push(id + 10) } Err(_) => {} }
            return 0
        }))
        i += 1
    }
    for h in hs { h.join() }
    let order = match log.lock() { Ok(g) => g.clone(), Err(_) => Vec::new() }
    // every "before" entry (0..3) precedes every "after" entry (10..13)
    let mut ok = true
    let mut k: usize = 0
    while k < 3 { if order[k] >= 10 { ok = false } k += 1 }
    while k < 6 { if order[k] < 10 { ok = false } k += 1 }
    println("barrier {} {}", ok, order.len())
}
"#;
    for sched in [0u64, 1, 2, 3, 4, 5] {
        let o = run(src, &["--sched-seed", &sched.to_string()]);
        assert_eq!(o.stdout, "ready true\nbounded [0, 1, 2, 3]\nbarrier true 6\n", "schedule {sched}: {}", o.stderr);
    }
}

#[test]
fn scoped_threads_borrow_and_join_at_scope_end() {
    let src = r#"
use std::thread

fn main() {
    let data: Vec<i64> = [1, 2, 3, 4, 5, 6]
    let mut left: i64 = 0
    let mut right: i64 = 0
    thread::scope(|s| {
        s.spawn(|| {
            let mut t: i64 = 0
            for x in 0..3 { t += data[x] }
            left = t
        })
        s.spawn(|| {
            let mut t: i64 = 0
            for x in 3..6 { t += data[x] }
            right = t
        })
    })
    println("{} {}", left, right)
}
"#;
    let o = run(src, &[]);
    assert_eq!(o.stdout, "6 15\n", "{}", o.stderr);
}
