# Side by side

<!-- feature: showcase-side-by-side -->

Four small tasks, written in Windjammer and in the languages people most often compare it with. Nothing on this page is a sketch: the Windjammer program is run, interpreted and compiled, and the Go and TypeScript versions are run too and must print exactly the same output. The Rust is not hand-written either. It is the Rust the Windjammer compiler generates for the same program, and the book's tests check that the compiler still generates what is shown.

## Count words

A small command-line job: split text and count words with a map, then print the counts in order.

```wj
// expect: and: 2
// expect: bat: 1
// expect: cat: 1
// expect: hat: 1
// expect: the: 3
use std::collections::HashMap

fn main() {
    let text = "the cat and the hat and the bat"
    let words: Vec<string> = text.split(" ").map(|w| w.to_string()).collect()
    let mut counts: HashMap<string, int> = HashMap::new()
    for w in words {
        counts.entry(w).and_modify(|c| *c += 1).or_insert(1)
    }
    let mut keys: Vec<string> = counts.keys().collect()
    keys.sort()
    for k in keys {
        println("${k}: ${counts[k]}")
    }
}
```

The Rust the compiler generates:

```rust,generated
use std::collections::HashMap;
fn main() {
    let text: String = String::from("the cat and the hat and the bat");
    let words: Vec<String> = text.split(" ").map(|w| w.to_string()).collect::<Vec<_>>();
    let mut counts: HashMap<String, i64> = HashMap::new();
    for w in words {
        counts.entry(w).and_modify(|c| {
            *c += 1;
        }).or_insert(1);
    }
    let mut keys: Vec<String> = counts.keys().map(|s| s.to_string()).collect::<Vec<String>>();
    keys.sort();
    for k in keys {
        println!("{}: {}", k, counts[&*k]);
    }
}
```

Go:

```go,run
package main

import (
	"fmt"
	"sort"
	"strings"
)

func main() {
	text := "the cat and the hat and the bat"
	counts := map[string]int{}
	for _, w := range strings.Split(text, " ") {
		counts[w]++
	}
	keys := make([]string, 0, len(counts))
	for k := range counts {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		fmt.Printf("%s: %d\n", k, counts[k])
	}
}
```

TypeScript:

```ts,run
const text = "the cat and the hat and the bat";
const counts = new Map<string, number>();
for (const w of text.split(" ")) {
  counts.set(w, (counts.get(w) ?? 0) + 1);
}
for (const k of [...counts.keys()].sort()) {
  console.log(`${k}: ${counts.get(k)}`);
}
```

## Convert to and from JSON

A struct goes out as JSON and comes back in. No schema library, no derive.

```wj
// expect: {"name":"demo","retries":3,"verbose":false}
// expect: demo 3
use std::json

struct Settings {
    name: string,
    retries: int,
    verbose: bool,
}

fn main() {
    let s = Settings { name: "demo", retries: 3, verbose: false }
    let text = json.to_string(s).unwrap_or("")
    println(text)
    let back: Result<Settings, string> = json.parse_string(text)
    match back {
        Ok(v) => println("${v.name} ${v.retries}"),
        Err(e) => println(e),
    }
}
```

The Rust the compiler generates:

```rust,generated
use serde::{Serialize, Deserialize};

use windjammer_runtime::json;
use windjammer_runtime::json::Value;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]

struct Settings {
    name: String,
    retries: i64,
    verbose: bool,
}

fn main() {
    let s = Settings { name: String::from("demo"), retries: 3_i64, verbose: false };
    let text = json::to_string(s).unwrap_or(String::new());
    windjammer_runtime::io::println(&format!("{}", text));
    let back: Result<Settings, String> = json::parse_string(&text);
    match back {
        Ok(v) => windjammer_runtime::io::println(&format!("{} {}", v.name, v.retries)),
        Err(e) => windjammer_runtime::io::println(&format!("{}", e)),
    }
}
```

