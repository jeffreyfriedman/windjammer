# Parser: `let v = if (x as usize) < n { .. } else { .. }` fails to parse

```windjammer
fn main() {
    let ci: i64 = 1
    let p: Vec<string> = vec!["a".to_string(), "b".to_string()]
    let v = if (ci as usize) < p.len() { p[ci as usize].clone() } else { "".to_string() }
    println(v)
}
```

`parse error: Expected RParen, got As`. The same condition as a plain `if (ci as usize) < p.len() { .. }` statement parses, and
`if ci < (p.len() as i64)` in the expression form parses. Found by t052.
