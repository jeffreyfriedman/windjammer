#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

#[path = "common/test_utils.rs"]
mod test_utils;

/// `self.vec[i].method()` on non-Copy elements must clone before owned-receiver methods (E0507).
#[test]
fn test_index_owned_method_receiver_clones() {
    let source = r##"
pub enum Value {
    Float(f32),
    Text(string),
    None,
}

impl Value {
    pub fn as_float(self) -> f32 {
        match self {
            Value::Float(v) => v,
            _ => 0.0,
        }
    }
}

pub struct Node {
    inputs: Vec<Value>,
}

impl Node {
    pub fn read_input(self) -> f32 {
        self.inputs[0].as_float()
    }

    pub fn get_output(self, index: usize) -> Value {
        self.inputs[index]
    }
}
"##;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains(".clone().as_float()"),
        "owned method on indexed non-Copy element must clone. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("inputs[index]") && generated.contains(".clone()"),
        "return of indexed non-Copy element must clone. Got:\n{}",
        generated
    );
}
