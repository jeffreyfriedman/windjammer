# Compiled: `Vec<string>.contains(name)` makes the parameter `&String`, so a string-literal argument no longer type-checks

```windjammer
struct Room {
    members: Vec<string>,
}

impl Room {
    fn has(self, name: string) -> bool {
        self.members.contains(name)
    }
}

fn main() {
    let mut members = Vec::new()
    members.push("ann")
    let room = Room { members: members }
    println("${room.has("ann")}")
}
```

`wj build` emits `fn has(&self, name: &String)` and the call `room.has("ann")` fails (`E0308: expected &String, found
&'static str`). Inside the crate, a `&str` parameter used as `rooms.contains(room)` fails the other way (`expected &String,
found &str`). A `string` parameter should keep the `&str` form callers can pass, with the `contains` argument converted.

Status: fixed. `Vec<string>.contains(x)` with a text argument lowers to `v.iter().any(|m| *m == *x)`, so string parameters keep the `&str` form (the `&String` formal promotion is gone). Gates: `ecosystem_findings_test::vec_string_contains_param_accepts_string_literal`, `vec_string_contains_literal_and_local`.
