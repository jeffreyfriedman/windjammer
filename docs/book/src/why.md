# Why Windjammer

Systems languages ask you to say a great deal about memory before they let you say what your program does. Scripting
languages let you say what your program does and leave the rest to a runtime. Windjammer aims at both: write the
program, and still get native output.

## What it gives you

**Ownership, handled for you.** Parameters and `self` become shared borrows, mutable borrows or moves automatically.
You never write `&`, `&mut` or a lifetime, and you can always see the choice the compiler made with
`wj explain-ownership`, or read the generated Rust. The [ownership chapter](tutorial/ownership.md) shows how.

**A short loop, native output.** `wj run` interprets, with no build step ([measured](showcase/fast-loop.md)). `wj build`
turns the same source into Rust and then into a native binary. Go, JavaScript and WebAssembly targets sit alongside; see
[Compile targets](reference/targets.md).

**Honest numbers.** `int` is a 64-bit signed integer that checks for overflow, `float` is a 64-bit float, and the
sized types (`u8`, `i32`, `usize`, `f32` and friends) are first-class and never converted behind your back. See
[Numbers and sizes](tutorial/numbers.md).

**Programs that say what they do to the world.** Reading files, using the network, reading environment variables,
starting processes: each is an effect, inferred from your code and granted explicitly in `wj.toml`. A program
asks for exactly the authority it needs. See [Effects and capabilities](tutorial/effects.md).

**Made for people and their coding agents.** Diagnostics come as structured JSON with stable codes, an offline docs
search (`wj docs`) answers questions without a network, and the language surface is small enough to hold in your head.

## How it compares

| If you know | Windjammer keeps | Windjammer changes |
|---|---|---|
| Rust | the output (it compiles to Rust), the crates | no `&`, `&mut`, lifetimes, `.clone()` or `.to_string()` in everyday code |
| Go | the appetite for simplicity and a short feedback loop | sum types with exhaustive `match`, generics with traits |
| TypeScript | the light syntax and string interpolation | static types all the way down, and a native binary |
| Python | the edit-and-run loop with no build step | static types, and a path to a native binary without a rewrite |

The appendix has a migration guide for each of these.

## Where Windjammer is today

Windjammer is pre-1.0. The Rust backend and ownership inference are the primary target; the interpreter, the Go,
JavaScript and WebAssembly backends and shaders are in preview. The [compile targets](reference/targets.md) page is
generated from the conformance suite and shows exactly which programs each backend runs. Open items are tracked in
[Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md), and chapters link there when a feature has a rough edge worth knowing about.