Go:

```go,run
package main

import (
	"encoding/json"
	"fmt"
)

type Settings struct {
	Name    string `json:"name"`
	Retries int    `json:"retries"`
	Verbose bool   `json:"verbose"`
}

func main() {
	s := Settings{Name: "demo", Retries: 3, Verbose: false}
	data, _ := json.Marshal(s)
	fmt.Println(string(data))
	var back Settings
	if err := json.Unmarshal(data, &back); err != nil {
		fmt.Println(err)
		return
	}
	fmt.Printf("%s %d\n", back.Name, back.Retries)
}
```

TypeScript:

```ts,run
interface Settings {
  name: string;
  retries: number;
  verbose: boolean;
}

const s: Settings = { name: "demo", retries: 3, verbose: false };
const text = JSON.stringify(s);
console.log(text);
const back: Settings = JSON.parse(text);
console.log(`${back.name} ${back.retries}`);
```

## Run workers in parallel

Four workers each sum a quarter of a range; the results are joined.

```wj
// expect: total 4950
use std::thread

fn sum_range(lo: int, hi: int) -> int {
    let mut total = 0
    let mut i = lo
    while i < hi {
        total += i
        i += 1
    }
    total
}

fn main() {
    let a = thread::spawn(move || sum_range(0, 25))
    let b = thread::spawn(move || sum_range(25, 50))
    let c = thread::spawn(move || sum_range(50, 75))
    let d = thread::spawn(move || sum_range(75, 100))
    let zero: int = 0
    let total = a.join().unwrap_or(zero) + b.join().unwrap_or(zero) + c.join().unwrap_or(zero) + d.join().unwrap_or(zero)
    println("total ${total}")
}
```

The Rust the compiler generates:

```rust,generated
use windjammer_runtime::thread;
use windjammer_runtime::thread::Duration;
#[inline]
fn sum_range(lo: i64, hi: i64) -> i64 {
    let mut total = 0_i64;
    let mut i = lo;
    while i < hi {
        total += i;
        i += 1;
    }
    total
}

fn main() {
    let a = thread::spawn(|| sum_range(0_i64, 25_i64));
    let b = thread::spawn(|| sum_range(25_i64, 50_i64));
    let c = thread::spawn(|| sum_range(50_i64, 75_i64));
    let d = thread::spawn(|| sum_range(75_i64, 100_i64));
    let zero: i64 = 0_i64;
    let total = a.join().unwrap_or(zero) + b.join().unwrap_or(zero) + c.join().unwrap_or(zero) + d.join().unwrap_or(zero);
    windjammer_runtime::io::println(&format!("total {}", total));
}
```

Go:

```go,run
package main

import (
	"fmt"
	"sync"
)

func sumRange(lo, hi int) int {
	total := 0
	for i := lo; i < hi; i++ {
		total += i
	}
	return total
}

func main() {
	var wg sync.WaitGroup
	results := make([]int, 4)
	for w := 0; w < 4; w++ {
		wg.Add(1)
		go func(w int) {
			defer wg.Done()
			results[w] = sumRange(w*25, (w+1)*25)
		}(w)
	}
	wg.Wait()
	total := 0
	for _, r := range results {
		total += r
	}
	fmt.Printf("total %d\n", total)
}
```

TypeScript:

```ts,run
function sumRange(lo: number, hi: number): number {
  let total = 0;
  for (let i = lo; i < hi; i++) {
    total += i;
  }
  return total;
}

async function main() {
  const parts = await Promise.all([0, 1, 2, 3].map(async (w) => sumRange(w * 25, (w + 1) * 25)));
  const total = parts.reduce((a, b) => a + b, 0);
  console.log(`total ${total}`);
}

main();
```

## Route a request

The core of an HTTP handler: choose a response from a method and a path.

