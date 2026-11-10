# No annotations, the same Rust

<!-- feature: showcase-ownership -->

Rust's ownership rules are what make its programs fast and safe, and also what a newcomer meets first. Windjammer keeps the
rules and moves the bookkeeping into the compiler. You write values and functions; the compiler decides, for every
parameter, whether to move, borrow or borrow mutably, and writes the answer into the Rust it generates.

Here is a playlist with one function that changes it and two that only read it. The Windjammer source carries no `&`, no
`&mut`, no lifetime and no `.clone()`:

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

And this is the Rust the compiler generates for it, checked by the book's tests against the real compiler output:

```rust,generated
#[derive(Debug, Clone, PartialEq, Eq, Default)]

struct Playlist {
    name: String,
    songs: Vec<String>,
}

#[inline]
fn add_song(list: &mut Playlist, song: String) {
    list.songs.push(song);
}

#[inline]
fn count(list: &Playlist) -> i64 {
    (list.songs.len() as i64)
}

#[inline]
fn describe(list: &Playlist) -> String {
    format!("{}: {}", list.name, list.songs.join(", "))
}

fn main() {
    let mut list = Playlist { name: String::from("Road trip"), songs: vec![] };
    add_song(&mut list, "Africa".to_string());
    add_song(&mut list, "Hey Jude".to_string());
    println!("{}", describe(&list));
    println!("{} songs", count(&list));
}
```

Read the signatures. `add_song` changes the playlist, so the compiler gave it `&mut Playlist`; `count` and `describe` only
read, so they got `&Playlist`. The caller keeps using `list` after every call, because nothing was taken from it. The
derives on the struct were inferred from how it is used. This is the code a careful Rust programmer would write by hand; the
difference is who wrote it.

## Seeing the decision

You never have to guess what the compiler chose. `wj explain-ownership` prints the decision for every parameter and the rule
behind it:

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
fn count
param list: Playlist  ->  &
```

And `wj build --no-cargo` writes the Rust out, so you can read it, review it or take it with you.

## Values stay independent

The model you program against is simple: every assignment, argument and return behaves like a copy. The compiler chooses a
move, a borrow or a clone to make that true. This program changes a copy and leaves the original alone, with no clone in
sight:

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

When the inference is not yet as sharp as it will be, the failure is visible and reportable rather than silent: the compiled
path rejects the program with a diagnostic on your source, and [Known gaps](https://github.com/jeffreyfriedman/windjammer/blob/main/docs/known-gaps.md) lists what is being
tightened. The full rules are in [Ownership rules](../reference/ownership-rules.md).
