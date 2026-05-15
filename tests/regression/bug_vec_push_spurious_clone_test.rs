#![cfg(not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
)))]

/// TDD: Vec.push() generates spurious .clone() when vec is later passed to another method
///
/// Bug: When a Vec is declared with `let mut`, pushed to, and later passed to a method,
/// the compiler inserts `.clone()` before `.push()`, causing the push to happen on a
/// temporary clone that is immediately dropped. The original Vec stays empty.
///
/// Discovered: Dogfooding shader_graph.wj detect_cycles() method -- caused runtime panic
/// because `visited` Vec was always empty after push loop.
///
/// Expected Rust:
///   visited.push(false);
///
/// Actual (buggy) Rust:
///   visited.clone().push(false);

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_push_no_spurious_clone() {
    let generated = test_utils::compile_single(
        r#"
fn process(items: Vec<bool>) -> usize {
    items.len()
}

pub fn main() {
    let mut data = Vec::new()
    let mut i = 0
    while i < 5 {
        data.push(false)
        i = i + 1
    }
    let result = process(data)
    println!("{}", result)
}
"#,
    );

    println!("Generated Rust:\n{}", generated);

    assert!(
        !generated.contains("data.clone().push("),
        "BUG: Compiler generated spurious .clone() before .push()!\n\
         Generated code should use `data.push(false)` not `data.clone().push(false)`\n\
         Full output:\n{}",
        generated
    );
}

#[test]
fn test_vec_push_then_index_no_spurious_clone() {
    let generated = test_utils::compile_single(
        r#"
struct Graph {
    passes: Vec<bool>
}

impl Graph {
    fn check(self, items: Vec<bool>) -> bool {
        items[0]
    }

    fn detect(self) {
        let num = self.passes.len()
        let mut visited = Vec::new()
        let mut i: usize = 0
        while i < num {
            visited.push(false)
            i = i + 1
        }
        let mut start = 0
        while start < num {
            if visited[start] == false {
                let r = self.check(visited)
            }
            start = start + 1
        }
    }
}
"#,
    );

    println!("Generated Rust:\n{}", generated);

    assert!(
        !generated.contains("visited.clone().push("),
        "BUG: Compiler generated spurious .clone() before .push()!\n{}",
        generated
    );

    assert!(
        !generated.contains("visited.clone()["),
        "BUG: Compiler generated spurious .clone() before index!\n{}",
        generated
    );
}
