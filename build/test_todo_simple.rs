struct TodoItem {
    id: i64,
    text: String,
    completed: bool,
}

struct TodoApp {
    todos: Vec<TodoItem>,
}

fn main() {
    let app = TodoApp { todos: vec![TodoItem { id: 1, text: "Learn", completed: false }, TodoItem { id: 2, text: "Build", completed: false }] };
}

