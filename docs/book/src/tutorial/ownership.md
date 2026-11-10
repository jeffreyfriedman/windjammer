# Ownership and borrowing, inferred
<!-- feature: ownership -->

<!-- promise: feature ownership-inferred -->
<!-- promise: feature ownership-visible -->

Every program has to decide who owns each value, who may read it, who may change it, and when a copy is needed. Rust
asks you to write those decisions into the program: `&`, `&mut`, lifetimes, `.clone()`. Windjammer makes the same
decisions for you, with the same rules, and shows you the result on request.

You write the program as if every value were independent: passing a value to a function, assigning it, or returning
it gives the receiver its own copy as far as the program can tell. The compiler then chooses the cheapest way to make
that true: a move, a shared borrow, a mutable borrow, or a clone.

## Reading, changing, consuming

Here is a playlist with one function that changes it and two that only read it. Nothing in the source says which is
which:

```wj
// expect: Road trip: Africa, Hey Jude
// expect: 2 songs
struct Playlist {
    name: string,
    songs: Vec<string>,
}

fn add_song(list: Playlist, song: string) {
    list.songs.push(song)
}

fn count(list: Playlist) -> int {
    list.songs.len()
}

fn describe(list: Playlist) -> string {
    "${list.name}: ${list.songs.join(", ")}"
}

fn main() {
    let mut list = Playlist { name: "Road trip", songs: [] }
    add_song(list, "Africa")
    add_song(list, "Hey Jude")
    println(describe(list))
    println("${count(list)} songs")
}
```

