//! Interpreter feature lists for `docs/INTERPRETER.md`, generated from `src/interpreter/features.rs`
//! (whose entries are executed by `tests/interpreter_features_test.rs`).

use windjammer::interpreter::features::{Status, FEATURES};

fn list(status: Status) -> String {
    let mut areas: Vec<&str> = Vec::new();
    for f in FEATURES.iter().filter(|f| f.status == status) {
        if !areas.contains(&f.area) {
            areas.push(f.area);
        }
    }
    let mut s = String::new();
    for a in areas {
        s.push_str(&format!("- **{a}**: "));
        let items: Vec<&str> = FEATURES.iter().filter(|f| f.status == status && f.area == a).map(|f| f.summary).collect();
        s.push_str(&items.join("; "));
        s.push('\n');
    }
    s
}

/// Region body: what works.
pub fn supported() -> String {
    list(Status::Supported)
}

/// Region body: what is an `unsupported in the interpreter` error.
pub fn unsupported() -> String {
    list(Status::Unsupported)
}

pub fn count(status: Status) -> u64 {
    FEATURES.iter().filter(|f| f.status == status).count() as u64
}
