use anyhow::Result;
/// TDD Test: TryOp (?) Ownership Inference
///
/// PROBLEM: When a parameter is used inside a `?` (try/error propagation) expression,
/// the analyzer fails to detect the usage because `Expression::TryOp` wraps the inner
/// expression and the walking functions don't recurse into it.
///
/// Example:
/// ```
/// pub fn load_game_level(loader: AssetLoader, level_name: string) -> Result<...> {
///     let tilemap = loader.load("tilemap", "path", 8192)?
/// }
/// ```
///
/// The AST for `loader.load(...)?` is:
///   TryOp { expr: MethodCall { object: Identifier("loader"), method: "load", ... } }
///
/// The analyzer sees TryOp and returns false (catch-all), never checking the inner
/// MethodCall. This causes `loader` to be inferred as `&AssetLoader` (borrowed)
/// when it should stay Owned (since .load() is potentially mutating).
///
/// FIX: Add TryOp handling to all walking functions in the analyzer so that
/// expressions wrapped in `?` are still analyzed for ownership inference.
fn compile_wj_to_rust(source: &str) -> Result<String> {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_tryop_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    let src_dir = temp_dir.join("src_wj");
    fs::create_dir_all(&src_dir)?;

    fs::write(src_dir.join("main.wj"), source)?;

    fs::write(
        temp_dir.join("wj.toml"),
        "[package]\nname = \"tryop_test\"\nversion = \"0.1.0\"\n",
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
fn test_tryop_method_call_keeps_param_owned() -> Result<()> {
    // loader.load(...)? — .load() is potentially mutating, so loader should stay Owned
    let source = r#"
struct AssetLoader {
    pub base_path: String,
}

impl AssetLoader {
    fn new(path: String) -> AssetLoader {
        AssetLoader { base_path: path }
    }

    fn load(&mut self, name: String) -> Result<String, String> {
        Ok(name)
    }
}

fn load_game(loader: AssetLoader) -> Result<String, String> {
    let result = loader.load("tilemap".to_string())?
    Ok(result)
}
"#;

    let rust_code = compile_wj_to_rust(source)?;

    // The parameter should NOT be borrowed because .load() is potentially mutating
    // and it's inside a TryOp expression
    assert!(
        !rust_code.contains("loader: &AssetLoader"),
        "loader should NOT be &AssetLoader (borrowed) when .load()? is called.\n\
         The ? operator wraps the method call in TryOp, which must be recursed into.\n\
         Generated:\n{}",
        rust_code
    );

    // It should be either owned or mut borrowed
    let has_owned =
        rust_code.contains("loader: AssetLoader") || rust_code.contains("mut loader: AssetLoader");
    let has_mut_ref = rust_code.contains("loader: &mut AssetLoader");
    assert!(
        has_owned || has_mut_ref,
        "loader should be AssetLoader (owned) or &mut AssetLoader since .load() needs &mut self.\n\
         Generated:\n{}",
        rust_code
    );

    Ok(())
}

#[test]
fn test_tryop_passed_as_argument_keeps_param_owned() -> Result<()> {
    // process(data)? — data is passed as argument to a function that returns Result
    // Use a non-String, non-Copy custom type to properly test TryOp handling
    // Payload has a Vec<i32> field so it's definitely not Copy
    let source = r#"
struct Payload {
    pub items: Vec<i32>,
}

fn process(data: Payload) -> Result<i32, String> {
    Ok(data.items.len() as i32)
}

fn run(data: Payload) -> Result<i32, String> {
    let result = process(data)?
    Ok(result)
}
"#;

    let rust_code = compile_wj_to_rust(source)?;

    // data is passed as an argument to process() inside a TryOp,
    // so it should stay owned (consumed by the call)
    assert!(
        !rust_code.contains("fn run(data: &Payload)"),
        "data should NOT be &Payload when passed to process(data)?.\n\
         Generated:\n{}",
        rust_code
    );

    Ok(())
}
