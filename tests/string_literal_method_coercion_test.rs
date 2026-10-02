#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_string_literal_to_owned_param_in_field_access_call() {
    let code = r#"
struct Config {
    name: string
}

impl Config {
    fn set_name(self, n: string) {
        self.name = n
    }
}

fn main() {
    let mut c = Config { name: "default" }
    c.set_name("hello")
}
"#;
    let rust = test_utils::compile_single(code);
    println!("{}", rust);
    // Tip may demote read-only/setter `n: string` → `&str` and convert inside the body.
    let owned_lit = rust.contains(r#""hello".to_string()"#)
        || rust.contains(r#"string::from("hello")"#)
        || rust.contains(r#"String::from("hello")"#);
    let demoted_ok = rust.contains(r#"set_name("hello")"#)
        && (rust.contains("n: &str") || rust.contains("n:&str"));
    assert!(
        owned_lit || demoted_ok,
        "String literal must match set_name formal (owned .to_string() or demoted &str).\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_string_literal_coercion_through_self_assigned_variable() {
    let code = r#"
struct PassBuilder {
    name: string
}

impl PassBuilder {
    fn named_uniform(self, name: string, wjsl_type: string, buffer_id: u32) -> PassBuilder {
        self.name = name
        self
    }

    fn bind_auto_uniforms(self) -> PassBuilder {
        let mut pb = self
        pb = pb.named_uniform("screen_width", "u32", 0)
        pb
    }
}

fn main() {
    let b = PassBuilder { name: "test" }
    let _result = b.bind_auto_uniforms()
}
"#;
    let rust = test_utils::compile_single(code);
    println!("{}", rust);
    let owned_name = rust.contains(r#""screen_width".to_string()"#);
    let demoted_ok = rust.contains(r#"named_uniform("screen_width""#)
        && (rust.contains("name: &str") || rust.contains("name:&str"));
    assert!(
        owned_name || demoted_ok,
        "String literal via Self-assigned var must match named_uniform formal.\nGenerated:\n{}",
        rust
    );
    assert!(
        !rust.contains(r#""u32".to_string()"#) || demoted_ok,
        "Borrowed wjsl_type lit should stay bare &str (or both params demoted).\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_string_literal_to_method_with_multiple_consumed_params() {
    let code = r#"
struct Builder {
    key: string,
    value: string
}

impl Builder {
    fn configure(self, key: string, value: string) {
        self.key = key
        self.value = value
    }
}

fn main() {
    let mut b = Builder { key: "a", value: "b" }
    b.configure("name", "value")
}
"#;
    let rust = test_utils::compile_single(code);
    println!("{}", rust);
    let demoted = rust.contains("key: &str") || rust.contains("key:&str");
    let name_ok = rust.contains(r#""name".to_string()"#)
        || (demoted && rust.contains(r#"configure("name""#));
    let value_ok = rust.contains(r#""value".to_string()"#)
        || (demoted && rust.contains(r#""value")"#));
    assert!(
        name_ok,
        "First configure string arg must match formal (owned or demoted &str).\nGenerated:\n{}",
        rust
    );
    assert!(
        value_ok,
        "Second configure string arg must match formal (owned or demoted &str).\nGenerated:\n{}",
        rust
    );
}
