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

//! Tasks as nested calls on one thread (ADR-0034, `src/interpreter/sched.rs`): the mode the browser playground runs in,
//! where there are no OS threads. Every program here runs in both modes, in this process, and the one-thread mode must
//! print what the thread mode prints, or stop with the error that says a task would have to resume; it must never print
//! something different. The five book samples that start tasks must agree outright.

use std::fs;
use std::path::{Path, PathBuf};
use windjammer::interpreter::engine::Interpreter;
use windjammer::interpreter::project::LoadedFile;
use windjammer::interpreter::{loader, runner};

struct Done {
    stdout: String,
    stderr: String,
    code: i32,
}

fn exec(src: &str, one_thread: bool, sched_seed: u64) -> Done {
    let src = src.to_string();
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(move || {
            let program = loader::parse_source(&src, "main.wj").expect("parse");
            let mut it = Interpreter::new_capturing();
            it.set_capture_stderr(true);
            it.set_jit(false);
            it.set_deterministic_time(true);
            it.set_sched_seed(sched_seed);
            it.set_one_thread_tasks(one_thread);
            let file = LoadedFile { module_path: Vec::new(), file: PathBuf::from("main.wj"), program: Box::leak(Box::new(program)) };
            let mut shown = String::new();
            let code = match it.load_sources(vec![file], Vec::new(), Vec::new()) {
                Err(e) => runner::exit_code_for(&e),
                Ok(()) => match it.call_main() {
                    Ok(v) => runner::main_result_code(&mut it, &v),
                    Err(e) => {
                        shown = format!("{}\n", e.render());
                        runner::exit_code_for(&e)
                    }
                },
            };
            it.flush();
            Done { stdout: it.get_output(), stderr: it.get_err_output() + &shown, code }
        })
        .unwrap()
        .join()
        .unwrap()
}

const RESUME: &str = "would have to resume";

fn same_or_refused(name: &str, src: &str, seed: u64) {
    let threads = exec(src, false, seed);
    let nested = exec(src, true, seed);
    if nested.stderr.contains(RESUME) {
        return;
    }
    assert_eq!(nested.stdout, threads.stdout, "{name} (schedule {seed}): one-thread output differs\n{}", nested.stderr);
    assert_eq!(nested.code, threads.code, "{name} (schedule {seed}): exit status differs\n{}", nested.stderr);
}

/// The program of the fence that opens at `line` (1-based) of a book page.
fn book_fence(page: &str, line: usize) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/book/src").join(page);
    let text = fs::read_to_string(&path).unwrap();
    let body: Vec<&str> = text.lines().skip(line).take_while(|l| !l.starts_with("```")).collect();
    assert!(text.lines().nth(line - 1).is_some_and(|l| l.starts_with("```wj")), "{page}:{line} is not a wj fence");
    body.join("\n") + "\n"
}

#[test]
fn the_book_samples_that_start_tasks_run_on_one_thread_with_the_thread_output() {
    for (page, line) in [
        ("showcase/side-by-side.md", 200),
        ("tutorial/async.md", 24),
        ("tutorial/async.md", 60),
        ("tutorial/concurrency.md", 18),
        ("tutorial/concurrency.md", 40),
    ] {
        let src = book_fence(page, line);
        let threads = exec(&src, false, 0);
        let nested = exec(&src, true, 0);
        assert_eq!(threads.code, 0, "{page}:{line} thread mode: {}", threads.stderr);
        assert_eq!(nested.code, 0, "{page}:{line} one-thread mode: {}", nested.stderr);
        assert_eq!(nested.stdout, threads.stdout, "{page}:{line}");
        assert!(!threads.stdout.is_empty());
    }
}

#[test]
fn the_synchronised_corpus_program_agrees_in_both_modes() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/interpreter_corpus");
    for seed in [3u64, 21] {
        let src = fs::read_to_string(dir.join("threads_sync.wj")).unwrap().replace("__SEED__", &seed.to_string());
        let want = fs::read_to_string(dir.join(format!("threads_sync.vectors/{seed}.out"))).unwrap();
        let nested = exec(&src, true, 0);
        assert_eq!(nested.stdout, want, "{}", nested.stderr);
        same_or_refused("threads_sync", &src, 0);
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

const SLEEPERS: &str = r#"
use std::sync::mpsc
use std::thread
use std::time::Duration

fn main() {
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
}
"#;

const PRODUCER: &str = r#"
use std::sync::mpsc
use std::thread

fn main() {
    let (tx, rx) = mpsc::channel()
    let h = thread::spawn(move || {
        let mut i = 0
        while i < 3 {
            tx.send(i)
            i += 1
        }
        return 7
    })
    for v in rx { println("got {}", v) }
    println("done {}", h.join().unwrap_or(0))
}
"#;

const PANICKER: &str = r#"
use std::thread

fn main() {
    let h = thread::spawn(move || {
        panic("boom")
        return 1
    })
    match h.join() {
        Ok(v) => println("ok {}", v),
        Err(_) => println("task panicked"),
    }
}
"#;

#[test]
fn other_programs_agree_or_say_that_a_task_would_have_to_resume() {
    for seed in [0u64, 1, 2, 5] {
        same_or_refused("race", RACE, seed);
        same_or_refused("sleepers", SLEEPERS, seed);
        same_or_refused("producer", PRODUCER, seed);
        same_or_refused("panicker", PANICKER, seed);
    }
    assert_eq!(exec(RACE, true, 0).stdout, "final 2\n");
    assert_eq!(exec(PRODUCER, true, 0).stdout, "got 0\ngot 1\ngot 2\ndone 7\n");
    assert_eq!(exec(PANICKER, true, 0).stdout, "task panicked\n");
}

#[test]
fn a_schedule_that_needs_a_suspended_task_is_refused_not_reordered() {
    // three sleepers: the second to start wakes first, but the first to start is below it on the stack
    let nested = exec(SLEEPERS, true, 0);
    assert!(nested.stderr.contains(RESUME), "{}", nested.stderr);
    assert_ne!(nested.code, 0);
    assert_eq!(exec(SLEEPERS, false, 0).stdout, "woke 100\nwoke 200\nwoke 300\n");
}

/// The ADR-0004 conformance programs (`spawn`/`async`/`scope`, channel methods) in the playground's mode: the same
/// output as with threads, or the error that says a task would have to resume, never a different output.
#[test]
fn the_concurrency_conformance_programs_agree_or_say_that_a_task_would_have_to_resume() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/concurrency");
    let mut n = 0;
    for e in fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "wj") {
            let src = fs::read_to_string(&p).unwrap();
            same_or_refused(&p.display().to_string(), &src, 0);
            n += 1;
        }
    }
    assert!(n >= 4, "conformance programs missing");
}
