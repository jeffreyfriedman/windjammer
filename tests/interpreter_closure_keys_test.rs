//! The resolved engine finds the locals a closure or island names from the source text of its AST (`Debug`). That text
//! must stay exact (same closures, same environments) and must not carry the source file's path, whose length would
//! otherwise change the interpreter's cost (see src/source_map.rs `elided_debug`).
//!
//! Closures with identical bodies at different spans capturing different locals, closures in loops, recursion and a second
//! file are each run from two working directories of very different path lengths under both engines.

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const EXPECTED: &str = "2 101\n2 6\n1 11 21 31 \n4 31\n9\n1001 8\n6\n";

fn run_in(dir: &Path, engine: &str) -> String {
    let o = Command::new(WJ)
        .args(["run", "--interpret", "--no-lint", "--engine", engine, "main.wj"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn copy_program(into: &Path) {
    std::fs::create_dir_all(into).unwrap();
    for f in ["main.wj", "two_files.wj"] {
        std::fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/closure_keys").join(f), into.join(f)).unwrap();
    }
}

#[test]
fn identical_closure_bodies_keep_their_own_environments_at_any_path_length() {
    let root = std::env::temp_dir().join(format!("wj-closure-keys-{}", std::process::id()));
    let short = root.join("a");
    let long = root.join("a_very_long_directory_name_that_is_much_longer_than_the_short_one_by_dozens_of_characters");
    copy_program(&short);
    copy_program(&long);
    for dir in [&short, &long] {
        for engine in ["resolved", "walker"] {
            assert_eq!(run_in(dir, engine), EXPECTED, "engine {engine} in {}", dir.display());
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn location_debug_elides_the_path_only_on_request() {
    use windjammer::source_map::{elided_debug, Location};
    let l = Location { file: "/very/secret/dir/f.wj".into(), line: 3, column: 4 };
    let full = format!("{:?}", l);
    assert!(full.contains("/very/secret/dir/f.wj") && full.contains("line: 3") && full.contains("column: 4"), "{full}");
    let elided = elided_debug(&l);
    assert!(!elided.contains("secret") && elided.contains("line: 3") && elided.contains("column: 4"), "{elided}");
    assert_eq!(format!("{:?}", l), full, "the elision is a property of the call, not of the process");
    // It reaches Locations nested in containers, next to values whose Debug could be sensitive to the precision.
    let nested = (vec![Some(l.clone())], true, false, 1.5f64, "s", 7usize);
    let text = elided_debug(&nested);
    assert!(!text.contains("secret") && text.contains("true") && text.contains("false") && text.contains("\"s\""), "{text}");
    assert!(format!("{:?}", nested).contains("secret"));
}
