# Try ideas: eval and the REPL
<!-- feature: repl -->

Some questions are best asked without writing a file: what does this method return, how does this slice behave at
the edge. Windjammer answers them in the interpreter, with no project and no build.

## One expression: `wj eval`

`wj eval` runs a piece of code and prints the value of its last expression. The common standard modules (`strings`,
`math`, `json`, `time`, `fs`, `env` and the collections) are already imported:

```sh,verify
$ wj eval 'strings.to_upper("hi")'
HI
$ wj eval 'let x = 6; x * 7'
42
```

Statements are separated by `;` or by a newline. A `;` inside a string, a bracket or a block does not split anything, so
what you paste is read the way you wrote it:

```sh,verify
$ wj eval 'let s = "a;b"; strings.len(s)'
3
$ wj eval "$(printf 'let xs = [1, 2, 3]\nlet total = xs.len() + 10\ntotal')"
13
```

## A session: `wj repl`

`wj repl` keeps what you define between inputs. When an input is not finished (an operator at the end of the line, an
open bracket, a string that has not closed), the REPL waits for the next line and tries again, until the whole input
parses. A function you define stays available for the lines after it:

```sh,verify
$ printf 'let total = 1 +\n  2 +\n  3\ntotal\nfn sq(n: int) -> int {\n    n * n\n}\nsq(total)\n' | wj repl
6
36
```

Every input is remembered. `:history` lists them, `!N` runs input number N again, and `!!` runs the last one again. A
multi-line input is listed whole, and recalling a number that is not there tells you how many there are:

```sh,verify
$ printf 'let x = 20\nx + 1\n:history\n!2\n!!\n!9\n' | wj repl 2>&1
21
  1  let x = 20
  2  x + 1
21
21
no input 9 in the history (4 so far)
```

At a terminal the REPL also gives you line editing and the arrow keys; the examples here pipe their input so they can be
run by the book's tests.

## The REPL tells you what the compiler would say

The REPL runs in the interpreter, and the interpreter runs everything that would run compiled. A few inputs are valid for
the interpreter yet would be refused by the compiled build, such as mixing two integer widths in one expression. The
REPL warns about those and then runs them anyway, so you learn while you explore:

```sh,verify
$ printf 'let five = 5\nlet add5 = |x: int| x + five\nadd5(1)\n' | wj repl 2>&1 | grep -c "the compiled build would reject this"
1
$ printf 'let five = 5\nlet add5 = |x: int| x + five\nadd5(1)\n' | wj repl 2>/dev/null
6
```

The same checks are what [Interpreter or compiled](interpreter-or-compiled.md) describes for whole programs. When an idea
grows past a few lines, move it to a file and `wj run` it; [Your first project](../getting-started/first-project.md)
shows how.

## In the browser

The [playground](playground.md) is the same interpreter compiled to WebAssembly. Every program in this book has a Run
link under it, so you can try the examples without installing anything.
