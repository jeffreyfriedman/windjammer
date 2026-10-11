# Grammar and syntax conventions

The normative grammar is `docs/spec/grammar.ebnf`. The docs-truth check `tools/docs_truth/spec_grammar.rs` (run by `cargo test --release --test docs_truth spec_grammar` and `wj-docs check spec-grammar`) reads it with the shared EBNF reader of `tools/docs_truth/ebnf.rs`, derives every alternative plus a seeded random corpus, wraps each sentence in the production's `@ctx` template and parses it with the real parser; lexical productions must lex to one token of the stated kind. A disagreement fails the test. This chapter states the conventions that a context-free grammar cannot.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/grammar/`.

**What the grammar test does and does not prove.** It proves *grammar ⊆ parser*: everything the EBNF derives parses. It does not prove *parser ⊆ grammar*: the parser accepts more than the EBNF lists (Rust-style `&`/`*` unary operators, `as` casts, macro-style calls, ranges in expressions, `for`/`return` as expressions, turbofish, lifetimes). Those extensions are deliberately omitted from the normative grammar unless the language wants to keep them; the negative examples (`mode: parse-error`) police the other direction for a few important cases. Productions marked `@status unparsed` document constructs the old grammar advertised but the parser rejects.

**Older grammar.** `docs/windjammer.ebnf` is the earlier reference grammar, also checked by docs-truth (`tools/docs_truth/ebnf.rs`). It predates this spec and lists Rust's operator precedence table, which the parser does not implement (see SPEC-EXPR-001); `docs/spec/grammar.ebnf` is normative where they differ.

## Normative grammar

**SPEC-GRAM-001** The EBNF in docs/spec/grammar.ebnf is the normative syntax. Every sentence it derives is accepted by the real parser; productions the parser rejects are flagged `unparsed` in the file. (untestable: verified by the docs-truth check tools/docs_truth/spec_grammar.rs, which derives sentences from the EBNF and parses them)

## Trailing commas

**SPEC-GRAM-002** A trailing comma is allowed in argument lists, parameter lists, struct and enum bodies, struct literals, list literals and tuples.

<!-- example: tests/spec/grammar/SPEC-GRAM-002.wj -->
```windjammer
struct P { x: int, y: int, }
fn f(a: int, b: int,) -> int { a + b }
fn main() {
    let p = P { x: 1, y: 2, }
    let v = [1, 2, 3,]
    let t = (7, 8,)
    println("${f(p.x, p.y,)} ${v.len()} ${t.1}")
}
```
Output:
```text
3 3 8
```

## Optional semicolons

**SPEC-GRAM-003** A `;` may end any statement but is never required; a line that begins with `(` starts a new statement.

<!-- example: tests/spec/grammar/SPEC-GRAM-003.wj -->
```windjammer
fn g(x: int) -> int {
    return x + 1;
}
fn main() {
    let a = 1
    let b = 2;
    (a + b)
    println("${g(a)}");
}
```
Output:
```text
2
```

## Line-initial tokens

**SPEC-GRAM-004** A newline ends a statement unless the next line begins with `.`, a binary operator (`+ - / % == != < <= > >= && || |> ^ << >>`) or `|>`, which continue the expression; a line beginning with `(` or `[` starts a new statement; a line beginning with `*` is a dereference-assignment only when the line contains a depth-0 assignment. Normative table: docs/design/statement-boundaries.md.

<!-- example: tests/spec/grammar/SPEC-GRAM-004.wj -->
<!-- ignore: known divergence Q-019; the example shows the specified behaviour, which the implementation does not have yet -->
```windjammer,ignore
fn f() -> int { 5 }
fn main() {
    let a = f()
    [1, 2].len()
    let total = a
        + 1
    let n = [3, 4]
        .len()
    println("${a} ${total} ${n}")
}
```
Output:
```text
5 6 2
```
> **Known divergence:** the implementation does not do this yet; see [Q-019](OPEN_QUESTIONS.md). The example is marked `xfail`.

## Match arm separators

**SPEC-GRAM-005** Arms are separated by `,`; the comma may be omitted after an arm whose body is a block, and a trailing comma is allowed.

<!-- example: tests/spec/grammar/SPEC-GRAM-005.wj -->
```windjammer
fn main() {
    let n = 2
    match n {
        1 => { println("one") }
        2 => { println("two") }
        _ => println("many"),
    }
}
```
Output:
```text
two
```

## Explicit type arguments

**SPEC-GRAM-006** In expression position generic arguments are written with a leading `::` (turbofish): `f::<int>(3)`, `.collect::<Vec<int>>()`; a bare `<` is always the less-than operator.

<!-- example: tests/spec/grammar/SPEC-GRAM-006.wj -->
```windjammer
fn id<T>(x: T) -> T { x }
fn main() {
    let a = id::<int>(3)
    let v = "ab cd".split(" ").map(|s| s.len()).collect::<Vec<int>>()
    println("${a} ${v.len()} ${a < 4} ${v.len() > 1}")
}
```
Output:
```text
3 2 true true
```

## Empty statements

**SPEC-GRAM-007** A `;` only terminates the statement before it; a second `;` with nothing before it is not a statement.

<!-- example: tests/spec/grammar/SPEC-GRAM-007.wj -->
<!-- should_fail: the rule says this program is rejected -->
```windjammer,should_fail
fn main() {
    println("x");
    ;
}
```
Rejected at compile time.

## Labeled loops

**SPEC-GRAM-008** A loop may carry a label `'name:` before `for`, `while` or `loop`; `break 'name` / `continue 'name` target that loop, and `break 'name v` gives a labeled `loop` its value. Without a label they target the innermost loop.

<!-- example: tests/spec/grammar/SPEC-GRAM-008.wj -->
```windjammer
fn main() {
    'outer: for i in 0..3 {
        for j in 0..3 {
            if j == 1 { continue 'outer }
            println("${i}${j}")
        }
    }
    let v = 'o: loop {
        loop { break 'o 7 }
    }
    println("${v}")
}
```
Output:
```text
00
10
20
7
```

## Struct literal versus block

**SPEC-GRAM-009** `Name { ... }` is a struct literal only when `Name` begins with an upper-case letter; in `if`/`while`/`match` heads a lower-case name followed by `{` always begins the body.

<!-- example: tests/spec/grammar/SPEC-GRAM-009.wj -->
```windjammer
struct P { x: int }
fn main() {
    let ready = true
    if ready { println("body") }
    let p = P { x: 1 }
    match p { P { x } => println("${x}") }
}
```
Output:
```text
body
1
```

