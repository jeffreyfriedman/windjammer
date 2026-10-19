# Compiled: `let mut body = line` (a `string` parameter) is typed `&str`, so assigning a `String` to it fails

```windjammer
use std::strings

struct Rule {
    pattern: string,
    negated: bool,
}

fn parse_rule(line: string) -> Rule {
    let mut body = line
    let mut negated = false
    if strings.starts_with(body, "!") {
        negated = true
        body = strings.substring(body, 1, body.len())
    }
    Rule { pattern: body, negated: negated }
}

fn main() {
    let r = parse_rule("!a")
    println("${r.pattern} ${r.negated}")
}
```

`line` is inferred `&str`, `body` becomes `line.clone()` (still `&str`), and the assignment of `strings::substring(..)`
(a `String`) fails with `E0308: expected &str, found String`; the struct field then fails too. A mutable local that is
reassigned with owned strings must be an owned `String` (`line.to_string()`), whatever the parameter mode is.
