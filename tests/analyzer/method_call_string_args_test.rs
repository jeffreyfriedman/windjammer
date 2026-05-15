//! TDD Test: Method call string argument handling
//!
//! This test verifies that when a method takes a string parameter that is only read,
//! and we call it with a string literal, the generated code compiles correctly.
//!
//! Issue: Method infers &String but caller adds .to_string() creating String

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_with_read_only_string_param() {
    // A method that only reads its string parameter should work when called with literals
    let code = r#"
pub struct Editor {
    items: Vec<string>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor { items: Vec::new() }
    }
    
    // This method only reads 'name' - should infer correctly
    pub fn add_item(&mut self, name: string) -> i32 {
        println!("Adding: {}", name)
        self.items.len() as i32
    }
}

pub fn create_editor() {
    let mut editor = Editor::new()
    // These calls should work without type errors
    let _ = editor.add_item("First")
    let _ = editor.add_item("Second")
    let _ = editor.add_item("Third")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    // Print debug info
    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_with_stored_string_param() {
    // A method that stores its string parameter should take owned String
    let code = r#"
pub struct NameList {
    names: Vec<string>,
}

impl NameList {
    pub fn new() -> NameList {
        NameList { names: Vec::new() }
    }
    
    // This method stores 'name' - should be owned
    pub fn add(&mut self, name: string) {
        self.names.push(name)
    }
}

pub fn test_name_list() {
    let mut list = NameList::new()
    list.add("Alice")
    list.add("Bob")
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
fn test_method_returning_computed_value() {
    // A method that uses string for computation should handle correctly
    let code = r#"
pub struct Counter {
    count: i32,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { count: 0 }
    }
    
    // Uses name for logging, returns computed value
    pub fn increment(&mut self, label: string) -> i32 {
        self.count = self.count + 1
        println!("{}: {}", label, self.count)
        self.count
    }
}

pub fn test_counter() {
    let mut counter = Counter::new()
    let a = counter.increment("Step A")
    let b = counter.increment("Step B")
    println!("Total: {}", a + b)
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
fn test_chained_method_calls_with_strings() {
    // Chained method calls with string parameters
    let code = r#"
pub struct Builder {
    parts: Vec<string>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder { parts: Vec::new() }
    }
    
    pub fn add(&mut self, part: string) -> &mut Builder {
        self.parts.push(part)
        self
    }
    
    pub fn build(&self) -> string {
        self.parts.join(", ")
    }
}

pub fn test_builder() -> string {
    let mut builder = Builder::new()
    builder.add("one").add("two").add("three")
    builder.build()
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
