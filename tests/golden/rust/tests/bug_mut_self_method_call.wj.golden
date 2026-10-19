#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct State {
    pub value: i32,
}









impl State {
#[inline]
pub fn new() -> State {
        State { value: 0_i32 }
}
#[inline]
pub fn change_value(&self, name: &str, delta: i32) {
        println!("Changing: {}, delta: {}", name, delta);
}
}

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Change(String, i32),
}

impl Action {
#[inline]
pub fn execute(self, state: State) {
        match self {
            Action::Change(name, delta) => {
                state.change_value(&name, delta);
            },
        }
}
}

fn main() {
    let mut action = Action::Change(String::from("test"), 10_i32);
    let mut state = State::new();
    action.execute(state);
}

