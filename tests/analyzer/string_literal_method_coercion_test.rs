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

#[path = "../common/test_utils.rs"]
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
    assert!(
        rust.contains(r#""hello".to_string()"#) || rust.contains(r#"String::from("hello")"#),
        "String literal passed to Owned String param should get .to_string() conversion.\nGenerated:\n{}",
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
    assert!(
        rust.contains(r#""screen_width".to_string()"#),
        "String literal passed via Self-assigned variable should get .to_string() when param is Owned.\nGenerated:\n{}",
        rust
    );
    assert!(
        !rust.contains(r#""u32".to_string()"#),
        "String literal for Borrowed param should NOT get .to_string() (stays as &str).\nGenerated:\n{}",
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
    assert!(
        rust.contains(r#""name".to_string()"#),
        "First consumed string param should get .to_string().\nGenerated:\n{}",
        rust
    );
    assert!(
        rust.contains(r#""value".to_string()"#),
        "Second consumed string param should get .to_string().\nGenerated:\n{}",
        rust
    );
}
