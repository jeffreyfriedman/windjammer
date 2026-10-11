# Rust target: tail `None` is emitted as `None as i32`

```windjammer
fn find(v: Vec<i32>, t: i32) -> Option<i32> {
    for x in v {
        if x == t {
            return Some(x)
        }
    }
    None
}
```

emits `None as i32` as the function tail -> `E0308: expected Option<i32>, found i32`. Affected: corpus programs 18, 68.
