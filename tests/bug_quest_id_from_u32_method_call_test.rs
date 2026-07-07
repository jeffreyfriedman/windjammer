#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// `QuestId::from_u32(...)` at a method call must become `&QuestId::from_u32(...)`
/// when the callee lowers the param to `&QuestId`.
#[test]
fn test_quest_id_from_u32_borrowed_at_method_call() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("test.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    fs::write(
        &wj,
        r##"
pub struct QuestId {
    id: string,
}

pub struct QuestManager {
    active: bool,
}

impl QuestManager {
    pub fn new() -> QuestManager {
        QuestManager { active: false }
    }

    pub fn is_quest_active(self, id: QuestId) -> bool {
        id.id == "1"
    }
}

pub struct Game {
    quests: QuestManager,
}

impl Game {
    pub fn new() -> Game {
        Game { quests: QuestManager::new() }
    }

    pub fn tick(self) {
        self.quests.is_quest_active(QuestId::from_u32(42))
    }
}

impl QuestId {
    pub fn from_u32(n: u32) -> QuestId {
        QuestId { id: "x" }
    }
}
"##,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    assert!(
        build.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("test.rs")).expect("test.rs");

    assert!(
        generated.contains("is_quest_active(&QuestId::from_u32(42")
            || generated.contains("is_quest_active( &QuestId::from_u32(42"),
        "QuestId constructor arg must be borrowed. Generated:\n{generated}"
    );
}
