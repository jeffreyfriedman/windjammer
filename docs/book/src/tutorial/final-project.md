# A final project

To close the tutorial, here is a small but complete program that uses most of what you have learned: modules, structs,
collections, closures, error handling, tests and a capability grant. It reads a text file and prints the most common words.

The project has three source files and a manifest.

```text
wordfreq/
  wj.toml
  input.txt
  src/
    main.wj      reads the file and prints the report
    text.wj      splits text into lowercase words
    tally.wj     counts words and ranks them
    text_test.wj tests for text.wj
```

## The manifest

The program reads one file, so it asks for exactly that: reading files inside the project. Nothing else is granted, and
`wj check` would tell you if the code tried to do more (see [Effects and capabilities](effects.md)).

## Text

`text.wj` turns raw text into words. `clean` keeps letters and digits and lowercases them; `words` splits on whitespace
and drops anything that cleans to nothing.

## Counting

`tally.wj` counts with a `HashMap`, sorts the keys so the order is the same on every run, and ranks entries by count. The
sort is stable, so words with equal counts stay in alphabetical order.

## Main

`main.wj` ties them together. `report` returns a `Result`, so a missing file is a message and not a crash; the `?` after
`map_err` hands the error to the caller.

## Build it and run it

```sh,verify
$ mkdir -p wordfreq/src
$ cat > wordfreq/wj.toml <<'EOF'
[package]
name = "wordfreq"
version = "0.1.0"

[capabilities]
fs.read = ["./**"]
EOF
$ cat > wordfreq/src/text.wj <<'EOF'
// Splitting text into lowercase words.

pub fn clean(word: string) -> string {
    let mut out = ""
    for c in word.chars() {
        if c.is_alphanumeric() {
            out = out + c.to_lowercase().to_string()
        }
    }
    out
}

pub fn words(text: string) -> Vec<string> {
    let mut found: Vec<string> = []
    for raw in text.split_whitespace() {
        let w = clean(raw)
        if w != "" {
            found.push(w)
        }
    }
    found
}
EOF
$ cat > wordfreq/src/tally.wj <<'EOF'
// Counting words and ranking them.
use std::collections::HashMap

pub struct Entry {
    pub word: string,
    pub count: int,
}

pub fn count(words: Vec<string>) -> Vec<Entry> {
    let mut counts: HashMap<string, int> = HashMap::new()
    for w in words {
        counts.entry(w).and_modify(|c| *c += 1).or_insert(1)
    }
    let mut keys: Vec<string> = counts.keys().collect()
    keys.sort()
    let mut entries: Vec<Entry> = []
    for k in keys {
        entries.push(Entry { word: k, count: counts[k] })
    }
    entries.sort_by(|a, b| b.count.cmp(a.count))
    entries
}
EOF
$ cat > wordfreq/src/main.wj <<'EOF'
mod tally
mod text

use std::fs
use tally::Entry
use tally::count
use text::words

fn report(path: string, top: int) -> Result<string, string> {
    let body = fs::read_to_string(path).map_err(|e| "cannot read ${path}: ${e}")?
    let ranked = count(words(body))
    let mut lines: Vec<string> = []
    let mut shown: int = 0
    for e in ranked {
        if shown < top {
            lines.push("${e.count} ${e.word}")
            shown += 1
        }
    }
    Ok(lines.join("\n"))
}

fn main() {
    match report("input.txt", 3) {
        Ok(text) => println(text),
        Err(e) => println(e),
    }
}
EOF
$ cat > wordfreq/input.txt <<'EOF'
The cat and the hat.
The end, and THE start!
EOF
$ cd wordfreq && wj run
4 the
2 and
1 cat
```

The most common words come out ranked, with ties in alphabetical order.

## Test it

Tests sit beside the module they cover:

```sh,verify
$ mkdir -p wordfreq/src
$ cat > wordfreq/wj.toml <<'EOF'
[package]
name = "wordfreq"
version = "0.1.0"
EOF
$ cat > wordfreq/src/text.wj <<'EOF'
pub fn clean(word: string) -> string {
    let mut out = ""
    for c in word.chars() {
        if c.is_alphanumeric() {
            out = out + c.to_lowercase().to_string()
        }
    }
    out
}

pub fn words(text: string) -> Vec<string> {
    let mut found: Vec<string> = []
    for raw in text.split_whitespace() {
        let w = clean(raw)
        if w != "" {
            found.push(w)
        }
    }
    found
}
EOF
$ cat > wordfreq/src/text_test.wj <<'EOF'
use text::clean
use text::words

@test
fn clean_strips_punctuation_and_case() {
    assert_eq(clean("Hat,"), "hat")
    assert_eq(clean("..."), "")
}

@test
fn words_skips_empty_pieces() {
    let found = words("The end -- and THE start!")
    assert_eq(found.len(), 5)
    assert_eq(found[0], "the")
}
EOF
$ cd wordfreq && wj test --interpret .
clean_strips_punctuation_and_case ... ok
words_skips_empty_pieces ... ok
test result: ok. 2 passed; 0 failed
```

## Ship it

Everything so far ran in the interpreter. Before you hand the program to anyone, build it natively and check that it
agrees:

```text
wj run --compiled
wj build --release
```

The release build is a plain native binary, and `wj build --no-cargo` shows the Rust it came from if you want to read it.

## Where to take it

* Read the file name from the command line with `std::env::args()` and grant `fs.read` for just that path.
* Add a `--top N` option and parse it with `parse`, reporting a bad number as an `Err`.
* Replace the `HashMap` with a `BTreeMap` and see how the sorting code simplifies.
* Count words from several files in parallel with `spawn`, as described in [Concurrency](concurrency.md).
* Publish the counting logic as a package that other projects depend on, following [wj.toml](../reference/wj-toml.md).

The [cookbook](../cookbook/recipes.md) has more complete answers to everyday tasks.
