use smallvec::{SmallVec, smallvec};

struct TodoItem {
    id: i64,
    text: String,
    completed: bool,
}

#[component]
struct TodoApp {
    todos: Vec<TodoItem>,
}

impl TodoApp {
#[inline]
fn new(&self) -> TodoApp {
        TodoApp { todos: vec![TodoItem { id: 1, text: "Learn", completed: false }, TodoItem { id: 2, text: "Build", completed: false }] }
}
}

fn main() {
    let app: SmallVec<[_; 4]> = TodoApp::new().into();
}

