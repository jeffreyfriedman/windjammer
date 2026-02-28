/// TDD Test: Method Receiver Mutability Inference
/// 
/// Problem: When parameter calls mutating methods, compiler should infer &mut
/// 
/// Example:
/// ```windjammer
/// struct Loader {
///     pub data: Vec<string>
/// }
/// 
/// impl Loader {
///     pub fn add(&mut self, item: string) {
///         self.data.push(item)
///     }
/// }
/// 
/// fn process(loader: Loader) {  // Should infer: &mut Loader
///     loader.add("test")         // Calls mutating method
/// }
/// ```

use std::fs;
use std::process::Command;

#[test]
fn test_method_mut_borrow_inference() {
    let source = r#"
struct Loader {
    pub data: Vec<string>
}

impl Loader {
    pub fn new() -> Loader {
        Loader { data: Vec::new() }
    }
    
    pub fn add(&mut self, item: string) {
        self.data.push(item)
    }
}

fn process(loader: Loader) {
    loader.add("test")
    loader.add("another")
}

fn main() {
    let mut ldr = Loader::new()
    process(ldr)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();
    
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();
    
    let out_dir = test_dir.join("out");
    
    let output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");
    
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated);
    
    // Compile with rustc
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");
    
    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    // Verify loader parameter inferred as &mut
    assert!(
        generated.contains("fn process(loader: &mut Loader)") 
        || generated.contains("fn process(_loader: &mut Loader)"),
        "process() should infer &mut Loader for parameter calling mutating methods"
    );
    
    // Verify method calls use the parameter directly (not &mut)
    assert!(
        generated.contains("loader.add("),
        "Method calls should use parameter directly"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_multiple_mut_method_calls() {
    // Test with struct that has multiple mutating methods
    let source = r#"
struct Config {
    pub values: Vec<string>
}

impl Config {
    pub fn new() -> Config {
        Config { values: Vec::new() }
    }
    
    pub fn set(&mut self, key: string) {
        self.values.push(key)
    }
    
    pub fn clear(&mut self) {
        self.values.clear()
    }
}

fn setup(config: Config) {
    config.set("width")
    config.set("height")
    config.clear()
}

fn main() {
    let mut cfg = Config::new()
    setup(cfg)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();
    
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();
    
    let out_dir = test_dir.join("out");
    
    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");
    
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated);
    
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");
    
    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    assert!(
        generated.contains("fn setup(config: &mut Config)") 
        || generated.contains("fn setup(_config: &mut Config)"),
        "setup() should infer &mut Config"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}
