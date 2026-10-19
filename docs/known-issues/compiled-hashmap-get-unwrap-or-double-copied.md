# Compiled: `map.get(key).unwrap_or(0)` on a `HashMap<string, int>` emits `.copied().copied()`

```windjammer
use std::collections::HashMap

fn lookup(seen: HashMap<string, int>, key: string) -> int {
    seen.get(key).unwrap_or(0)
}

fn main() {
    let mut m = HashMap::new()
    m.insert("a", 1)
    println("${lookup(m, "a")}")
}
```

`wj build` emits `seen.get(&key).copied().copied().unwrap_or(0_i64)` (`E0599: no method named copied found for enum
Option<i64>`). Only one `.copied()` is valid.

Status: fixed. Exactly one `.copied()`, and no leading `*` on the by-value result. Gate: `ecosystem_findings_test::hashmap_get_unwrap_or_literal_copies_once`.