[Run this in the playground](../playground/index.html#lang=wj&code=Ly8gZXhwZWN0OiBSb2FkIHRyaXA6IEFmcmljYSwgSGV5IEp1ZGUKLy8gZXhwZWN0OiAyIHNvbmdzCnN0cnVjdCBQbGF5bGlzdCB7CiAgICBuYW1lOiBzdHJpbmcsCiAgICBzb25nczogVmVjPHN0cmluZz4sCn0KCmZuIGFkZF9zb25nKGxpc3Q6IFBsYXlsaXN0LCBzb25nOiBzdHJpbmcpIHsKICAgIGxpc3Quc29uZ3MucHVzaChzb25nKQp9CgpmbiBjb3VudChsaXN0OiBQbGF5bGlzdCkgLT4gaW50IHsKICAgIGxpc3Quc29uZ3MubGVuKCkKfQoKZm4gZGVzY3JpYmUobGlzdDogUGxheWxpc3QpIC0-IHN0cmluZyB7CiAgICAiJHtsaXN0Lm5hbWV9OiAke2xpc3Quc29uZ3Muam9pbigiLCAiKX0iCn0KCmZuIG1haW4oKSB7CiAgICBsZXQgbXV0IGxpc3QgPSBQbGF5bGlzdCB7IG5hbWU6ICJSb2FkIHRyaXAiLCBzb25nczogW10gfQogICAgYWRkX3NvbmcobGlzdCwgIkFmcmljYSIpCiAgICBhZGRfc29uZyhsaXN0LCAiSGV5IEp1ZGUiKQogICAgcHJpbnRsbihkZXNjcmliZShsaXN0KSkKICAgIHByaW50bG4oIiR7Y291bnQobGlzdCl9IHNvbmdzIikKfQo)

The compiler reads each function body. `add_song` pushes onto a field of `list`, so `list` is borrowed mutably.
`count` and `describe` only read, so they borrow it shared. Neither read-only function takes the playlist away from
`main`, which is why `main` can keep using `list` afterwards.

## Ask the compiler what it chose

`wj explain-ownership` prints the decision for every parameter, with the rule that made it:

```sh,verify
$ cat > playlist.wj <<'EOF'
struct Playlist {
    name: string,
    songs: Vec<string>,
}

fn add_song(list: Playlist, song: string) {
    list.songs.push(song)
}

fn count(list: Playlist) -> int {
    list.songs.len()
}

fn main() {
    let mut list = Playlist { name: "Road trip", songs: [] }
    add_song(list, "Africa")
    println("${count(list)}")
}
EOF
$ wj explain-ownership playlist.wj
fn add_song
param list: Playlist  ->  &mut
OWN-MUT-RECEIVER
fn count
param list: Playlist  ->  &
```

Each line names a rule, such as `OWN-MUT-RECEIVER` (the value is the receiver of a method that changes it), so a
surprising choice is never a mystery. For the full picture, `wj build --no-cargo` writes the Rust the program
compiles to:

```sh,verify
$ cat > playlist.wj <<'EOF'
struct Playlist {
    name: string,
    songs: Vec<string>,
}

fn add_song(list: Playlist, song: string) {
    list.songs.push(song)
}

fn count(list: Playlist) -> int {
    list.songs.len()
}

fn main() {
    let mut list = Playlist { name: "Road trip", songs: [] }
    add_song(list, "Africa")
    println("${count(list)}")
}
EOF
$ wj build --no-cargo playlist.wj -o out
$ cat out/playlist.rs
fn add_song(list: &mut Playlist, song: String) {
fn count(list: &Playlist) -> i64 {
```

That is exactly the signature a Rust programmer would have written by hand.

## Methods

A method takes plain `self`. Whether the method reads, changes or consumes its receiver is inferred from the body.
When a method changes the receiver, the caller must hold the value in a `let mut`, which keeps mutation visible at the
call site:

```wj
// expect: 7
struct Total {
    sum: int,
}

impl Total {
    fn add(self, n: int) {
        self.sum += n
    }

    fn value(self) -> int {
        self.sum
    }
}

fn main() {
    let mut t = Total { sum: 0 }
    t.add(2)
    t.add(5)
    println("${t.value()}")
}
```

A method that builds a new value from `self` and returns it consumes the receiver. That is how builders chain, each
step handing its result to the next:

```wj
// expect: 3 x
struct Builder {
    n: int,
    tag: string,
}

impl Builder {
    fn new() -> Builder {
        Builder { n: 0, tag: "" }
    }

    fn n(self, v: int) -> Builder {
        Builder { n: v, tag: self.tag }
    }

    fn tag(self, t: string) -> Builder {
        Builder { n: self.n, tag: t }
    }
}

fn main() {
    let b = Builder::new().n(3).tag("x")
    println("${b.n} ${b.tag}")
}
```

## Values stay independent

Because every assignment behaves like a copy, changing one name never changes another. Where the compiler needs a real
copy to keep that true, it inserts the clone itself:

```wj
// expect: 1 50 2
struct P {
    x: int,
}

fn bump(p: P) -> P {
    let mut q = p
    q.x += 1
    q
}

fn main() {
    let a = P { x: 1 }
    let mut b = a
    b.x = 50
    let c = bump(a)
    println("${a.x} ${b.x} ${c.x}")
}
```

Lists and fields hold independent copies too:

```wj
// expect: 9 5
struct P {
    x: int,
}

fn main() {
    let mut p = P { x: 1 }
    let mut v = [p]
    p.x = 9
    v[0].x = 5
    println("${p.x} ${v[0].x}")
}
```

`.clone()` is always available when you want to say it out loud, and it never changes what the program means. The
linter notes it (`W0005`) because the compiler's own copy is usually enough:

```wj
// expect: 3 4
fn main() {
    let a = [1, 2, 3]
    let mut b = a.clone()
    b.push(4)
    println("${a.len()} ${b.len()}")
}
```

## Mutating through a parameter

A function that changes a parameter changes the caller's value; there is no `&mut` at the call site or in the
signature:

```wj
// expect: 4 2
fn fill(v: Vec<int>, n: int) {
    let mut i = 0
    while i < n {
        v.push(i)
        i += 1
    }
}

fn main() {
    let mut v = [100]
    fill(v, 3)
    println("${v.len()} ${v[3]}")
}
```

## What you can skip

You do not write `&`, `&mut`, lifetimes, `.iter()`, `.unwrap()`, `.to_string()` or `#[derive(...)]` in everyday code.
`wj lint` points out the Rust habits that creep in, and `wj fix` rewrites them. When you do write a reference in a
signature, the compiler accepts it as a hint.

## When you want control

Inference covers the ordinary cases, and the generated Rust is always there to read. For the places where a project
needs a particular layout, such as arenas for bulk allocation or explicit widths at a binary boundary, the language has
explicit tools, introduced in [Numbers and sizes](numbers.md) and the later chapters. The ownership inference rules are
specified in detail in the repository under `docs/spec/ownership.md` and `docs/spec/inference.md`, and
[Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) lists where the inference is still being tightened.
