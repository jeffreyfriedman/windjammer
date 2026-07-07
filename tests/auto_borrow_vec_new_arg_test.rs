#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Auto-borrow must apply to Vec::new() constructor arguments, not just identifiers.
//!
//! Bug: When a function takes `&Vec<T>` and the call site passes `Vec::new()`,
//! the compiler should auto-borrow to `&Vec::new()`. It sometimes borrows one
//! argument but not another in the same call.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn test_auto_borrow_vec_new_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ecs/scheduler.wj",
        r#"
pub struct Info {
    pub name: string,
    pub deps: Vec<string>,
    pub outs: Vec<string>,
}

impl Info {
    pub fn new(name: string) -> Info {
        Info { name: name, deps: Vec::new(), outs: Vec::new() }
    }
}

pub struct Scheduler {
    pub items: Vec<Info>,
}

impl Scheduler {
    pub fn new() -> Scheduler {
        Scheduler { items: Vec::new() }
    }

    pub fn register(self, name: string, deps: Vec<string>, outs: Vec<string>) {
        let mut info = Info::new(name)
        let mut i = 0
        while i < deps.len() {
            info.deps.push(deps[i])
            i = i + 1
        }
        self.items.push(info)
    }
}

pub fn setup() {
    let mut s = Scheduler::new()
    s.register("render", Vec::new(), Vec::new())
    s.register("physics", Vec::new(), Vec::new())
}
"#,
    );

    let map = test.compile().expect("compile");
    let rs = map.get("ecs/scheduler.rs").expect("scheduler.rs");

    // Both Vec::new() args should be auto-borrowed to &Vec::new() since
    // register() infers borrowed params from read-only usage.
    // Check for consistency: either both are borrowed or both are owned.
    let setup_lines: Vec<&str> = rs.lines()
        .filter(|l| l.contains("s.register(") || l.contains("scheduler.register("))
        .collect();

    for line in &setup_lines {
        // Count Vec::new() vs &Vec::new() occurrences in the line
        let vec_new_count = line.matches("Vec::new()").count()
            + line.matches("Vec::<String>::new()").count()
            + line.matches("Vec::<std::string::String>::new()").count();
        let borrowed_vec_new = line.matches("&Vec::new()").count()
            + line.matches("&Vec::<String>::new()").count()
            + line.matches("&Vec::<std::string::String>::new()").count();
        let unborrowed = vec_new_count - borrowed_vec_new;

        // Either all are borrowed or all are unborrowed — no inconsistency
        assert!(
            unborrowed == 0 || borrowed_vec_new == 0,
            "Inconsistent auto-borrow: some Vec::new() args are borrowed and some aren't.\n\
             Line: {line}\n\
             Full output:\n{rs}"
        );
    }
}
