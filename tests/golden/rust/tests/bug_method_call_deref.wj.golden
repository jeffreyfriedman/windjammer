#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct State {
    pub name: String,
    pub value: i32,
}

impl State {
#[inline]
pub fn new() -> State {
        State { name: "".to_string(), value: 0_i32 }
}
#[inline]
pub fn update(&self, name: &str, value: i32) {
        println!("Updating: {}, {}", name, value);
}
}

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Update(String, i32),
}

impl Action {
#[inline]
pub fn execute(self, state: &State) {
        match self {
            Action::Update(name, value) => {
                state.update(&name, value);
            },
        }
}
}

fn main() {
    let mut action = Action::Update("test".to_string(), 42_i32);
    let state = State::new();
    action.execute(&state);
}

