# Parser: a statement starting with `*` after a line ending in `}` or `)` is parsed as multiplication

<!-- should_fail: documents a parser bug: the program is valid but is rejected today; remove the annotation when the bug is fixed -->
```windjammer,should_fail
use std::collections::HashMap

fn main() {
    let mut m: HashMap<string, i64> = HashMap::new()
    let a = "x".to_string()
    let b = if a.len() > 3 { a.clone() } else { "".to_string() }
    *m.entry(a.clone()).or_insert(0) += 1
    println("${m.len()} ${b}")
}
```

`parse error: Unexpected token in expression: PlusAssign`. The same statement is fine when it follows a line ending in `{`.
Found by t055 (log summary). Newline-terminated statements should not continue into a leading unary `*`.
