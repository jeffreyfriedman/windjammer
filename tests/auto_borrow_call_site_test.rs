#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! TDD: Auto-borrow at method/function call sites.
//!
//! These patterns are currently broken in the Windjammer compiler:
//! - Chained instance methods should auto-borrow string params, not `.clone()`
//! - String literals must coerce to owned `String` or match `&str` formals
//! - Copy-type HashMap values must not generate `*value.clone()`
//! - `Vec::new()` args must auto-borrow when the formal is `&Vec<T>`

#[path = "common/test_utils.rs"]
mod test_utils;

use test_utils::{compile_single, compile_single_check};

/// Instance method taking `string` used only for read (HashSet::contains) should
/// lower to a borrowed formal; call sites must not `.clone()` the argument.
#[test]
fn test_instance_method_string_param_auto_borrow() {
    let source = r#"
pub struct Registry {
    names: HashSet<string>,
}

impl Registry {
    pub fn is_registered(self, name: string) -> bool {
        self.names.contains(name)
    }

    pub fn check(self, name: string) {
        if self.is_registered(name) {
            let _ = 1
        }
    }
}
"#;

    let generated = compile_single(source);

    assert!(
        generated.contains("name: &String") || generated.contains("name: &str"),
        "is_registered formal should be borrowed (&String or &str). Generated:\n{generated}"
    );

    assert!(
        !generated.contains("is_registered(name.clone())"),
        "check → is_registered call site must not clone name. Generated:\n{generated}"
    );

    assert!(
        generated.contains("is_registered(name)")
            || generated.contains("is_registered(&name)")
            || generated.contains(".contains(name)")
            || generated.contains(".contains(&name)"),
        "call site should pass name by borrow or direct use. Generated:\n{generated}"
    );

    let (generated, compiles) = compile_single_check(source);
    assert!(
        compiles,
        "Registry auto-borrow pattern should compile.\nGenerated:\n{generated}"
    );
}

/// String literal passed to a `string` formal must either allocate (`.to_string()`)
/// or the formal lowers to `&str` and the literal passes through unchanged.
#[test]
fn test_string_literal_to_string_coercion() {
    let source = r#"
pub struct Named {
    name: string,
}

impl Named {
    pub fn set_name(self, name: string) {
        self.name = name
    }
}

pub fn test_set() {
    let mut obj = Named { name: "".to_string() }
    obj.set_name("hello")
}
"#;

    let generated = compile_single(source);

    let literal_coerced_to_owned = generated.contains("set_name(\"hello\".to_string()")
        || generated.contains("set_name(\"hello\".into()");

    let formal_is_str_ref = (generated.contains("name: &str") || generated.contains("name: &String"))
        && generated.contains("set_name(\"hello\")")
        && !generated.contains("\"hello\".to_string()");

    assert!(
        literal_coerced_to_owned || formal_is_str_ref,
        "string literal must coerce to owned String or formal must be &str. Generated:\n{generated}"
    );

    let (generated, compiles) = compile_single_check(source);
    assert!(
        compiles,
        "string literal coercion pattern should compile.\nGenerated:\n{generated}"
    );
}

/// Copy-type tuple values from HashMap::get must push without `*parent.clone()`.
#[test]
fn test_copy_type_no_deref_clone() {
    let source = r#"
pub fn test_copy_no_deref() {
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new()
    came_from.insert((0, 0), (1, 1))
    let mut path: Vec<(i32, i32)> = Vec::new()
    if let Some(parent) = came_from.get((0, 0)) {
        path.push(parent)
    }
}
"#;

    let generated = compile_single(source);

    assert!(
        !generated.contains("*parent.clone()"),
        "Copy tuple from HashMap::get must not generate *parent.clone(). Generated:\n{generated}"
    );

    assert!(
        !generated.contains("path.push(parent.clone())"),
        "Copy tuple ref must not clone at push site — use *parent or direct copy. Generated:\n{generated}"
    );

    assert!(
        generated.contains("path.push(parent)") || generated.contains("path.push(*parent)"),
        "Copy value should push as parent or *parent. Generated:\n{generated}"
    );

    let (generated, compiles) = compile_single_check(source);
    assert!(
        compiles,
        "Copy-type HashMap get + Vec push should compile.\nGenerated:\n{generated}"
    );
}

/// `Vec::new()` passed to a method whose formals infer as borrowed must get `&` at the call site.
#[test]
fn test_register_method_vec_param_auto_borrow() {
    let source = r#"
pub struct Scheduler {
    tasks: Vec<string>,
}

impl Scheduler {
    pub fn register(self, name: string, deps: Vec<string>, excludes: Vec<string>) {
        self.tasks.push(name)
    }
}

pub fn test_register() {
    let mut s = Scheduler { tasks: Vec::new() }
    s.register("render".to_string(), Vec::new(), Vec::new())
}
"#;

    let (generated, compiles) = compile_single_check(source);
    assert!(
        compiles,
        "Scheduler::register with Vec::new() args should compile.\nGenerated:\n{generated}"
    );

    let register_lines: Vec<&str> = generated
        .lines()
        .filter(|l| l.contains(".register("))
        .collect();

    assert!(
        !register_lines.is_empty(),
        "expected a register() call site. Generated:\n{generated}"
    );

    for line in &register_lines {
        let vec_new_count = line.matches("Vec::new()").count()
            + line.matches("Vec::<String>::new()").count()
            + line.matches("Vec::<std::string::String>::new()").count();
        let borrowed_vec_new = line.matches("&Vec::new()").count()
            + line.matches("&Vec::<String>::new()").count()
            + line.matches("&Vec::<std::string::String>::new()").count();
        let unborrowed = vec_new_count.saturating_sub(borrowed_vec_new);

        if generated.contains("deps: &Vec") || generated.contains("excludes: &Vec") {
            assert_eq!(
                unborrowed, 0,
                "Vec::new() args must be auto-borrowed when formal is &Vec<T>.\n\
                 Line: {line}\nFull output:\n{generated}"
            );
        }
    }
}
