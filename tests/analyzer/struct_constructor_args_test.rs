//! TDD Test: Struct constructor argument handling
//!
//! Tests that when calling struct constructors (like Node::new(name)),
//! the arguments are correctly converted based on the parameter ownership.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_struct_with_borrowed_param() {
    // When a method has a borrowed string param and we use it to create a struct,
    // the struct constructor should receive the correct type
    let code = r#"
pub struct Node {
    id: string,
    name: string,
}

impl Node {
    pub fn new(id: string, name: string) -> Node {
        Node { id: id, name: name }
    }
}

pub struct Editor {
    nodes: Vec<Node>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor { nodes: Vec::new() }
    }
    
    // name is borrowed (only passed to struct constructor)
    pub fn add_node(&mut self, id: string, name: string) {
        self.nodes.push(Node::new(id, name))
    }
}

pub fn test_editor() {
    let mut editor = Editor::new()
    editor.add_node("1", "First")
    editor.add_node("2", "Second")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_cloned_struct_to_vec() {
    // When iterating and pushing clones, the vec should work correctly
    // WINDJAMMER FIX: No need for .as_str() when parameter is already inferred to &str
    let code = r#"
pub struct Item {
    name: string,
}

pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: Vec::new() }
    }
    
    pub fn filter_items(&self, prefix: string) -> Vec<Item> {
        let mut result = Vec::new()
        for item in self.items.iter() {
            if item.name.starts_with(prefix) {
                result.push(item.clone())
            }
        }
        result
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_constructor_with_multiple_string_params() {
    // Constructor with multiple string params that are stored
    let code = r#"
pub struct Person {
    first_name: string,
    last_name: string,
    email: string,
}

impl Person {
    pub fn new(first: string, last: string, email: string) -> Person {
        Person {
            first_name: first,
            last_name: last,
            email: email,
        }
    }
}

pub fn create_person() -> Person {
    Person::new("John", "Doe", "john@example.com")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_chain_with_borrowed_params() {
    // Method that takes borrowed params and passes to another method
    let code = r#"
pub struct Logger {
    prefix: string,
}

impl Logger {
    pub fn new(prefix: string) -> Logger {
        Logger { prefix: prefix }
    }
    
    pub fn log(&self, message: string) {
        println!("[{}] {}", self.prefix, message)
    }
}

pub fn test_logger() {
    // Logger::new takes owned string, so "MyApp" should get .to_string()
    let logger = Logger::new("MyApp")
    logger.log("Application started")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}
