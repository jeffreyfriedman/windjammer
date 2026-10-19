# Compiled: a local `Vec::new()` whose element type comes from a method call is annotated with the function's return element type

```windjammer
struct Delivery {
    conn: int,
}

struct Conn {
    id: int,
    user: string,
}

struct Hub {
    conns: Vec<Conn>,
}

impl Hub {
    fn members(self) -> Vec<Conn> {
        let mut out = Vec::new()
        for c in self.conns {
            out.push(Conn { id: c.id, user: c.user.clone() })
        }
        out
    }

    fn presence(self) -> Vec<Delivery> {
        let members = self.members()
        let mut users = Vec::new()
        for m in members {
            users.push(m.user.clone())
        }
        users.sort()
        let mut out = Vec::new()
        out.push(Delivery { conn: users.len() })
        out
    }
}

fn main() {
    let hub = Hub { conns: Vec::new() }
    println("${hub.presence().len()}")
}
```

`wj build` emits `let mut users: Vec<Delivery> = Vec::new();` (the return type of `presence`) although `users` only receives
`String`s (`E0308`, `E0277: Delivery: Ord`). The element type of `m.user.clone()` (`m` iterates a `Vec<Conn>` returned by a
method) is not resolved and the annotation falls back to the enclosing function's return element type. The same program
with `users.push("x")` instead works. Also here: `Delivery { conn: users.len() }` needs `usize` -> `int` (`E0308`).
