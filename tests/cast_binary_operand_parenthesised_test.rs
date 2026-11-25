// Regression: a binary-expression operand of `as` keeps its parentheses in the generated
// Rust, in every position (let initialiser, index, nested cast, compound operand), so
// `(i * 3) as usize` is never printed as `i * 3 as usize` (which rustc and WJ0003 reject).

#[path = "common/test_utils.rs"]
mod test_utils;

fn generated(source: &str) -> String {
    test_utils::compile_single_result(source).expect("compiles")
}

const SRC: &str = r#"
fn pick(data: Vec<i32>, i: i32, j: i32) -> i32 {
    let base = (i * 3) as usize
    let a = data[(i * 3) as usize]
    let b = data[((i * 3) + j) as usize]
    let c = data[(i * 3 + j * 2) as usize + 1]
    let d = ((i * 3) as usize + (j * 2) as usize) as i32
    let e = ((i + j) * 2) as f32
    a + b + c + d + (e as i32) + data[base]
}

fn main() {
    println("${pick([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 1, 1)}")
}
"#;

#[test]
fn binary_operand_of_cast_keeps_parentheses() {
    let rs = generated(SRC);
    for want in [
        "let base = (i * 3_i32) as usize;",
        "data[(i * 3_i32) as usize]",
        "data[(i * 3_i32 + j) as usize]",
        "data[(i * 3_i32 + j * 2_i32) as usize + 1]",
        "((i * 3_i32) as usize + (j * 2_i32) as usize) as i32",
        "((i + j) * 2_i32) as f32",
    ] {
        assert!(rs.contains(want), "missing `{want}` in:\n{rs}");
    }
}
