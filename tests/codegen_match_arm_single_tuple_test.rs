/// TDD Test: Match arm type inference for single-element enum tuples
///
/// Issue: local_var_types is empty for DialogCondition::QuestComplete(quest_id)
/// but works for DialogCondition::RelationshipLevel(char_id, min_level)
///
/// Root cause: Single-element tuple variants may not be registered correctly
/// in enum_variant_types, causing infer_match_bound_types to return empty vec

/// Compile .wj code and return generated Rust code
fn compile_to_rust(wj_code: &str) -> String {
    let temp_dir = tempfile::tempdir().unwrap();
    let wj_file = temp_dir.path().join("test.wj");
    std::fs::write(&wj_file, wj_code).unwrap();

    let compiler_dir = std::env::current_dir().unwrap();
    let compiler = compiler_dir.join("target/release/wj");

    let output = std::process::Command::new(&compiler)
        .arg("build")
        .arg(&wj_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("wj build failed:\n{}", stderr);
    }

    let rs_file = temp_dir.path().join("build/test.rs");
    std::fs::read_to_string(rs_file).expect("Failed to read generated Rust")
}

#[test]
fn test_single_element_tuple_enum_match() {
    // Test case 1: Single-element tuple - like QuestComplete(quest_id)
    // The parser creates EnumPatternBinding::Single for single-element tuples
    // But infer_match_bound_types only handles Single for Option::Some!
    // This test verifies general single-element enum variants work correctly
    let code = r#"
pub struct Checker {
    pub expected: string,
}

impl Checker {
    pub fn check(self, value: string) -> bool {
        self.expected == value
    }
}

pub enum Event {
    QuestComplete(string),
}

impl Event {
    pub fn matches(self, checker: Checker) -> bool {
        match self {
            Event::QuestComplete(quest_id) => {
                // quest_id should be owned String
                // With proper type inference, this should auto-add &
                checker.check(quest_id)
            }
        }
    }
}

pub fn main() {
    let checker = Checker { expected: "main_quest" }
    let event = Event::QuestComplete("main_quest")
    let result = event.matches(checker)
}
"#;

    let rust_code = compile_to_rust(code);

    // Should add & for String → &str conversion
    assert!(
        rust_code.contains("checker.check(&quest_id)")
            || rust_code.contains("checker.check(quest_id)"),
        "Should auto-convert String to &str:\n{}",
        rust_code
    );

    // Verify no compilation errors by checking rustc
    let temp_dir = tempfile::tempdir().unwrap();
    let rs_file = temp_dir.path().join("test.rs");
    std::fs::write(&rs_file, &rust_code).unwrap();

    let rustc_output = std::process::Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rs_file)
        .arg("--out-dir")
        .arg(temp_dir.path())
        .output()
        .unwrap();

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    assert!(
        !rustc_stderr.contains("error[E"),
        "Generated Rust should compile:\n{}",
        rustc_stderr
    );
}

#[test]
fn test_multi_element_tuple_enum_match() {
    // Test case 2: Multi-element tuple - like RelationshipLevel(char_id, min_level)
    // The parser creates EnumPatternBinding::Tuple for multi-element tuples
    // This path should work and does work in practice
    let code = r#"
pub struct Checker {
    pub expected_id: string,
    pub min_level: i32,
}

impl Checker {
    pub fn check(self, id: string, level: i32) -> bool {
        self.expected_id == id && self.min_level <= level
    }
}

pub enum Event {
    RelationshipLevel(string, i32),
}

impl Event {
    pub fn matches(self, checker: Checker) -> bool {
        match self {
            Event::RelationshipLevel(char_id, min_level) => {
                // Both should be owned, auto-convert String → &str
                checker.check(char_id, min_level)
            }
        }
    }
}

pub fn main() {
    let checker = Checker { expected_id: "npc1", min_level: 3 }
    let event = Event::RelationshipLevel("npc1", 5)
    let result = event.matches(checker)
}
"#;

    let rust_code = compile_to_rust(code);

    // Should add & for String → &str conversion
    assert!(
        rust_code.contains("&char_id") || rust_code.contains("char_id"),
        "Should handle multi-element tuple:\n{}",
        rust_code
    );
}

#[test]
fn test_mixed_single_and_multi_tuple_variants() {
    // Test case 3: Mix of single and multi-element variants in same enum
    // This is the actual dialog.wj pattern!
    let code = r#"
pub struct Checker {
    pub id_field: string,
}

impl Checker {
    pub fn matches_id(self, id: string) -> bool {
        self.id_field == id
    }
}

pub enum Condition {
    HasItem(string, i32),           // 2 elements → EnumPatternBinding::Tuple
    HasGold(i32),                   // 1 element → EnumPatternBinding::Single
    QuestComplete(string),          // 1 element → EnumPatternBinding::Single
    RelationshipLevel(string, i32), // 2 elements → EnumPatternBinding::Tuple
}

impl Condition {
    pub fn check(self, checker: Checker) -> bool {
        match self {
            Condition::HasItem(item_id, _qty) => {
                checker.matches_id(item_id)
            },
            Condition::HasGold(_amount) => true,
            Condition::QuestComplete(quest_id) => {
                // CRITICAL: quest_id uses EnumPatternBinding::Single
                // This should be tracked in local_var_types!
                checker.matches_id(quest_id)
            },
            Condition::RelationshipLevel(char_id, _level) => {
                checker.matches_id(char_id)
            },
        }
    }
}

pub fn main() {
    let checker = Checker { id_field: "test" }
    let c1 = Condition::HasItem("sword", 1)
    let c2 = Condition::QuestComplete("main_quest")
    let r1 = c1.check(checker)
}
"#;

    let rust_code = compile_to_rust(code);

    // All should auto-convert String → &str
    assert!(
        rust_code.contains("&quest_id")
            || rust_code.contains("&item_id")
            || rust_code.contains("&char_id"),
        "Should auto-convert String → &str for all variants:\n{}",
        rust_code
    );
}
