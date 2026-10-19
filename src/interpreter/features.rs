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

const fn ok(
    id: &'static str,
    area: &'static str,
    summary: &'static str,
    probe: &'static str,
    expect: &'static str,
) -> Feature {
    Feature {
        id,
        area,
        summary,
        status: Status::Supported,
        probe,
        expect,
    }
}

const fn no(
    id: &'static str,
    area: &'static str,
    summary: &'static str,
    probe: &'static str,
    expect: &'static str,
) -> Feature {
    Feature {
        id,
        area,
        summary,
        status: Status::Unsupported,
        probe,
        expect,
    }
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
       "fn f(x: int) -> Result<int, string> {\n    if x > 0 { Ok(x) } else { Err(\"neg\") }\n}\nfn g() -> Result<int, string> {\n    let a = f(1)?\n    let b = f(-1)?\n    Ok(a + b)\n}\nfn main() {\n    println(format(\"{:?}\", g()))\n}\n", "Err(\"neg\")"),
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
       "use std::collections::HashMap\nfn main() {\n    let mut m: HashMap<string, int> = HashMap::new()\n    m.entry(\"a\").or_insert(7)\n    m.entry(\"a\").or_insert(9)\n    let v = m.get(\"a\").unwrap()\n    println(\"${v}\")\n}\n", "7"),
    ok("collections.iterators", "Collections", "iterator adaptors (`map filter fold sum zip enumerate rev take skip chain flat_map windows chunks sort_by`), lazy `cycle`/`repeat`/`from_fn`",
       "fn main() {\n    let v = vec![1, 2, 3]\n    let s: int = v.iter().map(|x| x * 2).sum()\n    let t: Vec<int> = (1..).map(|x| x * x).take(3).collect()\n    println(\"${s} ${t.len()}\")\n}\n", "12 3"),
    ok("collections.option_result", "Collections", "`Option`/`Result` combinators",
       "fn main() {\n    let o = Some(2)\n    println(\"${o.map(|x| x + 1).unwrap_or(0)}\")\n}\n", "3"),
    ok("strings.methods", "Strings", "string and char methods, `split`, `parse`",
       "fn main() {\n    let s = \"a,b\"\n    let n = s.split(\",\").count()\n    println(\"${n}\")\n}\n", "2"),
    ok("memory.drop", "Memory", "`impl Drop`: destructors at scope exit, in reverse declaration order",
       "struct G { n: int }\nimpl Drop for G {\n    fn drop(self) { println(\"drop ${self.n}\") }\n}\nfn main() {\n    let a = G { n: 1 }\n    let b = G { n: 2 }\n    println(\"end\")\n}\n", "end\ndrop 2\ndrop 1"),
    ok("memory.rc_refcell", "Memory", "`Rc`/`Arc`, `RefCell`/`Cell`, `strong_count` by reachability",
       "use std::rc::Rc\nfn main() {\n    let a = Rc::new(5)\n    let b = a.clone()\n    println(\"${Rc::strong_count(&a)}\")\n}\n", "2"),
    ok("async.async_fn", "Concurrency", "`async fn` and `.await` (lazy futures run on `.await`)",
       "async fn two() -> int { 2 }\nasync fn main() {\n    let x = two().await\n    println(\"${x}\")\n}\n", "2"),
    ok("intrinsics.assert_debug", "Intrinsics", "`assert`, `assert_eq`, `panic`, `todo`, `debug`",
       "fn main() {\n    assert(1 + 1 == 2)\n    assert_eq(2, 2)\n    println(\"ok\")\n}\n", "ok"),
    ok("concurrency.threads", "Concurrency", "`thread::spawn`/`join`, `thread::scope`, `sleep` on a virtual clock (deterministic cooperative scheduler; `--sched-seed N` explores other interleavings)",
       "use std::thread\nuse std::thread::JoinHandle\nfn main() {\n    let mut hs: Vec<JoinHandle<int>> = Vec::new()\n    for i in 0..3 {\n        hs.push(thread::spawn(move || { return i * 10 }))\n    }\n    let mut t = 0\n    for h in hs { t += h.join().unwrap_or(0) }\n    println(\"${t}\")\n}\n", "30"),
    ok("concurrency.channels", "Concurrency", "`mpsc::channel`/`sync_channel`: send, recv, try_recv, recv_timeout, iteration; closes when every sender is gone",
       "use std::sync::mpsc\nuse std::thread\nfn main() {\n    let (tx, rx) = mpsc::channel()\n    for i in 0..3 {\n        let t = tx.clone()\n        thread::spawn(move || { t.send(i) })\n    }\n    drop(tx)\n    let mut s = 0\n    for v in rx { s += v }\n    println(\"${s}\")\n}\n", "3"),
    ok("concurrency.sync", "Concurrency", "`Arc<Mutex>`/`RwLock` guards (released at scope end), `Condvar`, `Barrier`, atomics; deadlocks are reported",
       "use std::sync::{Arc, Mutex}\nuse std::sync::atomic::{AtomicUsize, Ordering}\nuse std::thread\nfn main() {\n    let m = Arc::new(Mutex::new(0))\n    let a = Arc::new(AtomicUsize::new(0))\n    let m2 = m.clone()\n    let a2 = a.clone()\n    let h = thread::spawn(move || {\n        match m2.lock() { Ok(mut g) => { *g += 5 } Err(_) => {} }\n        a2.fetch_add(2, Ordering::SeqCst)\n    })\n    h.join()\n    let v = match m.lock() { Ok(g) => *g, Err(_) => 0 }\n    println(\"${v} ${a.load(Ordering::SeqCst)}\")\n}\n", "5 2"),
    ok("memory.refcell_borrow_panics", "Memory", "`RefCell` borrow tracking with Rust's `BorrowMutError` panic",
       "use std::cell::RefCell\nfn main() {\n    let c = RefCell::new(1)\n    let a = c.borrow_mut()\n    let b = c.borrow_mut()\n}\n", "panic: already borrowed: BorrowMutError"),
    ok("std.time", "Std modules", "`Duration`, `Instant`, `SystemTime`, `std::time::DateTime` (chrono formats)",
       "use std::time\nfn main() {\n    match time::parse_rfc3339(\"2024-03-05T10:20:30Z\") {\n        Ok(d) => println(d.format(\"%Y/%m/%d\")),\n        Err(e) => println(e)\n    }\n}\n", "2024/03/05"),
    ok("std.json", "Std modules", "`std::json` dynamic values and typed `to_string`/`parse_string::<T>`",
       "use std::json\nstruct P { x: int, y: int }\nfn main() {\n    println(json::to_string(P { x: 1, y: 2 }).unwrap_or(\"\"))\n}\n", "{\"x\":1,\"y\":2}"),
    ok("std.regex", "Std modules", "`std::regex` (same engine as compiled programs)",
       "use std::regex\nfn main() {\n    match regex::compile(\"\\\\d+\") {\n        Ok(re) => println(re.replace_all(\"a1b22\", \"#\")),\n        Err(e) => println(e)\n    }\n}\n", "a#b#"),
    ok("std.fs", "Std modules", "`std::fs` files, directories (`read_dir`, `metadata`), path helpers",
       "use std::fs\nfn main() {\n    let e = fs::extension(\"a/b.txt\").unwrap_or(\"-\")\n    let s = fs::file_stem(\"a/b.txt\").unwrap_or(\"-\")\n    println(\"${e} ${s}\")\n}\n", "txt b"),
    ok("std.data", "Std modules", "`csv`, `url`, `mime`, `crypto` (sha/base64/bcrypt), `yaml`, `compress`, `jwt`, `uuid`, seeded `random`",
       "use std::crypto\nfn main() {\n    println(crypto::base64_encode(\"hi\"))\n}\n", "aGk="),
    ok("std.http", "Std modules", "`std::http` client and servers (`serve`, `serve_fn`, `Server::serve`) behind a capability port: refused by default, `--mock-http FILE` replays responses/requests, `--allow-network` uses real sockets",
       "use std::http\nfn main() {\n    match http::get(\"http://example.invalid/\") { Ok(_) => println(\"x\"), Err(e) => println(\"${e.starts_with(\"network access is disabled\")}\") }\n}\n", "true"),
    ok("std.config_db", "Std modules", "`std::config` (toml/yaml, same code as the runtime), `std::db` on SQLite, `std::subprocess` pipes, `std::cli` (clap)",
       "use std::config\nfn main() {\n    println(config::to_json(\"a = 1\", \"toml\").unwrap_or(\"\"))\n}\n", "{\"a\":1}"),
    ok("memory.moves", "Memory", "moves decide where destructors run: into collections, aggregates, `let`, partial moves, callee parameters",
       "struct G { n: int }\nimpl Drop for G {\n    fn drop(self) { println(\"drop ${self.n}\") }\n}\nfn main() {\n    let a = G { n: 1 }\n    {\n        let mut v: Vec<G> = Vec::new()\n        v.push(a)\n    }\n    println(\"end\")\n}\n", "drop 1\nend"),
    ok("numbers.u128", "Numbers", "`u128` over its full range",
       "fn main() {\n    let m = u128::MAX\n    println(\"${m} ${m / 3}\")\n}\n", "340282366920938463463374607431768211455 113427455640312821154458202477256070485"),
    ok("collections.ordered", "Collections", "`OrderedMap`/`OrderedSet`: insertion order, `remove` keeps the order of the rest; `HashMap`/`HashSet` order is unspecified (`--hash-seed N` permutes it)",
       "fn main() {\n    let mut m: OrderedMap<string, int> = OrderedMap::new()\n    m.insert(\"b\", 2)\n    m.insert(\"a\", 1)\n    m.insert(\"c\", 3)\n    m.remove(\"a\")\n    for k in m.keys() { println(k) }\n}\n", "b\nc"),
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
            assert!(
                FEATURES[i + 1..].iter().all(|g| g.id != f.id),
                "duplicate id {}",
                f.id
            );
            assert!(f.probe.contains("fn main"), "{}: probe needs fn main", f.id);
        }
    }
}
