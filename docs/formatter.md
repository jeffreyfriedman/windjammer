# Windjammer formatter (`wj fmt`)

`wj fmt` formats `.wj` source. It is deterministic, idempotent, keeps every comment, and
never changes program meaning: only whitespace and optional trailing commas are touched.

## Usage

```
wj fmt                 # format all .wj files under the current directory
wj fmt src/main.wj lib # format files and/or directories (recursive)
wj fmt --check [PATH]  # report unformatted files, exit 1 if any (no writes)
wj fmt -               # read stdin, write formatted source to stdout
wj fmt --rust          # legacy: run `cargo fmt --all` (generated Rust crates)
```

Directories named `target`, `build`, `node_modules` and hidden directories are skipped.
Files that cannot be tokenized are reported and make the command fail (exit 1).

## Configuration (`wj.toml`)

The nearest `wj.toml` above each file is read:

```toml
[fmt]
max_width = 100  # lists longer than this are broken one element per line
indent = 4       # spaces per level
```

## What it does

- Indentation from bracket nesting; continuation lines (leading `.`, `|>`, `&&`, `||`, or a
  previous line ending in a binary operator) get one extra level.
- Canonical spacing around operators, commas, colons, braces; `{` on its own line after a
  declaration/control header moves up; `} else` is joined.
- Blank lines collapse to at most one; none directly after `{` or before `}`.
- Trailing commas are added in multi-line lists that already use commas.
- Over-long `(...)`/`[...]` lists are broken one element per line.
- Trailing comments are separated from code by one space; comments are never moved.

## Editor integration

The language server answers `textDocument/formatting` and `textDocument/rangeFormatting`
with minimal line-based `TextEdit`s (range formatting keeps only hunks touching the range).

## Known limitations

- Author line breaks are preserved; lines are not re-joined to fit within `max_width`.
- Only parenthesised and bracketed lists are wrapped (not binary expressions, generics,
  or brace bodies).
- Spacing around `<`, `>`, `>>` and closure pipes `|` is kept as written because they are
  ambiguous without a parse (generics vs comparison, closure vs bit-or).
- Files with characters the compiler lexer rejects (e.g. `#`, `$` outside strings) are not
  formatted.
