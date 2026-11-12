//! Compiled-build verdicts keyed on content and compiler build (`cli::verdict`, ADR-0033): the key follows
//! every input and the compiler, entries round-trip and stay bounded, a cargo failure without a rustc error
//! is no verdict, and an interpreted run of content whose compiled build failed says so on stderr only.

use std::path::Path;
use windjammer::cli::verdict::{self, cargo_json_outcome, key, load_from, lookup_in, record_in, store_in, Outcome, Verdict};

fn inputs(text: &str) -> Vec<(String, String)> {
    vec![("/p/main.wj".to_string(), text.to_string())]
}

#[test]
fn key_follows_content_and_compiler() {
    let a = key(&inputs("fn main() {}"), "0.50.0:1");
    assert_eq!(a, key(&inputs("fn main() {}"), "0.50.0:1"));
    assert_ne!(a, key(&inputs("fn main() { }"), "0.50.0:1"), "an edit is a new program");
    assert_ne!(a, key(&inputs("fn main() {}"), "0.50.0:2"), "another compiler build is a new verdict");
    // length prefixes: moving a byte between path and text is another input
    assert_ne!(key(&[("ab".into(), "c".into())], "c"), key(&[("a".into(), "bc".into())], "c"));
}

#[test]
fn verdicts_round_trip_and_wrong_or_damaged_keys_miss() {
    let d = tempfile::tempdir().unwrap();
    let k = key(&inputs("x"), "c");
    let v = Verdict::new(&k, Outcome::Rejected { errors: 2, first: Some("WJ0003 mismatch at main.wj:3".into()) }, false);
    store_in(d.path(), &v, 8);
    assert_eq!(load_from(d.path(), &k), Some(v));
    assert_eq!(load_from(d.path(), &key(&inputs("y"), "c")), None);
    assert_eq!(load_from(d.path(), "../../etc/passwd"), None, "only hex digests name entries");
    std::fs::write(d.path().join(format!("{k}.json")), "{ not json").unwrap();
    assert_eq!(load_from(d.path(), &k), None, "a damaged entry is a miss");
}

#[test]
fn the_directory_keeps_the_newest_entries() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..5 {
        let k = key(&inputs(&i.to_string()), "c");
        store_in(d.path(), &Verdict::new(&k, Outcome::Accepted, false), 3);
        // distinct modification times order the entries
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    // the directory also holds its lock file (writers hold it so `wj cache prune` leaves them alone)
    assert_eq!(std::fs::read_dir(d.path()).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "json")).count(), 3);
    assert!(load_from(d.path(), &key(&inputs("4"), "c")).is_some());
    assert!(load_from(d.path(), &key(&inputs("0"), "c")).is_none());
}

#[test]
fn a_cargo_failure_without_a_rustc_error_is_no_verdict() {
    assert_eq!(cargo_json_outcome(true, ""), Some(Outcome::Accepted));
    assert_eq!(cargo_json_outcome(false, "{\"reason\":\"build-finished\",\"success\":false}\nnot json\n"), None);
    let err = r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","code":{"code":"E0308"}}}"#;
    let warn = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused","code":null}}"#;
    let abort = r#"{"reason":"compiler-message","message":{"level":"error","message":"aborting due to 1 previous error","code":null}}"#;
    assert_eq!(
        cargo_json_outcome(false, &format!("{warn}\n{err}\n{abort}\n")),
        Some(Outcome::Rejected { errors: 1, first: Some("rustc E0308 mismatched types".into()) })
    );
}

#[test]
fn a_program_is_one_key_whatever_the_path_spelling_and_an_edit_misses() {
    let d = tempfile::tempdir().unwrap();
    let cache = d.path().join("verdicts");
    let src = d.path().join("main.wj");
    std::fs::write(&src, "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    record_in(&cache, &src, Outcome::Rejected { errors: 1, first: None }, false);
    let dotted = d.path().join(".").join("main.wj");
    assert!(lookup_in(&cache, &dotted).is_some(), "another spelling of the same file");
    std::fs::write(&src, "fn main() {\n    println(\"hi!\")\n}\n").unwrap();
    assert!(lookup_in(&cache, &src).is_none(), "an edit is another program");
}

fn interpret(root: &Path, src: &Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["run", "--interpret"])
        .arg(src)
        .env("WJ_CARGO_TARGET_ROOT", root)
        .output()
        .unwrap()
}

#[test]
fn an_interpreted_run_mentions_a_failed_compiled_build_of_the_same_content() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("cache-root");
    let src = d.path().join("main.wj");
    std::fs::write(&src, "fn main() {\n    println(\"hello\")\n}\n").unwrap();

    let quiet = interpret(&root, &src);
    assert_eq!(String::from_utf8_lossy(&quiet.stdout), "hello\n");
    assert!(!String::from_utf8_lossy(&quiet.stderr).contains("compiled build"), "no verdict, no footer");

    // the entry the `wj` binary would record after a failed `wj build` of this content
    let inputs = windjammer::check::program_inputs(&src).unwrap();
    let k = key(&inputs, &windjammer::check::compiler_identity_of(Path::new(env!("CARGO_BIN_EXE_wj"))));
    let first = Some("rustc E0308 mismatched types".to_string());
    store_in(&root.join("verdicts"), &Verdict::new(&k, Outcome::Rejected { errors: 1, first }, false), verdict::MAX_ENTRIES);

    let noted = interpret(&root, &src);
    assert_eq!(String::from_utf8_lossy(&noted.stdout), "hello\n", "stdout stays the program's");
    assert_eq!(noted.status.code(), Some(0));
    let err = String::from_utf8_lossy(&noted.stderr);
    assert!(err.contains("the compiled build of this exact program failed (1 error; first: rustc E0308"), "{err}");

    std::fs::write(&src, "fn main() {\n    println(\"hello again\")\n}\n").unwrap();
    let edited = interpret(&root, &src);
    assert!(!String::from_utf8_lossy(&edited.stderr).contains("compiled build"), "the verdict was for other content");
}
