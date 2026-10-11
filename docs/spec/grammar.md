# Grammar and syntax conventions

The normative grammar is `docs/spec/grammar.ebnf`. `tests/spec_grammar_test.rs` reads it, generates sentences, wraps them in a context and feeds them to the real lexer and parser; a disagreement fails the test. This chapter states the conventions that a context-free grammar cannot.

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/grammar/`.

**What the grammar test does and does not prove.** It proves *grammar ⊆ parser*: everything the EBNF derives parses. It does not prove *parser ⊆ grammar*: the parser accepts more than the EBNF lists (Rust-style `&`/`*` unary operators, `as` casts, macro-style calls, ranges in expressions, `for`/`return` as expressions, turbofish, lifetimes). Those extensions are deliberately omitted from the normative grammar unless the language wants to keep them; the negative examples (`mode: parse-error`) police the other direction for a few important cases. Productions marked `@status unparsed` document constructs the old grammar advertised but the parser rejects.

**Old grammar.** `docs/windjammer.ebnf` predates this spec and contains errors (nested block comments, `~`, `\xNN` escapes, range patterns, `loop` as expression, wrong operator precedence). It is superseded by `docs/spec/grammar.ebnf`.

## Normative grammar

**SPEC-GRAM-001** The EBNF in docs/spec/grammar.ebnf is the normative syntax. Every sentence it derives is accepted by the real parser; productions the parser rejects are flagged `unparsed` in the file. (untestable: verified by tests/spec_grammar_test.rs, which generates sentences from the EBNF and parses them)

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

## Line-initial bracket

**SPEC-GRAM-004** [OPEN Q-019] A line that begins with `[` continues the previous expression as an index operation (`f()` newline `[1, 2]` is `f()[1, 2]`). Start such a list expression with `let` or parenthesise it.

<!-- example: tests/spec/grammar/SPEC-GRAM-004.wj -->
```windjammer
fn f() -> int { 5 }
fn main() {
    let a = f()
    [1, 2].len()
}
```
Rejected at compile time.

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
```windjammer
fn main() {
    println("x");
    ;
}
```
Rejected at compile time.

## Labeled loops

**SPEC-GRAM-008** [OPEN Q-020] Loop labels (`'outer: for ...`, `break 'outer`) do not exist; `break`/`continue` always target the innermost loop.

<!-- example: tests/spec/grammar/SPEC-GRAM-008.wj -->
```windjammer
fn main() {
    'outer: for i in 0..3 {
        for j in 0..3 {
            if j == 1 { continue 'outer }
        }
    }
}
```
Rejected at compile time.

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

