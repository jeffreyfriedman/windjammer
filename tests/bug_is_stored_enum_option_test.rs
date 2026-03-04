/// TDD Test: is_stored should detect enum variant construction, Some() wrapping,
/// and parameters nested in tuples within collection methods
///
/// Bug: Parameters used to construct enum variants, wrap in Some(), or embedded
/// in tuples that are pushed/inserted are not detected as "stored" and are
/// incorrectly inferred as Borrowed instead of Owned.
///
/// Dogfooding evidence: 12+ E0308 errors in windjammer-game from this pattern
use std::fs;
use std::process::Command;

fn compile_wj(source: &str) -> (String, bool) {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
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
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

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

    let rustc_ok = rustc_output.status.success();
    if !rustc_ok {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        eprintln!("rustc stderr:\n{}", stderr);
    }

    fs::remove_dir_all(&test_dir).ok();
    (generated, rustc_ok)
}

#[test]
fn test_enum_variant_construction_keeps_owned() {
    let source = r#"
enum Objective {
    Kill(string, i32),
    TalkTo(string),
    Collect(string, i32),
}

fn make_kill(enemy_type: string, count: i32) -> Objective {
    Objective::Kill(enemy_type, count)
}

fn make_talk(npc_name: string) -> Objective {
    Objective::TalkTo(npc_name)
}

fn main() {
    let obj = make_kill("goblin".to_string(), 5)
    let obj2 = make_talk("elder".to_string())
}
"#;

    let (generated, rustc_ok) = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("fn make_kill(enemy_type: String")
            || generated.contains("fn make_kill(enemy_type: &String"),
        "make_kill should have enemy_type param. Generated:\n{}",
        generated
    );

    assert!(
        rustc_ok,
        "Should compile with rustc. Generated:\n{}",
        generated
    );
}

#[test]
fn test_some_wrapping_keeps_owned() {
    let source = r#"
struct Config {
    pub current: Option<string>,
}

impl Config {
    pub fn new() -> Config {
        Config { current: None }
    }
    pub fn set_current(self, id: string) {
        self.current = Some(id)
    }
}

fn main() {
    let mut cfg = Config::new()
    cfg.set_current("test".to_string())
}
"#;

    let (generated, rustc_ok) = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        rustc_ok,
        "Should compile with rustc. Generated:\n{}",
        generated
    );
}

#[test]
fn test_tuple_in_push_keeps_owned() {
    let source = r#"
fn track_relationship(relationships: Vec<(string, f32)>, npc: string, delta: f32) {
    relationships.push((npc, delta))
}

fn main() {
    let mut rels: Vec<(string, f32)> = Vec::new()
    track_relationship(rels, "guard".to_string(), 1.0)
}
"#;

    let (generated, rustc_ok) = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        rustc_ok,
        "Should compile with rustc. Generated:\n{}",
        generated
    );
}

#[test]
fn test_passthrough_to_owned_method() {
    let source = r#"
struct Logger {
    pub entries: Vec<string>,
}

impl Logger {
    pub fn new() -> Logger {
        Logger { entries: Vec::new() }
    }
    pub fn log(self, msg: string) {
        self.entries.push(msg)
    }
    pub fn info(self, message: string) {
        self.log(message)
    }
    pub fn warn(self, message: string) {
        self.log(message)
    }
}

fn main() {
    let mut logger = Logger::new()
    logger.info("hello".to_string())
    logger.warn("warning".to_string())
}
"#;

    let (generated, rustc_ok) = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        rustc_ok,
        "Should compile with rustc. Generated:\n{}",
        generated
    );
}
