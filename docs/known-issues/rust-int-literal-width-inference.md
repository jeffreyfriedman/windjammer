# Rust target: unsuffixed integer literals get the wrong width

Found by the backend conformance corpus (`tests/conformance_programs`). The Go and JS backends infer the width from use
(`v[i]`, `.len()`, assignment targets); the Rust backend picks a width per literal and rustc rejects the result.

Minimal repro (`wj build --target rust`):

```windjammer
fn main() {
    let mut v = vec![1, 2, 3]
    v[1] = 9
    println("{} {} {}", v[0], v[1], v[2])
}
```

generates `let mut v = vec![1_i32, 2_i32, 3_i32]; v[1] = 9_i64;` -> `E0308: expected i32, found i64`.

Other shapes: `let x: i64 = ...; x * 31 + c` mixing `i32` literals, `shift count` literals against `u32`, `Vec<i64>` collected from an `i32`
iterator. Affected corpus programs are listed in `tests/conformance_programs/SUPPORT.toml` with `rust-int-literal-width-inference`.
