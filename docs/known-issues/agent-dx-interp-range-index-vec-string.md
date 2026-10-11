# Interpreter: `x[a..b]` panics for `Vec` and `String` ("cannot index Vec with Range")

```windjammer
fn main() {
    let v: Vec<i64> = vec![1, 2, 3, 4]
    let w = v[1..3].to_vec()
    println("${w.len()}")
    let s = "hello world".to_string()
    println(s[0..5].to_string())
}
```

Both lines panic in `wj run --interpret` (compiled Rust accepts them). `v[1..]` (open range) works; `s.as_str()[0..5]` works.
Workaround used in the benchmark solutions: `.as_str()` before slicing strings. Found by t019, t025, t027, t031, t035, t042.