```wj
// expect: 200 welcome
// expect: 201 created
// expect: 405 method not allowed
// expect: 404 not found
fn handle(method: string, path: string) -> string {
    match path {
        "/" => "200 welcome",
        "/health" => "200 ok",
        "/items" => if method == "POST" { "201 created" } else { "405 method not allowed" },
        _ => "404 not found",
    }
}

fn main() {
    println(handle("GET", "/"))
    println(handle("POST", "/items"))
    println(handle("DELETE", "/items"))
    println(handle("GET", "/missing"))
}
```

The Rust the compiler generates:

```rust,generated
#[inline]
fn handle(method: &str, path: &str) -> String {
    match path {
        "/" => String::from("200 welcome"),
        "/health" => String::from("200 ok"),
        "/items" => {
            if method == String::from("POST") {
                String::from("201 created")
            } else {
                String::from("405 method not allowed")
            }
        },
        _ => String::from("404 not found"),
    }
}

fn main() {
    println!("{}", handle("GET", "/"));
    println!("{}", handle("POST", "/items"));
    println!("{}", handle("DELETE", "/items"));
    println!("{}", handle("GET", "/missing"));
}
```

Go:

```go,run
package main

import "fmt"

func handle(method, path string) string {
	switch {
	case method == "GET" && path == "/":
		return "200 welcome"
	case method == "GET" && path == "/health":
		return "200 ok"
	case method == "POST" && path == "/items":
		return "201 created"
	case path == "/items":
		return "405 method not allowed"
	default:
		return "404 not found"
	}
}

func main() {
	fmt.Println(handle("GET", "/"))
	fmt.Println(handle("POST", "/items"))
	fmt.Println(handle("DELETE", "/items"))
	fmt.Println(handle("GET", "/missing"))
}
```

TypeScript:

```ts,run
function handle(method: string, path: string): string {
  if (method === "GET" && path === "/") return "200 welcome";
  if (method === "GET" && path === "/health") return "200 ok";
  if (method === "POST" && path === "/items") return "201 created";
  if (path === "/items") return "405 method not allowed";
  return "404 not found";
}

console.log(handle("GET", "/"));
console.log(handle("POST", "/items"));
console.log(handle("DELETE", "/items"));
console.log(handle("GET", "/missing"));
```

## Counting the lines

The table is computed from the programs above. Blank lines and comment lines are not counted, and the Rust column counts the
generated Rust shown, which includes the imports and attributes the compiler adds.

<!-- wj:gen name=showcase_loc page=showcase/side-by-side.md -->
| Task | Windjammer | Rust (generated) | Go | TypeScript |
|---|---|---|---|---|
| Count words | 14 | 16 | 21 | 8 |
| Convert to and from JSON | 16 | 20 | 21 | 10 |
| Run workers in parallel | 19 | 21 | 29 | 13 |
| Route a request | 14 | 21 | 22 | 11 |
<!-- wj:end -->

In every row the Windjammer program is shorter than the Go one and no longer than the Rust the compiler generates for it; a
test checks both statements against the computed table. TypeScript, which leaves most types to inference and has no ownership
to speak of, is the shortest here, and Windjammer is the version that is statically typed, ownership-checked and compiles to
native code at that size. Line counts are a rough measure on small programs, so read the table as a feel for the ceremony and
not as a benchmark. What the page proves is that all four versions behave the same.

## What to notice

* **Ownership never appears.** The Windjammer sources have no `&`, no lifetimes and no `.clone()`. The generated Rust has
  the borrows: look at the signature of `handle` in the last task, where the compiler chose `&str` for both parameters.
  [Ownership and borrowing, inferred](../tutorial/ownership.md) and the next page go deeper.
* **JSON needs no glue.** The struct is converted as it is, and the Rust shows the derives the compiler added.
* **Parallel work is a closure and a join.** The Go and TypeScript versions need a wait group or a promise combinator for
  the same job.
* **A request router is a `match`.** The compiler checks that every path ends in a response.
