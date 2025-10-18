use windjammer_ui::prelude::*;


#[derive(Debug, Clone)]
struct TodoItem {
    id: i64,
    text: String,
    completed: bool,
}

#[component]
struct TodoApp {
    todos: Vec<TodoItem>,
    next_id: i64,
    input_value: String,
}

impl TodoApp {
#[inline]
fn new(&self) -> TodoApp {
        TodoApp { todos: vec![TodoItem { id: 1, text: "Learn Windjammer", completed: false }, TodoItem { id: 2, text: "Build awesome apps", completed: false }], next_id: 3, input_value: "" }
}
}

fn main() {
    let app = TodoApp::new();
    println!("Done")
}

