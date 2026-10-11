<!-- GENERATED FILE - DO NOT EDIT. Generator: `keywords` (scripts/docs_gen.sh, tools/docs_truth/). Checked by `cargo test --test all docs_truth`. -->

# Keywords and operators

Generated from `src/lexer/keywords.rs` (pinned to the lexer by unit tests) and from the lexer itself.

## Reserved words

| Word | Meaning |
|---|---|
| `fn` | function item or function type |
| `let` | local binding |
| `mut` | mutable binding or mutable reference |
| `const` | compile-time constant |
| `static` | static item |
| `struct` | struct item |
| `enum` | enum item |
| `trait` | trait item |
| `impl` | implementation block |
| `match` | pattern match expression |
| `if` | conditional expression |
| `else` | alternative branch of `if` |
| `for` | loop over an iterator |
| `in` | separator in `for pat in expr` |
| `while` | conditional loop |
| `loop` | unconditional loop |
| `return` | return from a function |
| `break` | leave a loop |
| `continue` | next loop iteration |
| `use` | import |
| `mod` | module declaration |
| `extern` | foreign declaration |
| `thread` | `thread { }` block |
| `async` | async function or block |
| `await` | await a future |
| `defer` | run a statement at scope exit |
| `pub` | public visibility |
| `self` | the receiver value |
| `unsafe` | unsafe block or function |
| `as` | type cast |
| `where` | trait-bound clause |
| `type` | type alias or associated type |
| `dyn` | trait object |

## Built-in type names

| Word | Meaning |
|---|---|
| `int` | default signed integer type |
| `int32` | 32-bit signed integer |
| `uint` | default unsigned integer type |
| `float` | default floating-point type |
| `bool` | boolean type |
| `string` | owned string type |

## Literals and wildcard

| Word | Meaning |
|---|---|
| `true` | boolean literal |
| `false` | boolean literal |
| `_` | wildcard pattern |

## Soft keywords (identifiers outside their position)

| Word | Meaning |
|---|---|
| `Self` | the implementing type (lexes as an identifier) |
| `bound` | `bound Alias = Trait + ...` item (identifier elsewhere) |

## Operators and punctuation

Every punctuation string of up to three characters that the lexer turns into exactly one token.

| Lexeme | Token |
|---|---|
| `!` | Bang |
| `%` | Percent |
| `&` | Ampersand |
| `(` | LParen |
| `)` | RParen |
| `*` | Star |
| `+` | Plus |
| `,` | Comma |
| `-` | Minus |
| `.` | Dot |
| `/` | Slash |
| `:` | Colon |
| `;` | Semicolon |
| `<` | Lt |
| `=` | Assign |
| `>` | Gt |
| `?` | Question |
| `@` | At |
| `[` | LBracket |
| `]` | RBracket |
| `^` | Caret |
| `{` | LBrace |
| `\|` | Pipe |
| `}` | RBrace |
| `!=` | Ne |
| `%=` | PercentAssign |
| `&&` | And |
| `&=` | AndAssign |
| `*=` | StarAssign |
| `+=` | PlusAssign |
| `-=` | MinusAssign |
| `->` | Arrow |
| `..` | DotDot |
| `/=` | SlashAssign |
| `::` | ColonColon |
| `<-` | LeftArrow |
| `<<` | Shl |
| `<=` | Le |
| `==` | Eq |
| `=>` | FatArrow |
| `>=` | Ge |
| `>>` | Shr |
| `^=` | XorAssign |
| `\|=` | OrAssign |
| `\|>` | PipeOp |
| `\|\|` | Or |
| `..=` | DotDotEq |
| `<<=` | ShlAssign |
| `>>=` | ShrAssign |
| `\|>?` | PipeTry |
