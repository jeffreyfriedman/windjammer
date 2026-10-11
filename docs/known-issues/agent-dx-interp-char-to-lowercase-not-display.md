# Interpreter: `char::to_lowercase()` result has no `Display`/`to_string`

```windjammer
fn main() {
    let c = 'A'
    let lc: string = c.to_lowercase().to_string()
    println(lc)
}
```

`error: `Iterator` doesn't implement `std::fmt::Display``. In Rust `ToLowercase` implements `Display`, so this prints `a`.
`s.to_lowercase()` on a string works and is used by the benchmark. Found by t018.
