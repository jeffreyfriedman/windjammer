



use windjammer_ui::prelude::*;

use windjammer_ui::vdom::{VElement, VNode, VText}::*;

use windjammer_ui::reactivity::Signal::*;


struct Todo {
    id: i64,
    text: String,
    completed: bool,
}

#[component]
struct TodoApp {
    todos: Signal<Vec<Todo>>,
    input: Signal<String>,
    filter: Signal<String>,
    next_id: Signal<i64>,
}

impl Component for TodoApp {
#[inline]
fn render(&self) -> VNode {
        let current_input = self.input.get();
        let filtered_todos = get_filtered_todos();
        let active_count = get_active_count();
        VElement::new("div").attr("class", "todo-app").child(VNode::Element(VElement::new("h1").child(VNode::Text(VText::new("Windjammer Todos"))))).child(VNode::Element(VElement::new("input").attr("type", "text").attr("placeholder", "What needs to be done?").attr("value", current_input).attr("oninput", "update_input"))).child(render_todo_list(&filtered_todos)).child(VNode::Text(VText::new(format!("{} items left", active_count)))).into()
}
}

impl TodoApp {
#[inline]
fn new(&self) -> Self {
        Self { todos: Signal::new(Vec::new()), input: Signal::new(String::new()), filter: Signal::new("all".to_string()), next_id: Signal::new(0) }
}
#[inline]
fn render_todo_list(&self, todos: &Vec<Todo>) -> VNode {
        let items = self.todos.iter().map(|todo| {
            VElement::new("li").child(VNode::Text(VText::new(todo.text::clone()))).into();
        }).collect();
        VElement::new("ul").children(items).into()
}
#[inline]
fn add_todo(&self) {
        let text = self.input.get();
        if text.len() > 0 {
            let id = self.next_id.get();
            self.todos.update(|list| {
                list.push(Todo { id, text: text, completed: false });
            });
            self.next_id.update(|n| {
                *n = *n + 1;
            });
            self.input.set(String::new())
        }
}
#[inline]
fn get_filtered_todos(&self) -> Vec<Todo> {
        self.todos.get()
}
#[inline]
fn get_active_count(&self) -> i64 {
        self.todos.get().iter().filter(|t| !t.completed).count()
}
#[inline]
fn update_input(&self, value: &String) {
        self.input.set(value)
}
}

fn main() {
    let app = TodoApp::new();
    mount("#app", app)
}

