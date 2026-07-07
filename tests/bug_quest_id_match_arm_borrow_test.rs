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

/// Match arm binds owned `QuestId`; callee lowers to `&QuestId` (read-only field access).
/// Call site must pass `&quest_id`, not owned `quest_id` or `quest_id.clone()`.
#[test]
fn test_match_arm_quest_id_borrowed_at_method_call() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("test.wj");
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).expect("mkdir");

    fs::write(
        &wj,
        r##"
pub struct QuestId {
    pub id: string,
}

pub struct DialogueState {
    pub completed_quests: Vec<QuestId>,
}

impl DialogueState {
    pub fn is_quest_completed(self, quest_id: QuestId) -> bool {
        for i in 0..self.completed_quests.len() {
            if self.completed_quests[i].id == quest_id.id {
                return true
            }
        }
        false
    }
}

enum DialogueCondition {
    QuestCompleted(QuestId),
}

impl DialogueCondition {
    pub fn is_met(self, state: DialogueState) -> bool {
        match self {
            DialogueCondition::QuestCompleted(quest_id) => {
                state.is_quest_completed(quest_id)
            },
        }
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
        generated.contains("fn is_quest_completed(") && generated.contains("quest_id: &QuestId"),
        "read-only QuestId param should lower to &QuestId. Generated:\n{generated}"
    );
    assert!(
        generated.contains("is_quest_completed(&quest_id)")
            || generated.contains("is_quest_completed( &quest_id)"),
        "match-bound QuestId must be borrowed at call site. Generated:\n{generated}"
    );
    assert!(
        !generated.contains("is_quest_completed(quest_id.clone()"),
        "must not clone QuestId for borrowed param. Generated:\n{generated}"
    );
}
