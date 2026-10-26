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

Status: fixed. Ownership decision, made at the binding: a `let` (mutable or not) initialised from a `string` parameter that is emitted as `&str` owns the value (`line.to_string()`; a `.clone()` of such a parameter is still a `&str`). The semantic layer has no per-binding ownership fact for locals yet (ADR-0017 covers parameters and calls), so this is a small rule in `let_statement_generation::own_borrowed_text_param_binding`, consumed where the `let` is emitted; when convergence exposes a local-binding mode fact this rule becomes its consumer. Gates: `ecosystem_findings2::mut_local_from_string_param_takes_owned_strings`, `mut_local_from_string_param_stored_in_struct_field`.
