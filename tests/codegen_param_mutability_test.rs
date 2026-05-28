#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

// TDD Test: Standalone Function Parameters Should Be Declared `mut` When Used with &mut self Methods
// Bug: When a standalone function takes an owned parameter and calls a &mut self method on it,
//      the generated Rust doesn't declare the parameter as `mut`, causing E0596.
// Example: pub fn load_game_level(loader: AssetLoader, ...) calls loader.load(...)
//          where load() takes &mut self. Generated code needs `mut loader: AssetLoader`.
// Root Cause: statement_mutates_variable_field() didn't check Statement::Let bindings,
//             only Statement::Expression. When a mutating method call was the RHS of a let binding
//             (e.g., `let result = loader.load(...)`), the parameter wasn't marked as needing `mut`.
// Fix: Extended statement_mutates_variable_field() to recurse into Statement::Let values.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_owned_param_with_mut_method_call_gets_mut_binding() {
    let source = r#"
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
"#;
    let (generated, ok) = test_utils::compile_single_check(source);

    assert!(
        ok,
        "Generated Rust should compile without E0596.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_owned_param_with_mut_method_call_in_let_binding() {
    let source = r#"
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
"#;
    let (generated, ok) = test_utils::compile_single_check(source);

    assert!(
        ok,
        "Generated Rust should compile correctly with automatic &mut inference.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("loader: &mut AssetLoader"),
        "Parameter 'loader' should be inferred as `&mut AssetLoader`.\nGenerated:\n{}",
        generated
    );
}
