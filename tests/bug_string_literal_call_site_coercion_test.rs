#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;
use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

/// SaveData getters lower to `&str` keys; literals must not allocate `.to_string()`.
#[test]
fn test_save_data_get_int_literal_not_to_string_with_engine_metadata() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let build = tmp.path().join("build");
    let engine_meta = tmp.path().join("engine_metadata.json");
    fs::create_dir_all(&src).unwrap();

    fs::write(
        &engine_meta,
        r##"{
  "functions": {
    "SaveData::get_int": {
      "params": ["Custom(\"Self\")", "String"],
      "return_type": "Option(Custom(\"i32\"))",
      "is_associated": true,
      "parent_type": "SaveData",
      "param_ownership": ["Borrowed", "Owned"],
      "has_self_receiver": true,
      "is_extern": false
    },
    "SaveData::set_int": {
      "params": ["Custom(\"Self\")", "String", "Custom(\"i32\")"],
      "return_type": null,
      "is_associated": true,
      "parent_type": "SaveData",
      "param_ownership": ["MutBorrowed", "Owned", "Owned"],
      "has_self_receiver": true,
      "is_extern": false
    }
  },
  "structs": {
    "SaveData": {
      "int_fields": "Parameterized(\"HashMap\", [String, Custom(\"i32\")])"
    }
  }
}"##,
    )
    .unwrap();

    fs::write(
        src.join("save_data.wj"),
        r##"
pub struct SaveData {
    int_fields: Map<string, i32>,
}

impl SaveData {
    pub fn new() -> SaveData {
        SaveData { int_fields: Map::new() }
    }

    pub fn set_int(self, key: string, value: i32) {
        self.int_fields.insert(key, value)
    }

    pub fn get_int(self, key: string) -> Option<i32> {
        self.int_fields.get(key)
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("game_save.wj"),
        r##"
use crate::save_data::SaveData

pub fn restore(data: SaveData) -> i32 {
    if let Some(v) = data.get_int("health") {
        v
    } else {
        0
    }
}

pub fn capture(mut data: SaveData) {
    data.set_int("health", 100)
}
"##,
    )
    .unwrap();

    build_project_ext(
        &src,
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[("engine", &engine_meta)],
    )
    .expect("multipass build");

    let restore_rs = fs::read_to_string(build.join("game_save.rs")).expect("game_save.rs");
    assert!(
        restore_rs.contains("get_int(\"health\")"),
        "get_int key literal must stay bare for &str param. Got:\n{restore_rs}"
    );
    assert!(
        !restore_rs.contains("get_int(\"health\".to_string()"),
        "get_int must not allocate to_string on key literal. Got:\n{restore_rs}"
    );
}

/// Static factory methods that move string params into enums need owned literals (multipass).
#[test]
fn test_static_kill_factory_literal_to_string_for_owned_string_param() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let build = tmp.path().join("build");
    fs::create_dir_all(src.join("quest")).unwrap();

    fs::write(
        src.join("quest/objective.wj"),
        r##"
pub enum ObjectiveType {
    KillEnemies(string, u32),
}

pub struct Objective {
    pub kind: ObjectiveType,
}

impl Objective {
    pub fn kill(enemy_type: string, count: u32) -> Objective {
        Objective { kind: ObjectiveType::KillEnemies(enemy_type, count) }
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("content.wj"),
        r##"
use crate::quest::objective::Objective

pub fn build() -> Objective {
    Objective::kill("hostiles", 14)
}
"##,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build");

    let rs = fs::read_to_string(build.join("content.rs")).expect("content.rs");
    assert!(
        rs.contains("Objective::kill(\"hostiles\".to_string()")
            || rs.contains("Objective::kill(\"hostiles\".into()"),
        "owned string static arg must allocate from literal. Got:\n{rs}"
    );
}

/// Factory helpers that only pass string keys as `&str` must not allocate on arg 0.
#[test]
fn test_objective_reach_location_literal_stays_bare_for_str_param() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let build = tmp.path().join("build");
    fs::create_dir_all(src.join("quest")).unwrap();

    fs::write(
        src.join("quest/objective.wj"),
        r##"
pub enum ObjectiveType {
    GoToLocation(f32, f32, f32),
}

pub struct Objective {
    pub kind: ObjectiveType,
}

impl Objective {
    pub fn reach_location(id: string, description: string, x: f32, y: f32, z: f32, radius: f32) -> Objective {
        Objective { kind: ObjectiveType::GoToLocation(x, y, z) }
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("content.wj"),
        r##"
use crate::quest::objective::Objective

pub fn build() -> Objective {
    Objective::reach_location("cmd_deck", "Reach deck", 1.0, 2.0, 3.0, 4.0)
}
"##,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build");

    let rs = fs::read_to_string(build.join("content.rs")).expect("content.rs");
    assert!(
        rs.contains("Objective::reach_location(\"cmd_deck\""),
        "reach_location id literal must stay bare for &str param. Got:\n{rs}"
    );
    assert!(
        !rs.contains("\"cmd_deck\".to_string()"),
        "reach_location must not allocate to_string on &str param. Got:\n{rs}"
    );
}

/// Static `talk_to` moves into enum — literal must become owned without `&`.
#[test]
fn test_objective_talk_to_literal_to_string_without_ref() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let build = tmp.path().join("build");
    fs::create_dir_all(src.join("quest")).unwrap();

    fs::write(
        src.join("quest/objective.wj"),
        r##"
pub enum ObjectiveType {
    TalkTo(string),
}

pub struct Objective {
    pub kind: ObjectiveType,
}

impl Objective {
    pub fn talk_to(npc_name: string) -> Objective {
        Objective { kind: ObjectiveType::TalkTo(npc_name) }
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("content.wj"),
        r##"
use crate::quest::objective::Objective

pub fn build() -> Objective {
    Objective::talk_to("Lyra")
}
"##,
    )
    .unwrap();

    build_project_ext(&src, &build, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build");

    let rs = fs::read_to_string(build.join("content.rs")).expect("content.rs");
    assert!(
        rs.contains("Objective::talk_to(\"Lyra\".to_string()")
            || rs.contains("Objective::talk_to(\"Lyra\".into()"),
        "talk_to must coerce literal to owned String. Got:\n{rs}"
    );
    assert!(
        !rs.contains("talk_to(&\"Lyra\""),
        "talk_to must not borrow coerced literal. Got:\n{rs}"
    );
}

/// Copy enum payload bindings must stay owned in match arms (i32 > i32).
#[test]
fn test_match_arm_copy_i32_binding_not_ref_in_comparison() {
    #[path = "common/test_utils.rs"]
    mod test_utils;

    let source = r##"
pub enum DialogueCondition {
    HonorAbove(i32),
}

pub fn check(state_honor: i32, cond: DialogueCondition) -> bool {
    match cond {
        DialogueCondition::HonorAbove(threshold) => {
            state_honor > threshold
        },
    }
}
"##;

    let generated = test_utils::compile_single(source);
    assert!(
        !generated.contains("> &threshold"),
        "Copy i32 match binding must not be ref-wrapped. Got:\n{generated}"
    );
    assert!(
        generated.contains("> threshold"),
        "expected plain i32 comparison. Got:\n{generated}"
    );
}

/// Blackboard `set_*` keys lower to `&str` when the body borrows them (e.g. `find_index(key)`).
#[test]
fn test_blackboard_set_bool_literal_not_to_string() {
    #[path = "common/test_utils.rs"]
    mod test_utils;

    let source = r##"
pub struct Blackboard {
    keys: Vec<string>,
    values: Vec<bool>,
}

impl Blackboard {
    pub fn new() -> Blackboard {
        Blackboard { keys: Vec::new(), values: Vec::new() }
    }

    pub fn find_index(self, key: string) -> i32 {
        for i in 0..self.keys.len() {
            if self.keys[i] == key {
                return i as i32
            }
        }
        -1
    }

    pub fn set_bool(self, key: string, value: bool) {
        let idx = self.find_index(key)
        if idx >= 0 {
            self.values[idx as u32] = value
        }
    }
}

pub fn update(mut bb: Blackboard) {
    bb.set_bool("__cond_alive", true)
}
"##;

    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("set_bool(\"__cond_alive\""),
        "set_bool key literal must stay bare for &str param. Got:\n{generated}"
    );
    assert!(
        !generated.contains("\"__cond_alive\".to_string()"),
        "set_bool must not allocate to_string on literal. Got:\n{generated}"
    );
}

/// Static enum factory methods move string params; literals need owned coercion.
#[test]
fn test_quest_reward_relationship_literal_to_string() {
    #[path = "common/test_utils.rs"]
    mod test_utils;

    let source = r##"
pub enum QuestReward {
    Relationship(string, i32),
}

impl QuestReward {
    pub fn relationship(npc: string, delta: i32) -> QuestReward {
        QuestReward::Relationship(npc, delta)
    }
}

pub fn build() -> QuestReward {
    QuestReward::relationship("Lyra", 20)
}
"##;

    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("QuestReward::relationship(\"Lyra\".to_string()")
            || generated.contains("QuestReward::relationship(\"Lyra\".into()"),
        "owned string static arg must allocate from literal. Got:\n{generated}"
    );
}

/// Copy payload bindings from `match &self` must deref in comparisons and call args.
#[test]
fn test_match_ref_self_copy_i32_binding_derefed_in_comparison() {
    #[path = "common/test_utils.rs"]
    mod test_utils;

    let source = r##"
pub enum DialogueCondition {
    HasFlag(string),
    HonorAbove(i32),
}

pub struct DialogueState {
    honor: i32,
}

impl DialogueState {
    pub fn new() -> DialogueState {
        DialogueState { honor: 0 }
    }

    pub fn get_honor(self) -> i32 {
        self.honor
    }
}

impl DialogueCondition {
    pub fn is_met(self, state: DialogueState) -> bool {
        match self {
            DialogueCondition::HonorAbove(threshold) => {
                state.get_honor() > threshold
            },
            DialogueCondition::HasFlag(flag) => {
                flag == "x"
            },
        }
    }
}
"##;

    let generated = test_utils::compile_single(source);
    assert!(
        generated.contains("> *(threshold)") || generated.contains("> *threshold"),
        "Copy i32 binding from match &self must deref in comparison. Got:\n{generated}"
    );
    assert!(
        !generated.contains("> &threshold"),
        "must not compare against ref-wrapped binding. Got:\n{generated}"
    );
}
