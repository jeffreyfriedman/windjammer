# Introduction

Windjammer is a compiled language with the feel of a scripting language. You write plain values and functions; the
compiler works out who owns what, when to borrow and when to copy. The same source runs instantly in an interpreter
while you iterate, and compiles to native Rust when you ship.

```wj
// expect: Hello, Windjammer!
fn main() {
    let name = "Windjammer"
    println("Hello, ${name}!")
}
```

[Run this in the playground](playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBIZWxsbywgV2luZGphbW1lciEKZm4gbWFpbigpIHsKICAgIGxldCBuYW1lID0gIldpbmRqYW1tZXIiCiAgICBwcmludGxuKCJIZWxsbywgJHtuYW1lfSEiKQp9Cg)

This book takes you from that first program to a finished project, and then stays with you as a reference.

## How the book is organised

* **Getting started** installs `wj` and walks through creating, running, checking and building a project.
* **The tutorial** teaches the language one idea at a time. Each chapter builds on the last, and every example in it
  is a whole program you can paste into a file and run.
* **The reference** is generated from the compiler itself: the grammar, keywords, standard library, command line,
  diagnostic codes and compile targets always describe the `wj` you have.
* **The cookbook** collects short, complete answers to common tasks.
* **The appendix** helps you carry what you know from Rust, Go and TypeScript.

## Running the examples

Every example is a complete program. Save it as `example.wj` and run it with `wj run example.wj`.

Lines of output that an example prints are written in a leading comment, `// expect:`, so you can see what to expect
before you run it. The book's own test suite runs each example in the interpreter and in the compiled path and checks
that the output matches; see [How this book is tested](appendix/how-this-book-is-tested.md).

## Where to go from here

New to Windjammer? Start with [Install](getting-started/install.md). Coming from Rust, read
[Ownership and borrowing, inferred](tutorial/ownership.md) first: it shows what the compiler decides for you and how to
look at its decisions. Looking for a specific fact? The [reference](reference/language.md) is the place.
