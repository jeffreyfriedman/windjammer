# Collections and strings
<!-- feature: strings -->
<!-- feature: collections -->

Most programs spend their time on lists, maps and text. This chapter covers the types you will use for them and the
operations that make them pleasant.

## Lists

A list literal is a growable `Vec`. Add with `push`, order with `sort`, index with `[]`:

```wj
// expect: [1, 3, 5, 8] 4 1
fn main() {
    let mut v = [5, 3, 8]
    v.push(1)
    v.sort()
    println("${v:?} ${v.len()} ${v[0]}")
}
```

`for` walks a list directly. `enumerate` pairs each item with its position:

```wj
// expect: 0:a 1:b 2:c
fn main() {
    let mut parts: Vec<string> = []
    for (i, item) in ["a", "b", "c"].enumerate() {
        parts.push("${i}:${item}")
    }
    println(parts.join(" "))
}
```

Adapters such as `map` start at the collection itself, with no `.iter()` first. `collect` gathers the results and `sum`
adds them up:

```wj
// expect: [1, 9, 25, 64] 99
fn main() {
    let v = [1, 3, 5, 8]
    let squares: Vec<int> = v.map(|x| x * x).collect()
    println("${squares:?} ${squares.sum::<int>()}")
}
```

To keep only some items, `filter` works the same way; follow it with `.cloned()` to get values rather than references
before `collect`:

```wj
// expect: [8, 4]
fn main() {
    let v = [5, 3, 8, 4]
    let evens: Vec<int> = v.filter(|x| *x % 2 == 0).cloned().collect()
    println("${evens:?}")
}
```

`any` and `all` ask a question of every item, and `contains` checks for one:

```wj
// expect: true true true
fn main() {
    let v = [5, 3, 8, 4]
    println("${v.any(|x| x > 7)} ${v.all(|x| x > 2)} ${v.contains(8)}")
}
```

Lists nest, and an index chain reads the way you would expect:

```wj
// expect: [[1, 2], [3]] 2
~fn main() {
    let nested = [[1, 2], [3]]
    println("${nested:?} ${nested[0][1]}")
~}
```

An index past the end stops the program with a clear message instead of reading stray memory; see
[Errors and Result](errors.md).

## Maps and sets

`HashMap` associates keys with values. Its iteration order is not specified, so collect the keys and sort them when
output order matters:

```wj
// expect: a=1
// expect: b=2
use std::collections::HashMap

fn main() {
    let mut m: HashMap<string, int> = HashMap::new()
    m.insert("b", 2)
    m.insert("a", 1)
    let mut keys: Vec<string> = m.keys().collect()
    keys.sort()
    for k in keys {
        println("${k}=${m[k]}")
    }
}
```

Counting is a classic use. `entry` finds the slot for a key, and `and_modify` and `or_insert` say what to do whether it
exists or not:

```wj
// expect: a=2
// expect: b=1
use std::collections::HashMap

fn main() {
    let words: Vec<string> = ["a", "b", "a"]
    let mut counts: HashMap<string, int> = HashMap::new()
    for w in words {
        counts.entry(w).and_modify(|c| *c += 1).or_insert(1)
    }
    let mut keys: Vec<string> = counts.keys().collect()
    keys.sort()
    for k in keys {
        println("${k}=${counts[k]}")
    }
}
```

`BTreeMap` keeps its keys in order, which makes it the right choice when you print or iterate and want a stable result.
`HashSet` holds each value once:

```wj
// expect: 2 true
// expect: a 1
// expect: b 2
use std::collections::BTreeMap
use std::collections::HashSet

fn main() {
    let mut seen: HashSet<int> = HashSet::new()
    seen.insert(1)
    seen.insert(1)
    seen.insert(2)
    println("${seen.len()} ${seen.contains(2)}")

    let mut ordered: BTreeMap<string, int> = BTreeMap::new()
    ordered.insert("b", 2)
    ordered.insert("a", 1)
    for (k, val) in ordered {
        println("${k} ${val}")
    }
}
```

## Strings

A `string` holds UTF-8 text. `len()` counts bytes, `chars().count()` counts characters, and slicing with a range works on
byte positions:

```wj
// expect: 12 12 HELLO, WORLD true
// expect: Hello true HeLLo, WorLd
fn main() {
    let s = "Hello, World"
    println("${s.len()} ${s.chars().count()} ${s.to_uppercase()} ${s.contains("Wor")}")
    println("${s[0..5]} ${s.starts_with("Hell")} ${s.replace("l", "L")}")
}
```

Text with accented letters shows the difference between bytes and characters:

```wj
// expect: 6 5
fn main() {
    let word = "Wörld"
    println("${word.len()} ${word.chars().count()}")
}
```

Split, trim, repeat and join cover most of what text processing needs:

```wj
// expect: ["a", "b", "", "c"]
// expect: pad|ababab|x-y
// expect: THE QUICK BROWN FOX
fn main() {
    let parts: Vec<string> = "a,b,,c".split(",").collect()
    println("${parts:?}")
    println("${"  pad ".trim()}|${"ab".repeat(3)}|${["x", "y"].join("-")}")
    let words = "the quick brown fox"
    let caps: Vec<string> = words.split(" ").map(|w| w.to_uppercase()).collect()
    println(caps.join(" "))
}
```

Text becomes numbers with `parse`, which returns a `Result` because the text might not be a number; see
[Errors and Result](errors.md):

```wj
// expect: 43
// expect: -1
fn main() {
    let n = "42".parse::<int>().unwrap_or(0)
    println("${n + 1}")
    let bad = "x".parse::<int>().unwrap_or(-1)
    println("${bad}")
}
```

A `char` is a single Unicode character, written in single quotes:

```wj
// expect: a true 97
fn main() {
    let ch = 'a'
    println("${ch} ${ch.is_alphabetic()} ${ch as int}")
}
```

## Format specs

Inside `${...}`, a colon introduces a format spec: `:.2` for decimals, `:>5` to right-align in five columns, `:05` to pad
with zeros, `:?` for the debug form. These work on any value that has a text form, and the debug form works on lists,
maps, options and results too.

## Reading a whole file

`std::fs` reads and writes files. The default grant lets a program touch files inside its own project; see
[Effects and capabilities](effects.md) for how to grant more.

```sh,verify
$ cat > count_lines.wj <<'EOF'
use std::fs

fn main() {
    let text = fs::read_to_string("notes.txt").unwrap_or("")
    let mut n = 0
    for line in text.lines() {
        if line != "" {
            n += 1
        }
    }
    println("${n} lines")
}
EOF
$ cat > notes.txt <<'EOF'
first
second

third
EOF
$ wj run count_lines.wj
3 lines
```
