//! TDD Test: Method call string arguments need .to_string()
//!
//! When calling a method like `.icon("📄")` where the method expects String,
//! the string literal should be converted to String automatically.

#[path = "test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_chain_string_args() {
    // Method chain with string literal arguments
    let code = r#"
pub struct MenuItem {
    label: string,
    icon: string,
}

impl MenuItem {
    pub fn new(label: string) -> MenuItem {
        MenuItem { label: label, icon: "".to_string() }
    }
    
    pub fn icon(self, icon: string) -> MenuItem {
        MenuItem { label: self.label, icon: icon }
    }
}

pub fn create_menu() -> MenuItem {
    MenuItem::new("File").icon("📁")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // The .icon("📁") should have .to_string() added
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_builder_pattern_string_args() {
    // Builder pattern with multiple string methods
    let code = r#"
pub struct Config {
    name: string,
    description: string,
    author: string,
}

impl Config {
    pub fn new() -> Config {
        Config { 
            name: "".to_string(), 
            description: "".to_string(),
            author: "".to_string(),
        }
    }
    
    pub fn name(self, name: string) -> Config {
        Config { name: name, description: self.description, author: self.author }
    }
    
    pub fn description(self, desc: string) -> Config {
        Config { name: self.name, description: desc, author: self.author }
    }
    
    pub fn author(self, author: string) -> Config {
        Config { name: self.name, description: self.description, author: author }
    }
}

pub fn create_config() -> Config {
    Config::new()
        .name("MyApp")
        .description("A great app")
        .author("Me")
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Builder pattern should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_push_string_literal() {
    // Vec::push with string literal
    let code = r#"
pub fn test_push() -> Vec<string> {
    let mut items = Vec::new()
    items.push("first")
    items.push("second")
    items
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "Vec push should work with string literals. Error: {}",
        err
    );
}
