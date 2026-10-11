# Interpreter: a binding to `entry().or_insert(..)` does not alias the map value

```windjammer
use std::collections::HashMap

fn main() {
    let mut g: HashMap<string, Vec<string>> = HashMap::new()
    g.entry("a".to_string()).or_insert(Vec::new()).push("x".to_string())
    let list = g.entry("a".to_string()).or_insert(Vec::new())
    list.push("y".to_string())
    match g.get("a") {
        Some(v) => { println("${v.len()}") }
        None => { println("none") }
    }
}
```

Expected `2`; older interpreter binaries print `1` (the push through `list` is lost; the chained form works). Fixed on the
interp-gaps branch. Found by `bench/agent-dx` t021 (anagram groups).
