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

//! TDD tests for Copy-type auto-borrow bugs.
//!
//! Bug: when a Copy-type parameter is used in both mutating and non-mutating
//! call paths, the compiler auto-borrows to &mut for the mutating path but
//! then passes &mut to functions that expect owned (Copy) values.
//!
//! Root cause: ownership inference propagates &mut upward through the call
//! chain, but codegen doesn't insert *param (deref-copy) at call sites
//! expecting owned T when the local is &mut T and T: Copy.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Reproduces the dogfood pattern: a routing function passes a Copy deps struct
/// to both read-only handlers (expect owned) and write handlers (inferred &mut).
/// The caller gets &mut deps, but non-mutating call sites need owned.
///
/// Error: E0308 mismatched types: expected `Deps`, found `&mut Deps`
#[test]
fn copy_deps_mut_and_owned_call_paths() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "ports/writer.wj",
        r#"
trait Writer {
    fn write(self, key: string, value: string) -> Result<bool, string>
}
"#,
    );
    test.add_file(
        "ports/reader.wj",
        r#"
trait Reader {
    fn read(self, key: string) -> string
}
"#,
    );
    test.add_file(
        "adapters/seed_writer.wj",
        r#"
use ports::writer::Writer

pub struct SeedWriter {}

impl Writer for SeedWriter {
    fn write(self, key: string, value: string) -> Result<bool, string> {
        Ok(true)
    }
}
"#,
    );
    test.add_file(
        "adapters/seed_reader.wj",
        r#"
use ports::reader::Reader

pub struct SeedReader {}

impl Reader for SeedReader {
    fn read(self, key: string) -> string {
        "default"
    }
}
"#,
    );
    test.add_file(
        "composition/deps.wj",
        r#"
use adapters::seed_writer::SeedWriter
use adapters::seed_reader::SeedReader

pub struct AppDeps {
    pub writer: SeedWriter,
    pub reader: SeedReader,
}

pub fn default_deps() -> AppDeps {
    AppDeps { writer: SeedWriter {}, reader: SeedReader {} }
}
"#,
    );
    test.add_file(
        "composition/handlers.wj",
        r#"
use ports::reader::Reader
use composition::deps::AppDeps

pub fn read_value(deps: AppDeps, key: string) -> string {
    deps.reader.read(key)
}
"#,
    );
    test.add_file(
        "composition/write_handler.wj",
        r#"
use ports::writer::Writer
use composition::deps::AppDeps

pub fn write_value(deps: AppDeps, key: string, value: string) -> Result<bool, string> {
    deps.writer.write(key, value)
}
"#,
    );
    test.add_file(
        "adapters/routes.wj",
        r#"
use composition::deps::AppDeps
use composition::handlers::read_value
use composition::write_handler::write_value

pub fn handle_request(deps: AppDeps, action: string, body: string) -> string {
    if action == "read" {
        return read_value(deps, body)
    }
    if action == "write" {
        match write_value(deps, "key", body) {
            Ok(_) => return "ok",
            Err(msg) => return msg,
        }
    }
    "unknown"
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod ports {
    pub mod writer
    pub mod reader
}
pub mod adapters {
    pub mod seed_writer
    pub mod seed_reader
    pub mod routes
}
pub mod composition {
    pub mod deps
    pub mod handlers
    pub mod write_handler
}
"#,
    );

    test.assert_compiles_without_error();

    let map = test.compile().expect("compile map");
    let routes = map.get("adapters/routes.rs").expect("adapters/routes.rs");
    assert!(
        !routes.contains("&mut AppDeps"),
        "Copy AppDeps must not become &mut in routing function. Got:\n{routes}"
    );
}

/// Simpler case: a single function calls two functions, one that mutates
/// a Copy-type field and one that reads. The caller's parameter should
/// NOT become &mut - it should stay owned and be copied at each call site.
#[test]
fn copy_type_field_method_read_and_write() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "main.wj",
        r#"
struct Counter {
    value: i64,
}

impl Counter {
    fn get(self) -> i64 {
        self.value
    }
    fn increment(self) -> i64 {
        self.value = self.value + 1
        self.value
    }
}

struct Deps {
    counter: Counter,
}

fn read_counter(deps: Deps) -> i64 {
    deps.counter.get()
}

fn bump_counter(deps: Deps) -> i64 {
    deps.counter.increment()
}

pub fn process(deps: Deps) -> i64 {
    let a = read_counter(deps)
    let b = bump_counter(deps)
    a + b
}
"#,
    );
    let result = test.compile();
    match result {
        Ok(map) => {
            let code = map.get("main.rs").unwrap_or_else(|| {
                panic!("main.rs not found. Keys: {:?}", map.keys().collect::<Vec<_>>())
            });
            assert!(
                !code.contains("&mut deps"),
                "Copy Deps must not become &mut at call site. Generated:\n{code}"
            );
        }
        Err(e) => panic!("Compilation failed:\n{e}"),
    }
}
