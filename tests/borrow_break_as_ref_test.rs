use anyhow::Result;
/// TDD Test: Borrow Break Pattern - as_ref() vs as_deref()
///
/// PROBLEM: The codegen generates a "borrow break" pattern for match expressions
/// where the scrutinee borrows self and the arm body mutates self:
///   let __match_borrow_break = self.method().map(|v| v.to_owned());
///   match __match_borrow_break.as_deref() { ... }
///
/// The `.as_deref()` call requires the inner type to implement `Deref`, which
/// works for `String` (Deref<Target=str>) but fails for custom types like
/// `DialogueNode` (E0599: as_deref exists but trait bounds not satisfied).
///
/// FIX: Use `.as_ref()` instead of `.as_deref()`. This works for ALL types:
///   Option<String>.as_ref() → Option<&String>   (String auto-coerces to &str where needed)
///   Option<Custom>.as_ref() → Option<&Custom>   (works for any type)

fn compile_wj_to_rust(source: &str) -> Result<String> {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_borrow_break_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    fs::write(src_dir.join("main.wj"), source)?;

    fs::write(
        temp_dir.join("wj.toml"),
        "[package]\nname = \"borrow_break_test\"\nversion = \"0.1.0\"\n",
    )?;

    let wj_compiler = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output_dir = temp_dir.join("out");
    fs::create_dir_all(&output_dir)?;

    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(&src_dir)
        .arg("-o")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Compilation failed:\n{}", stderr);
    }

    let main_rs = output_dir.join("main.rs");
    let content = fs::read_to_string(&main_rs)?;

    let _ = fs::remove_dir_all(&temp_dir);

    Ok(content)
}

#[test]
fn test_borrow_break_uses_as_ref_not_as_deref() -> Result<()> {
    // This test models the exact pattern from windjammer-game-core/dialogue/manager.wj:
    // - self.tree.get_node(id) returns Option<&DialogueNode> (borrows from self.tree)
    // - The match arm body mutates self (self.current_index = ...)
    // - This triggers the borrow break pattern
    // - The generated code should use .as_ref() not .as_deref() because
    //   DialogueNode doesn't implement Deref
    // The key is that get_item returns Option<&Item> (a REFERENCE that borrows self)
    // and the match arm body mutates self. This creates the borrow conflict that
    // triggers the borrow break pattern.
    let source = r#"
use std::collections::HashMap

struct Item {
    pub name: String,
    pub value: i32,
}

impl Item {
    fn new(name: String, value: i32) -> Item {
        Item { name: name, value: value }
    }
}

struct Container {
    pub items: HashMap<String, Item>,
    pub last_accessed: Option<String>,
}

impl Container {
    fn new() -> Container {
        Container { items: HashMap::new(), last_accessed: None }
    }

    fn get_item(&self, key: &str) -> Option<&Item> {
        self.items.get(key)
    }

    fn access_item(&mut self, key: String) {
        match self.get_item(&key) {
            Some(item) => {
                self.last_accessed = Some(key)
            }
            _ => {}
        }
    }
}
"#;

    let rust_code = compile_wj_to_rust(source)?;

    // If borrow break is generated, it should use .as_ref() not .as_deref()
    if rust_code.contains("__match_borrow_break") {
        assert!(
            !rust_code.contains(".as_deref()"),
            "Borrow break pattern should use .as_ref() instead of .as_deref() \
             because custom types don't implement Deref.\nGenerated:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains(".as_ref()"),
            "Borrow break pattern should use .as_ref() for universal compatibility.\n\
             Generated:\n{}",
            rust_code
        );
    }

    // The generated code should compile correctly with rustc
    // (as_deref would fail for custom types that don't implement Deref)
    assert!(
        !rust_code.contains("as_deref"),
        "Generated code should not use as_deref() on custom types.\nGenerated:\n{}",
        rust_code
    );

    Ok(())
}
