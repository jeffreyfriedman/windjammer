# Lexical structure

How source text becomes tokens. Windjammer source is UTF-8. Whitespace separates tokens; a newline ends a statement (see GRAM-003).

Part of the [Windjammer language specification](README.md). Rule IDs are checked by `scripts/spec_check.sh`; each rule's executable example is the code block below it, stored in `tests/spec/lexical/`.

**Reserved words:** `fn let mut const static struct enum trait impl match if else for in while loop return break continue use mod extern thread async await defer pub self unsafe as where type dyn true false` and the type words `int int32 uint float bool string`. `Self` is a type name, not a keyword. `bound`, `spawn`, `scope`, `select` are ordinary identifiers today.

**Token kinds (src/lexer/token_types.rs):** identifiers, integer/float/string/char literals, interpolated strings, lifetimes (`'a`), decorators (`@name`), operators and delimiters. The operator set is `+ - * / % == != < <= > >= && || ! ^ & | << >> = += -= *= /= %= &= |= ^= <<= >>= -> <- => |> :: . .. ..= ? @ _`. There is **no `~`** operator: the lexer rejects it (see Q-001 note).

The lexical productions in `grammar.ebnf` (`IDENT`, `INT_LIT`, `FLOAT_LIT`, `STRING_LIT`, `CHAR_LIT`) are machine-checked: generated lexemes must lex to exactly one token of the stated kind.

## Comments

**SPEC-LEX-001** A `//` comment runs to the end of the line and has no effect on the program.

<!-- example: tests/spec/lexical/SPEC-LEX-001.wj -->
```windjammer
fn main() {
    // println("never")
    println("a") // trailing
    println("b")
}
```
Output:
```text
a
b
```

## Doc comments

**SPEC-LEX-002** A `///` comment is a documentation comment: it is ignored at run time and attached to the following item by tooling.

<!-- example: tests/spec/lexical/SPEC-LEX-002.wj -->
```windjammer
/// Adds one.
fn inc(x: int) -> int { x + 1 }

fn main() {
    println("${inc(1)}")
}
```
Output:
```text
2
```

## Identifiers

**SPEC-LEX-003** Identifiers start with a letter or `_`, continue with letters, digits or `_`, and are case sensitive.

<!-- example: tests/spec/lexical/SPEC-LEX-003.wj -->
```windjammer
fn main() {
    let a = 1
    let A = 2
    let _x1 = 3
    println("${a} ${A} ${_x1}")
}
```
Output:
```text
1 2 3
```

## Reserved words

**SPEC-LEX-004** The reserved words (`fn let mut const static struct enum trait impl match if else for in while loop return break continue use mod extern thread async await defer pub self unsafe as where type dyn true false`) cannot be used as identifiers.

<!-- example: tests/spec/lexical/SPEC-LEX-004.wj -->
```windjammer
fn main() {
    let loop = 1
}
```
Rejected at compile time.

## Integer literals

**SPEC-LEX-005** Integer literals are decimal, `0x` hexadecimal, `0b` binary or `0o` octal; `_` may separate digits and is ignored.

<!-- example: tests/spec/lexical/SPEC-LEX-005.wj -->
```windjammer
fn main() {
    println("${0xff} ${0b1010} ${0o17} ${1_000_000} ${0xFF_FF}")
}
```
Output:
```text
255 10 15 1000000 65535
```

## Integer suffixes

**SPEC-LEX-006** A literal may carry a width suffix (`i8 i16 i32 i64 u8 u16 u32 u64 usize isize`), which fixes its type.

<!-- example: tests/spec/lexical/SPEC-LEX-006.wj -->
```windjammer
fn main() {
    let a = 200u8
    let b = 7i32
    let c = 0xffu8
    println("${a} ${b} ${c}")
}
```
Output:
```text
200 7 255
```

## Float literals

**SPEC-LEX-007** A float literal has a fraction (`1.5`) or an exponent (`1e3`, `2.5E-1`), optionally both; `_` may separate digits.

<!-- example: tests/spec/lexical/SPEC-LEX-007.wj -->
```windjammer
fn main() {
    println("${1.5} ${1e3} ${2.5E-1} ${1_0.5}")
}
```
Output:
```text
1.5 1000 0.25 10.5
```

## String escapes

**SPEC-LEX-008** Inside a string literal `\n \t \r \\ \"` denote newline, tab, carriage return, backslash and double quote.

<!-- example: tests/spec/lexical/SPEC-LEX-008.wj -->
```windjammer
fn main() {
    println("a\tb")
    println("c\\d")
    println("say \"hi\"")
    println("x\ny")
}
```
Output:
```text
a	b
c\d
say "hi"
x
y
```

## Dollar escape

**SPEC-LEX-009** `$$` inside a string literal is a literal `$`; a `$` not followed by `{` is also literal.

<!-- example: tests/spec/lexical/SPEC-LEX-009.wj -->
```windjammer
fn main() {
    println("cost: $$5")
    println("a$b")
}
```
Output:
```text
cost: $5
a$b
```

## Other escapes

**SPEC-LEX-010** A backslash followed by any character not listed in LEX-008 denotes that character itself. (Hex escapes `\xNN` do not exist; see OPEN_QUESTIONS Q-001.)

<!-- example: tests/spec/lexical/SPEC-LEX-010.wj -->
```windjammer
fn main() {
    println("a\qb")
}
```
Output:
```text
aqb
```

## Char literals

**SPEC-LEX-011** A char literal is one Unicode scalar value in single quotes; escapes `\n \t \r \\ \' \0` are allowed.

<!-- example: tests/spec/lexical/SPEC-LEX-011.wj -->
```windjammer
fn main() {
    let c = 'a'
    let n = '\n'
    println("${c}${c}")
    println("${n == '\n'}")
}
```
Output:
```text
aa
true
```

## Statement separation

**SPEC-LEX-012** A newline ends a statement; `;` may also separate statements and may be omitted at line ends. A statement continues onto the next line when it ends with a binary operator or an open bracket.

<!-- example: tests/spec/lexical/SPEC-LEX-012.wj -->
```windjammer
fn main() {
    let a = 1; let b = 2
    let c = a +
        b
    println("${c}")
    let d = [
        1,
        2,
    ]
    println("${d.len()}")
}
```
Output:
```text
3
2
```

## Multi-line string literals

**SPEC-LEX-013** A string literal may contain raw newlines, which are part of the value.

<!-- example: tests/spec/lexical/SPEC-LEX-013.wj -->
```windjammer
fn main() {
    let s = "one
two"
    println("${s.len()}")
}
```
Output:
```text
7
```

## Raw string literals

**SPEC-LEX-014** `r#"..."#` is a raw string: no escapes and no interpolation are processed.

<!-- example: tests/spec/lexical/SPEC-LEX-014.wj -->
```windjammer
fn main() {
    let s = r#"a\nb ${x}"#
    println(s)
}
```
Output:
```text
a\nb ${x}
```

## Block comments

**SPEC-LEX-015** [OPEN Q-002] `/* ... */` is a block comment and nests (`/* a /* b */ c */` is one comment). The lexer has no block comments today: `/*` lexes as `/` `*` and is a parse error.

<!-- example: tests/spec/lexical/SPEC-LEX-015.wj -->
```windjammer
fn main() {
    /* outer /* inner */ still comment */
    println("ok")
}
```
Output:
```text
ok
```
> **Known divergence:** the implementation does not do this yet; see [Q-002](OPEN_QUESTIONS.md). The example is marked `xfail`.

