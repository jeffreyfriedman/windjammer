// TDD Test: Standalone Function Parameters Should Be Declared `mut` When Used with &mut self Methods
// Bug: When a standalone function takes an owned parameter and calls a &mut self method on it,
//      the generated Rust doesn't declare the parameter as `mut`, causing E0596.
// Example: pub fn load_game_level(loader: AssetLoader, ...) calls loader.load(...)
//          where load() takes &mut self. Generated code needs `mut loader: AssetLoader`.
// Root Cause: statement_mutates_variable_field() didn't check Statement::Let bindings,
//             only Statement::Expression. When a mutating method call was the RHS of a let binding
//             (e.g., `let result = loader.load(...)`), the parameter wasn't marked as needing `mut`.
// Fix: Extended statement_mutates_variable_field() to recurse into Statement::Let values.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_check(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return (
            false,
            String::new(),
            format!(
                "Compiler failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        );
    }

    let generated_path = out_dir.join("test.rs");
    let generated =
        fs::read_to_string(&generated_path).unwrap_or_else(|e| format!("Read error: {}", e));

    let rustc = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc {
        Ok(rustc_output) => {
            let err = String::from_utf8_lossy(&rustc_output.stderr).to_string();
            (rustc_output.status.success(), generated, err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

#[test]
fn test_owned_param_with_mut_method_call_gets_mut_binding() {
    let (ok, generated, err) = compile_and_check(
        r#"
struct Item {
    name: String,
}

struct Loader {
    items: Vec<Item>,
}

impl Loader {
    fn load(self, name: String) -> Item {
        let item = Item { name: name.clone() }
        self.items.push(item.clone())
        item
    }
}

fn use_loader(loader: Loader) -> Item {
    loader.load("test".to_string())
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    // The key: loader must be declared as `mut` since load() takes &mut self
    assert!(
        ok,
        "Generated Rust should compile without E0596.\nErrors:\n{}",
        err
    );
}

#[test]
fn test_owned_param_with_mut_method_call_in_let_binding() {
    // THE ACTUAL BUG: When the mutating method call is in a let binding (not a bare expression),
    // statement_mutates_variable_field didn't detect it because it only checked Statement::Expression.
    // This is the exact pattern from windjammer-game's load_game_level:
    //   let tilemap = loader.load("tilemap", "levels/tilemap.json", 8192)?
    let (ok, generated, err) = compile_and_check(
        r#"
struct Asset {
    name: String,
    data: Vec<i64>,
}

struct AssetLoader {
    loaded: Vec<Asset>,
    count: i64,
}

impl AssetLoader {
    fn load(self, name: String, path: String) -> Asset {
        self.count = self.count + 1
        let asset = Asset { name: name.clone(), data: Vec::new() }
        self.loaded.push(asset.clone())
        asset
    }
}

fn load_level(loader: AssetLoader, level: String) -> Vec<Asset> {
    let mut assets: Vec<Asset> = Vec::new()
    let tilemap = loader.load("tilemap".to_string(), "levels/tilemap.json".to_string())
    let texture = loader.load("texture".to_string(), "levels/texture.png".to_string())
    assets.push(tilemap)
    assets.push(texture)
    assets
}
"#,
    );

    println!("Generated:\n{}", generated);
    if !ok {
        println!("Errors:\n{}", err);
    }

    // loader must be `mut` because load() takes &mut self AND the calls are in let bindings
    assert!(
        ok,
        "Generated Rust should compile without E0596 when mutating method call is in let binding.\nErrors:\n{}",
        err
    );
    // Verify mut is present in the generated signature
    assert!(
        generated.contains("mut loader"),
        "Parameter 'loader' should be declared as 'mut' in generated code.\nGenerated:\n{}",
        generated
    );
}
