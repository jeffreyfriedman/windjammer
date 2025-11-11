use windjammer_ui::prelude::*;
use windjammer_ui::components::*;
use windjammer_ui::simple_vnode::{VNode, VAttr};
use wasm_bindgen::prelude::*;



struct Todo {
    id: i32,
    text: String,
    completed: bool,
}

impl Todo {
#[inline]
fn new(&self, id: i32, text: &String) -> Todo {
        Todo { id, text, completed: false }
}
}

struct TodoState {
    todos: windjammer_ui::reactivity::Signal<Vec<Todo>>,
    next_id: windjammer_ui::reactivity::Signal<i32>,
}

impl TodoState {
#[inline]
fn new() -> TodoState {
        let initial_todos = vec![Todo::new(1, "Learn Windjammer".to_string()), Todo::new(2, "Build amazing UIs".to_string()), Todo::new(3, "Deploy to production".to_string())];
        TodoState { todos: Signal::new(initial_todos), next_id: Signal::new(4) }
}
#[inline]
fn toggle_todo(&self, id: i32) {
        let mut todos = self.todos.get();
        let mut i = 0;
        while i < self.todos.len() {
            if self.todos[i].id == id {
                self.todos[i].completed = !self.todos[i].completed;
            }
            i += 1;
        }
        self.todos.set(self.todos)
}
#[inline]
fn count_active(&self) -> i32 {
        let todos = self.todos.get();
        let mut count = 0;
        let mut i = 0;
        while i < self.todos.len() {
            if !self.todos[i].completed {
                count += 1;
            }
            i += 1;
        }
        count
}
}

#[inline]
fn render_todo_app(state: &TodoState) -> Container {
    let todos = state.todos.get();
    let active_count = state.count_active();
    let mut list = Flex::new().direction(FlexDirection::Column).gap("10px");
    let mut i = 0;
    while i < todos.len() {
        let todo = todos[i].clone();
        let state_toggle = state.clone();
        let todo_id = todo.id;
        let checkbox = {
            if todo.completed {
                "✓"
            } else {
                "○"
            }
        };
        let item = Flex::new().direction(FlexDirection::Row).gap("10px").child(Button::new(checkbox.to_string()).variant({
            if todo.completed {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Ghost
            }
        }).on_click(move || {
            state_toggle.toggle_todo(todo_id)
        })).child(Text::new(todo.text));
        list = list.child(item);
        i += 1;
    }
    Container::new().max_width("600px").child(Panel::new("📝 Simple Todo App".to_string()).child(Flex::new().direction(FlexDirection::Column).gap("20px").child(Text::new("Click checkboxes to toggle completion!".to_string())).child(list).child(Text::new(format!("{} items left", active_count)).size(TextSize::Small))))
}

#[inline]
#[wasm_bindgen]
pub fn start() {
    println!("📝 Starting Simple Todo App");
    let state = TodoState::new();
    let render = move || {
        render_todo_app(&state.clone()).to_vnode()
    };
    ReactiveApp::new("Simple Todo".to_string(), render).run()
}

fn main() {
    start()
}

