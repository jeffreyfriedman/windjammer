


use windjammer_ui::prelude::*;

use windjammer_ui::vdom::{VElement, VNode, VText};


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
#[inline]
fn add_todo(&self, text: &String) {
        if !text.is_empty() {
            self.todos.push(TodoItem { id: self.next_id, text, completed: false });
            self.next_id += 1;
            self.input_value = "";
        }
}
#[inline]
fn toggle_todo(&self, id: i64) {
        for todo in self.todos {
            if todo.id == id {
                todo.completed = !todo.completed;
                break;
            }
        }
}
#[inline]
fn delete_todo(&self, id: i64) {
        self.todos = self.todos.filter(|t| t.id != id);
}
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "todo-app").child(render_header()).child(render_input()).child(render_list()).child(render_stats()).into()
}
#[inline]
fn render_header(&self) -> VNode {
        VElement::new("h1").child(VNode::Text(VText::new("📝 Windjammer Todos"))).into()
}
}

fn main() {
}

