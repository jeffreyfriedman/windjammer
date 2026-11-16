//! An interpreted program never changes the working directory of the process that runs it. The directory is
//! process-wide: a program calling `fs::set_current_dir` inside a test binary used to pull it from under every
//! parallel test ("Could not locate working directory"). The program keeps its own directory (relative paths,
//! `current_dir`, spawned processes follow it); the host keeps its own. A sampler thread watches the host directory
//! while programs run beside a compile job; the sampler is itself checked against a real chdir (mutation control).

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windjammer::interpreter::Interpreter;

fn run_program(dir: &Path, src: &str) -> String {
    let file = dir.join("prog.wj");
    std::fs::write(&file, src.replace("{DIR}", &dir.to_string_lossy())).unwrap();
    let mut it = Interpreter::new_capturing();
    it.load_program_at(&file).expect("loads");
    it.call_main().expect("runs");
    it.get_output()
}

/// Runs `f` while a thread samples the process working directory; `true` when it never moved from `home`.
fn cwd_held_during(home: &Path, f: impl FnOnce()) -> bool {
    let stop = Arc::new(AtomicBool::new(false));
    let moved = Arc::new(AtomicBool::new(false));
    let (s, m, h) = (stop.clone(), moved.clone(), home.to_path_buf());
    let watcher = std::thread::spawn(move || {
        while !s.load(Ordering::Relaxed) {
            if std::env::current_dir().ok().as_deref() != Some(h.as_path()) {
                m.store(true, Ordering::Relaxed);
            }
            std::thread::yield_now();
        }
    });
    f();
    stop.store(true, Ordering::Relaxed);
    watcher.join().unwrap();
    !moved.load(Ordering::Relaxed)
}

const CHDIR_PROGRAM: &str = "use std::fs\n\nfn main() {\n    let r = fs::set_current_dir(\"{DIR}\")\n    match r {\n        Ok(_) => println(\"changed\"),\n        Err(e) => println(\"chdir failed: ${e}\"),\n    }\n    let _ = fs::write(\"made.txt\", \"hi\")\n    match fs::current_dir() {\n        Ok(d) => println(\"cwd=${d}\"),\n        Err(e) => println(\"err ${e}\"),\n    }\n}\n";

fn home() -> PathBuf {
    std::env::current_dir().expect("the test starts in a directory that exists")
}

#[test]
fn a_program_that_changes_directory_leaves_the_process_directory_alone() {
    let d = Scratch::new("vcwd");
    let _cwd = windjammer::test_utils::hold_cwd();
    let home = home();
    let mut out = String::new();
    let held = cwd_held_during(&home, || out = run_program(d.path(), CHDIR_PROGRAM));
    assert!(held, "the process working directory moved while an interpreted program ran");
    assert_eq!(std::env::current_dir().unwrap(), home);
    // the program's own view: its directory moved, relative writes landed there
    assert!(out.contains("changed"), "{out}");
    assert!(out.contains(&format!("cwd={}", d.path().display())), "{out}");
    assert_eq!(std::fs::read_to_string(d.join("made.txt")).unwrap(), "hi");
    assert!(!home.join("made.txt").exists(), "a relative write must not land in the process directory");
}

#[test]
fn a_program_without_chdir_sees_the_process_directory() {
    let d = Scratch::new("vcwd-plain");
    let _cwd = windjammer::test_utils::hold_cwd();
    let out = run_program(d.path(), "use std::fs\n\nfn main() {\n    match fs::current_dir() {\n        Ok(d) => println(\"cwd=${d}\"),\n        Err(e) => println(\"err ${e}\"),\n    }\n}\n");
    assert!(out.contains(&format!("cwd={}", home().display())), "{out}");
}

/// Mutation control: the sampler does see a real chdir, so a regression to `std::env::set_current_dir` in the
/// interpreter would fail the first test.
#[test]
fn the_sampler_notices_a_real_chdir() {
    let d = Scratch::new("vcwd-mutant");
    let home = {
        let _cwd = windjammer::test_utils::hold_cwd();
        home()
    };
    let held = cwd_held_during(&home, || {
        windjammer::test_utils::in_dir(d.path(), || std::thread::sleep(std::time::Duration::from_millis(50)));
    });
    assert!(!held, "the watcher missed a real working-directory change");
}

/// The source scan: nothing under `src/interpreter` changes the host's working directory.
#[test]
fn the_interpreter_never_calls_the_hosts_chdir() {
    let needle = ["env::set_current_", "dir("].concat();
    let mut bad = Vec::new();
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src/interpreter")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                for (i, l) in text.lines().enumerate() {
                    if l.contains(&needle) && !l.trim_start().starts_with("//") {
                        bad.push(format!("{}:{}", p.display(), i + 1));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "interpreted programs keep a virtual working directory (virtual_cwd.rs): {bad:?}");
}
