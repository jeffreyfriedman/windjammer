//! Machine-readable registry of what the interpreter supports.
//!
//! `docs/INTERPRETER.md` (feature lists and counts) is generated from this table, and
//! `tests/interpreter_features_test.rs` runs every entry's `probe` program through the real interpreter:
//! a `Supported` feature must print exactly `expect` (or, when `expect` starts with `panic: `, panic with
//! that message); an `Unsupported` one must fail with
//! `ErrorKind::Unsupported` and a message containing `expect`. Adding interpreter support for a listed
//! `Unsupported` feature therefore fails that test until the entry is flipped, and a regression of a
//! `Supported` one fails it too, so the document cannot drift from the engine.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone, Copy)]
pub struct Feature {
    /// Stable id, `area.name`.
    pub id: &'static str,
    pub area: &'static str,
    pub summary: &'static str,
    pub status: Status,
    /// Complete Windjammer program (has `fn main`).
    pub probe: &'static str,
    /// Supported: the exact stdout (lines joined by `\n`, no trailing newline). Unsupported: a substring of the error.
    pub expect: &'static str,
}

const fn ok(id: &'static str, area: &'static str, summary: &'static str, probe: &'static str, expect: &'static str) -> Feature {
    Feature { id, area, summary, status: Status::Supported, probe, expect }
}

const fn no(id: &'static str, area: &'static str, summary: &'static str, probe: &'static str, expect: &'static str) -> Feature {
    Feature { id, area, summary, status: Status::Unsupported, probe, expect }
}

