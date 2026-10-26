#[derive(Clone, Debug, PartialEq)]
enum Action {
    Update(String, i32),
}

impl Action {
#[inline]
pub fn execute(self) {
        match self {
            Action::Update(name, value) => {
                process(&name, value);
            },
        }
}
}

fn main() {
    let mut action = Action::Update("test".to_string(), 42_i32);
    action.execute();
}

#[inline]
fn process(name: &str, value: i32) {
    println!("Name: {}, Value: {}", name, value);
}

