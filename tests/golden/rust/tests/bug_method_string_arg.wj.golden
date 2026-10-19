#[derive(Debug, Clone, PartialEq, Eq, Default)]

struct State {
    pub names: Vec<String>,
}

impl State {
#[inline]
pub fn new() -> State {
        State { names: vec![] }
}
#[inline]
pub fn get_value(&self, name: &str) -> i32 {
        42_i32
}
}

#[derive(Clone, Debug, PartialEq)]
enum Condition {
    Check(String, i32),
}

impl Condition {
#[inline]
pub fn is_met(&self, state: &State) -> bool {
        match self {
            Condition::Check(name, threshold) => {
                return state.get_value(name) > *threshold;
            },
        }
}
}

fn main() {
    let cond = Condition::Check(String::from("test"), 10_i32);
    let state = State::new();
    let result = cond.is_met(&state);
    println!("Result: {}", result);
}

