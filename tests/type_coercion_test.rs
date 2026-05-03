/// TDD: Vec.push with Index expressions — Copy elements use plain `vec[idx]` (no `&`, no `*`).
///
/// Non-Copy elements still use `&vec[idx]` or `.clone()` per ownership analysis.
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_push_index_tuple_copy_type() {
    // rev.push(path[0]) when rev: Vec<(i32,i32)>, path: Vec<(i32,i32)>
    // Index returns &(i32,i32), push expects (i32,i32) - need * to dereference
    let source = r#"
pub fn copy_first(path: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut rev = Vec::new()
    rev.push(path[0])
    rev
}

fn main() {
    let path = vec![(1, 2), (3, 4)]
    let rev = copy_first(path)
}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    // Must NOT generate &path[0] - that would be wrong (double ref, E0308)
    assert!(
        !rust.contains("&path[") && !rust.contains("& path["),
        "Should NOT add & for Vec.push(Index) when element is Copy, got:\n{}",
        rust
    );

    // Copy tuple: `path[0]` is already `(i32, i32)` in value position; explicit `*` is E0614.
    assert!(
        !rust.contains("*(path[") && !rust.contains("* (path["),
        "must not emit *(path[0]) for Copy tuple element. Got:\n{}",
        rust
    );
    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_vec_push_index_primitive_copy_type() {
    // buf.push(nums[i]) when buf: Vec<i32>, nums: Vec<i32>
    let source = r#"
pub fn copy_evens(nums: Vec<i32>) -> Vec<i32> {
    let mut buf = Vec::new()
    let mut i = 0
    while i < nums.len() {
        let n = nums[i]
        if n % 2 == 0 {
            buf.push(n)
        }
        i = i + 1
    }
    buf
}

fn main() {
    let nums = vec![1, 2, 3, 4]
    let evens = copy_evens(nums)
}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}

#[test]
fn test_vec_push_index_f32_param() {
    // Same pattern as e0308_index_deref_copy_param but for Vec::push
    let source = r#"
pub fn collect_floats(vals: Vec<f32>) -> Vec<f32> {
    let mut out = Vec::new()
    let mut i = 0
    while i < vals.len() {
        out.push(vals[i])
        i = i + 1
    }
    out
}

fn main() {
    let v: Vec<f32> = vec![1.0, 2.0, 3.0]
    let _ = collect_floats(v)
}
"#;

    let (rust, compiles) = test_utils::compile_single_check(source);

    assert!(compiles, "Generated Rust must compile:\n{}", rust);
}
