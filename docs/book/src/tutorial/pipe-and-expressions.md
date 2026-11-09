# Pipe notation and expression-oriented style

<!-- feature: pipe -->
<!-- feature: guard-match -->
<!-- feature: trailing-blocks -->
<!-- feature: named-arguments -->

Windjammer programs read top to bottom and left to right. Almost everything is an expression that produces a value: `if`,
`match`, blocks, and calls. A few small pieces of notation let you write data flowing through a series of steps without
naming every intermediate result.

## The pipe operator

`x |> f` means `f(x)`. A stage can carry more arguments: `x |> f(a, b)` means `f(x, a, b)`. Stages chain from left to
right, so the order you read is the order things happen:

```wj
// expect: 16
// expect: 6
fn add(x: int, y: int) -> int {
    x + y
}

fn double(x: int) -> int {
    x * 2
}

fn main() {
    println("${3 |> add(5) |> double}")
    println("${1 + 2 |> double}")
}
```

`|>` binds more loosely than every other operator, so `1 + 2 |> double` pipes the whole sum: it is `double(1 + 2)`.

Mark where the piped value goes with a single `_` when it is not the first argument:

```wj
// expect: -7
// expect: 7
fn sub(a: int, b: int) -> int {
    a - b
}

fn main() {
    println("${10 |> sub(3, _)}")
    println("${10 |> sub(_, 3)}")
}
```

The first line is `sub(3, 10)`, which is `-7`, and the second is `sub(10, 3)`, which is `7`. Read the placeholder as "the
value goes here".

### Methods and closures as stages

`x |> .method(args)` calls a method on `x`, which is how iterator-style chains read as a pipeline. A closure can be a
stage too:

```wj
// expect: [2, 4]
// expect: 6
fn main() {
    let evens: Vec<int> = [1, 2, 3, 4] |> .filter(|x| *x % 2 == 0) |> .cloned() |> .collect()
    println("${evens:?}")
    println("${5 |> |v| v + 1}")
}
```

A pipeline may continue on the next line by starting that line with `|>`.

### Pipes and errors

A stage followed by `?` propagates an error exactly like the nested call would. `text |> parse? |> half?` is
`half(parse(text)?)?`, so a chain of fallible steps stays flat:

```wj
// expect: Ok(4)
// expect: Err("odd: 7")
// expect: Err("bad: x")
fn parse(text: string) -> Result<int, string> {
    text.parse::<int>().map_err(|_| "bad: ${text}")
}

fn half(n: int) -> Result<int, string> {
    if n % 2 == 0 {
        Ok(n / 2)
    } else {
        Err("odd: ${n}")
    }
}

fn pipeline(text: string) -> Result<int, string> {
    let n = text |> parse? |> half?
    Ok(n)
}

fn main() {
    println("${pipeline("8"):?}")
    println("${pipeline("7"):?}")
    println("${pipeline("x"):?}")
}
```

The right-hand side of `|>` must be something callable: a function, a call, a method stage or a closure. A literal or
arithmetic there is a compile-time error. The parser rewrites every pipe into ordinary calls, so each backend behaves the
same.

## Named and default arguments

A parameter can declare a default with `name: Type = value`. At a call, positional arguments come first and named ones
follow in any order. Defaults are evaluated at the call, every time:

```wj
// expect: Hello, Ada!
// expect: Hello, Ada?
// expect: Hi, Ada.
// expect: 5 15 8
fn greet(name: string, greeting: string = "Hello", punct: string = "!") -> string {
    "${greeting}, ${name}${punct}"
}

fn area(w: int, h: int = 1) -> int {
    w * h
}

fn main() {
    println(greet("Ada"))
    println(greet("Ada", punct: "?"))
    println(greet("Ada", greeting: "Hi", punct: "."))
    println("${area(5)} ${area(5, 3)} ${area(h: 4, w: 2)}")
}
```

Mistakes are caught with a list of the real parameters: an unknown name, a parameter given twice, a positional argument
after a named one, or a required parameter left out. Parameters with defaults come after the ones without.

## Trailing blocks

When the last argument of a call is a function, write it after the parentheses. A trailing block is a closure with no
parameters, or with parameters before an arrow:

```wj
// expect: tick
// expect: tick
// expect: 50
fn repeat(times: int, action: fn()) {
    for _ in 0..times {
        action()
    }
}

fn apply(x: int, f: fn(int) -> int) -> int {
    f(x)
}

fn main() {
    repeat(2) { println("tick") }
    let r = apply(5) { n -> n * 10 }
    println("${r}")
}
```

A call takes at most one trailing block; pass further callbacks as named arguments. In the header of an `if`, `while`,
`for` or `match`, a `{` after a call always opens the body.

## Everything is an expression

`if` and blocks produce values, so a binding can be computed in place:

```wj
// expect: pass
// expect: 20
fn main() {
    let label = if 75 >= 70 { "pass" } else { "fail" }
    println(label)
    let total = {
        let a = 4
        let b = 5
        a * b
    }
    println("${total}")
}
```

### A table of conditions

`match` with no scrutinee is a table of conditions: it evaluates them in order and yields the value of the first that is
true. It must end with `_`, since the conditions are not known to cover everything:

```wj
// expect: A B F
fn grade(score: int) -> string {
    match {
        score >= 90 => "A",
        score >= 80 => "B",
        score >= 70 => "C",
        _ => "F",
    }
}

fn main() {
    println("${grade(95)} ${grade(85)} ${grade(10)}")
}
```

An arm after `_` is unreachable and an error.

## Choosing a style

Pipes shine when data moves through steps that each take it as their first argument. Named arguments shine when a
function has options. Trailing blocks shine for callbacks. A table of conditions shines when an `if / else if` ladder
would hide that every branch produces the same kind of value. None of them changes what a program means; they change how
much of it you can see at once.