pub const FEATURES: &[Feature] = &[
    ok("types.struct_impl", "Types", "structs, impls, associated functions, `Self`",
       "struct P { x: int }\nimpl P {\n    fn new(x: int) -> Self { P { x } }\n    fn get(self) -> int { self.x }\n}\nfn main() {\n    println(\"${P::new(3).get()}\")\n}\n", "3"),
    ok("types.enum_match", "Types", "enums with unit/tuple/struct variants; `match` with guards and or-patterns",
       "enum S { A(int), B, C { n: int } }\nfn f(s: S) -> int {\n    match s {\n        S::A(n) if n > 5 => 100,\n        S::A(n) => n * 2,\n        S::B | S::C { n: 0 } => 0,\n        S::C { n } => n,\n    }\n}\nfn main() {\n    println(\"${f(S::A(4))} ${f(S::A(9))} ${f(S::B)} ${f(S::C { n: 7 })}\")\n}\n", "8 100 0 7"),
    ok("types.tuples", "Types", "tuples and destructuring",
       "fn main() {\n    let (a, b) = (1, 2)\n    println(\"${a + b}\")\n}\n", "3"),
    ok("control.if_let_while_let", "Control flow", "`if let`, `while let`, `let else`",
       "fn first(v: Vec<int>) -> int {\n    let Some(x) = v.first() else { return -1 }\n    x\n}\nfn main() {\n    let mut s = vec![1, 2]\n    while let Some(x) = s.pop() {\n        println(\"${x}\")\n    }\n    if let Some(y) = Some(5) {\n        println(\"y=${y}\")\n    }\n    println(\"${first(vec![])}\")\n}\n", "2\n1\ny=5\n-1"),
    ok("control.loops", "Control flow", "`for` over ranges and iterators, `while`, `loop`/`break`/`continue`",
       "fn main() {\n    let mut t = 0\n    for i in 0..10 {\n        if i % 2 == 0 { continue }\n        t += i\n    }\n    let mut n = 0\n    loop {\n        n += 1\n        if n == 3 { break }\n    }\n    println(\"${t} ${n}\")\n}\n", "25 3"),
    ok("control.defer", "Control flow", "`defer`",
       "fn main() {\n    defer println(\"bye\")\n    println(\"hi\")\n}\n", "hi\nbye"),
    ok("traits.dispatch", "Traits and generics", "traits with default methods; dynamic dispatch by runtime type",
       "trait T {\n    fn a(self) -> int\n    fn b(self) -> int { self.a() + 1 }\n}\nstruct X {}\nimpl T for X {\n    fn a(self) -> int { 1 }\n}\nfn main() {\n    let x = X {}\n    println(\"${x.b()}\")\n}\n", "2"),
    ok("traits.generics", "Traits and generics", "generic functions (erased; dispatch is dynamic)",
       "fn id<T>(x: T) -> T { x }\nfn main() {\n    println(\"${id(5)}\")\n}\n", "5"),
    ok("traits.operators", "Traits and generics", "operator overloading via `add`, `neg`, `eq`, ...",
       "struct V { x: int }\nimpl Add for V {\n    fn add(self, o: V) -> V { V { x: self.x + o.x } }\n}\nfn main() {\n    let v = V { x: 1 } + V { x: 2 }\n    println(\"${v.x}\")\n}\n", "3"),
    ok("functions.closures", "Functions", "closures capturing by reference, mutation of captures, function values",
       "fn apply(f: fn(int) -> int, x: int) -> int { f(x) }\nfn main() {\n    let k = 3\n    let mut c = 0\n    let mut bump = || { c += 1 }\n    bump()\n    bump()\n    println(\"${apply(|x| x + k, 4)} ${c}\")\n}\n", "7 2"),
    ok("functions.pipe", "Functions", "pipe operator `|>`",
       "fn dbl(x: int) -> int { x * 2 }\nfn main() {\n    println(\"${3 |> dbl}\")\n}\n", "6"),
    ok("functions.question_mark", "Functions", "`?` on `Result`/`Option`",
       "fn f(x: int) -> Result<int, string> {\n    if x > 0 { Ok(x) } else { Err(\"neg\") }\n}\nfn g() -> Result<int, string> {\n    let a = f(1)?\n    let b = f(-1)?\n    Ok(a + b)\n}\nfn main() {\n    println(\"${g()}\")\n}\n", "Err(\"neg\")"),
    ok("format.interpolation", "Formatting", "`${}` interpolation",
       "fn main() {\n    let n = 5\n    println(\"v=${n + 1}\")\n}\n", "v=6"),
    ok("format.specs", "Formatting", "format specs: width, fill, align, precision, radix, `?`",
       "fn main() {\n    println(format(\"{:>4}|{:<3}|{:.1}|{:x}|{:08.3}\", 7, 1, 2.55, 255, 3.14159))\n}\n", "   7|1  |2.5|ff|0003.142"),
    ok("format.debug", "Formatting", "`{:?}` and `{:#?}` on collections and structs",
       "fn main() {\n    let v = vec![1, 2]\n    println(format(\"{:?}\", v))\n}\n", "[1, 2]"),
    ok("numbers.widths", "Numbers", "integer widths with Rust overflow semantics (`wrapping_*`, `checked_*`, `as` casts)",
       "fn main() {\n    let b: u8 = 200\n    let n: i32 = 300\n    println(\"${b.wrapping_add(100)} ${n as u8} ${b.checked_add(100).is_none()}\")\n}\n", "44 44 true"),
    ok("numbers.overflow_panics", "Numbers", "arithmetic overflow panics (exit 101)",
       "fn main() {\n    let x: i32 = 2147483647\n    let y = x + 1\n    println(\"${y}\")\n}\n", "panic: attempt to add with overflow"),
    ok("collections.vec", "Collections", "`Vec` push/len/index/sort/dedup, `VecDeque`",
       "fn main() {\n    let mut v = vec![3, 1, 3]\n    v.push(5)\n    v.sort()\n    v.dedup()\n    println(\"${v.len()} ${v[1]}\")\n}\n", "3 3"),
    ok("collections.maps_sets", "Collections", "`HashMap`/`BTreeMap`, `HashSet`/`BTreeSet`, `entry` API",
       "use std::collections::HashMap\nfn main() {\n    let mut m = HashMap::new()\n    m.insert(\"a\", 1)\n    *m.entry(\"a\").or_insert(0) += 5\n    println(\"${m.get(\"a\").unwrap()}\")\n}\n", "6"),
    ok("collections.iterators", "Collections", "iterator adaptors (`map filter fold sum zip enumerate rev take skip chain flat_map windows chunks sort_by`), lazy `cycle`/`repeat`/`from_fn`",
       "fn main() {\n    let v = vec![1, 2, 3]\n    let s: int = v.iter().map(|x| x * 2).sum()\n    let t: Vec<int> = (1..).map(|x| x * x).take(3).collect()\n    println(\"${s} ${t.len()}\")\n}\n", "12 3"),
    ok("collections.option_result", "Collections", "`Option`/`Result` combinators",
       "fn main() {\n    let o = Some(2)\n    println(\"${o.map(|x| x + 1).unwrap_or(0)}\")\n}\n", "3"),
    ok("strings.methods", "Strings", "string and char methods, `split`, `parse`",
       "fn main() {\n    println(\"${\"a,b\".split(\",\").count()}\")\n}\n", "2"),
    ok("memory.drop", "Memory", "`impl Drop`: destructors at scope exit, in reverse declaration order",
       "struct G { n: int }\nimpl Drop for G {\n    fn drop(self) { println(\"drop ${self.n}\") }\n}\nfn main() {\n    let a = G { n: 1 }\n    let b = G { n: 2 }\n    println(\"end\")\n}\n", "end\ndrop 2\ndrop 1"),
    ok("memory.rc_refcell", "Memory", "`Rc`/`Arc`, `RefCell`/`Cell`, `strong_count` by reachability",
       "use std::rc::Rc\nfn main() {\n    let a = Rc::new(5)\n    let b = a.clone()\n    println(\"${Rc::strong_count(&a)}\")\n}\n", "2"),
    ok("async.async_fn", "Concurrency", "`async fn` and `.await` (lazy futures run on `.await`)",
       "async fn two() -> int { 2 }\nasync fn main() {\n    let x = two().await\n    println(\"${x}\")\n}\n", "2"),
    ok("intrinsics.assert_debug", "Intrinsics", "`assert`, `assert_eq`, `panic`, `todo`, `debug`",
       "fn main() {\n    assert(1 + 1 == 2)\n    assert_eq(2, 2)\n    println(\"ok\")\n}\n", "ok"),
    no("concurrency.thread_block", "Concurrency", "`thread { }` and `async { }` blocks (ADR-0004: surface not settled; use `async fn` and `.await`)",
       "fn main() {\n    thread {\n        println(\"x\")\n    }\n}\n", "thread {}"),
    no("ffi.extern_fn", "FFI", "calls to `extern fn`",
       "extern fn c_abs(x: int) -> int\nfn main() {\n    println(\"${c_abs(-1)}\")\n}\n", "extern function"),
];

pub fn supported() -> impl Iterator<Item = &'static Feature> {
    FEATURES.iter().filter(|f| f.status == Status::Supported)
}

pub fn unsupported() -> impl Iterator<Item = &'static Feature> {
    FEATURES.iter().filter(|f| f.status == Status::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_well_formed() {
        for (i, f) in FEATURES.iter().enumerate() {
            assert!(f.id.contains('.') && !f.id.contains(' '), "{}", f.id);
            assert!(FEATURES[i + 1..].iter().all(|g| g.id != f.id), "duplicate id {}", f.id);
            assert!(f.probe.contains("fn main"), "{}: probe needs fn main", f.id);
        }
    }
}
