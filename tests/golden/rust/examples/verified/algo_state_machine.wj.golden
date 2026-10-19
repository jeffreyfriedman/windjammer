#[derive(Clone, Debug, PartialEq, Copy)]
enum State {
    Idle,
    Running,
    Paused,
    Stopped,
}

#[inline]
fn name(s: State) -> String {
    match s {
        State::Idle => String::from("Idle"),
        State::Running => String::from("Running"),
        State::Paused => String::from("Paused"),
        State::Stopped => String::from("Stopped"),
    }
}

#[inline]
fn next(s: State, event: String) -> State {
    match (s, event) {
        (State::Idle, "start") => State::Running,
        (State::Running, "pause") => State::Paused,
        (State::Paused, "resume") => State::Running,
        (State::Running, "stop") => State::Stopped,
        (other, _) => other,
    }
}

fn main() {
    let mut s = State::Idle;
    let mut trail: Vec<String> = vec![name(State::Idle)];
    for e in ["start", "pause", "bogus", "resume", "stop"] {
        s = next(s, e.clone());
        if e != "bogus" {
            let _ = trail.push(name(s));
        }
    }
    println!("{}", trail.join(" -> "));
}

