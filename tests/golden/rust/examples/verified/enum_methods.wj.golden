#[derive(Clone, Debug, PartialEq)]
enum Command {
    Quit,
    Move(i64, i64),
    Say(String),
}

impl Command {
#[inline]
fn describe(&self) -> String {
        match self {
            Command::Quit => String::from("exit"),
            Command::Move(x, y) => format!("moved to {},{}", x, y),
            Command::Say(s) => format!("says {}", s),
        }
}
#[inline]
fn name(&self) -> String {
        match self {
            Command::Quit => String::from("Quit"),
            Command::Move(x, y) => format!("Move({}, {})", x, y),
            Command::Say(s) => format!("Say({})", s),
        }
}
}

fn main() {
    let cmds = vec![Command::Quit, Command::Move(2_i32, 3_i32), Command::Say(String::from("hi"))];
    for c in cmds {
        println!("{} -> {}", c.name(), c.describe());
    }
}

