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
        self.todos = self.todos.filter(|t| {
            t.id != id;
        });
}
#[inline]
fn render(&self) -> VNode {
        VElement::new("div").attr("class", "todo-app").child(render_header()).child(render_input()).child(render_list()).child(render_stats()).into()
}
#[inline]
fn render_header(&self) -> VNode {
        VElement::new("h1").child(VNode::Text(VText::new("📝 Windjammer Todos"))).into()
}
#[inline]
fn render_input(&self) -> VNode {
        VElement::new("div").attr("class", "input-section").child(VNode::Element(VElement::new("input").attr("type", "text").attr("placeholder", "What needs to be done?").attr("value", self.input_value))).child(VNode::Element(VElement::new("button").attr("class", "add-btn").child(VNode::Text(VText::new("Add"))))).into()
}
#[inline]
fn render_list(&self) -> VNode {
        let mut ul = VElement::new("ul").attr("class", "todo-list");
        for todo in self.todos {
            ul = ul.child(render_todo_item(&todo));
        }
        ul.into()
}
#[inline]
fn render_todo_item(&self, todo: &TodoItem) -> VNode {
        let class = {
            if todo.completed {
                "todo-item completed"
            } else {
                "todo-item"
            }
        };
        VElement::new("li").attr("class", class).attr("data-id", todo.id::to_string()).child(VNode::Element(VElement::new("input").attr("type", "checkbox").attr("checked", {
            if todo.completed {
                "checked"
            } else {
                ""
            }
        }))).child(VNode::Element(VElement::new("span").attr("class", "todo-text").child(VNode::Text(VText::new(todo.text))))).child(VNode::Element(VElement::new("button").attr("class", "delete-btn").child(VNode::Text(VText::new("🗑️"))))).into()
}
#[inline]
fn render_stats(&self) -> VNode {
        let total = self.todos.len();
        let completed = self.todos.filter(|t| {
            t.completed;
        }).len();
        let remaining = total - completed;
        VElement::new("div").attr("class", "stats").child(VNode::Text(VText::new(format!("Total: {} | Completed: {} | Remaining: {}", total, completed, remaining)))).into()
}
}

fn main() {
    println!("🎨 Windjammer Todo App Example");
    println!("================================
");
    let mut app = TodoApp::new();
    println!("Initial state:");
    let vnode = app.render();
    println!("{vnode:#?}
");
    println!("Adding todo: 'Write more examples'");
    app.add_todo("Write more examples");
    println!("Total todos: {app.todos.len()}
");
    println!("Completing todo #1");
    app.toggle_todo(1);
    let completed = app.todos::filter(|t| {
        t.completed;
    }).len();
    println!(format!("Completed: {}/{app.todos.len()}", completed));
    use windjammer_ui.ssr.SSRRenderer;
    let mut renderer = SSRRenderer::new();
    let html = renderer.render_to_string(app);
    println!("
📄 Server-Side Rendered HTML:");
    println!(html)
}

