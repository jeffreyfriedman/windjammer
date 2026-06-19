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

/// Multipass + engine metadata: stale `Owned QuestId` on `is_quest_active` must not
/// suppress `&` at call sites after the defining module converges to `&QuestId`.
#[test]
fn test_quest_id_borrowed_with_engine_metadata_multipass() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let build = tmp.path().join("build");
    let engine_meta = tmp.path().join("engine_metadata.json");
    fs::create_dir_all(src.join("quest")).unwrap();

    // Minimal engine stub: Borrowed self + Owned QuestId (real engine metadata shape).
    fs::write(
        &engine_meta,
        r##"{
  "functions": {
    "QuestManager::is_quest_active": {
      "params": ["Custom(\"Self\")", "Custom(\"QuestId\")"],
      "return_type": "Bool",
      "is_associated": true,
      "parent_type": "QuestManager",
      "param_ownership": ["Borrowed", "Owned"],
      "has_self_receiver": true,
      "is_extern": false
    }
  },
  "structs": {
    "QuestManager": {
      "quests": "Parameterized(\"HashMap\", [Custom(\"QuestId\"), Custom(\"Quest\")])"
    },
    "QuestId": {
      "id": "String"
    }
  }
}"##,
    )
    .unwrap();

    fs::write(
        src.join("quest/quest_id.wj"),
        r##"
pub struct QuestId {
    id: string,
}

impl QuestId {
    pub fn from_u32(n: u32) -> QuestId {
        QuestId { id: "x" }
    }
}
"##,
    )
    .unwrap();

    // Defining module AFTER caller alphabetically (like breach game_systems before manager).
    fs::write(
        src.join("quest/manager.wj"),
        r##"
use crate::quest::quest_id::QuestId

pub struct QuestManager {
    quests: Map<QuestId, Quest>,
}

pub struct Quest {
    active: bool,
}

impl QuestManager {
    pub fn new() -> QuestManager {
        QuestManager { quests: Map::new() }
    }

    pub fn is_quest_active(self, id: QuestId) -> bool {
        if let Some(q) = self.quests.get(id) {
            q.active
        } else {
            false
        }
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("game_systems.wj"),
        r##"
use crate::quest::manager::QuestManager
use crate::quest::quest_id::QuestId

pub struct Game {
    quest_manager: QuestManager,
}

impl Game {
    pub fn new() -> Game {
        Game { quest_manager: QuestManager::new() }
    }

    pub fn tick(self) {
        self.quest_manager.is_quest_active(QuestId::from_u32(42))
    }
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
    .expect("multipass build with engine metadata");

    let rs = fs::read_to_string(build.join("game_systems.rs")).expect("game_systems.rs");
    assert!(
        rs.contains("is_quest_active(&QuestId::from_u32(42")
            || rs.contains("is_quest_active( &QuestId::from_u32(42"),
        "Expected borrowed QuestId at call site with engine metadata. Generated:\n{rs}"
    );
}

/// Multipass + engine metadata: multi-arg methods with converged `&QuestId` must not lose
/// the borrow when trailing params are legitimately Owned copy scalars (`usize`, `u32`).
#[test]
fn test_quest_id_borrowed_on_update_objective_progress_with_engine_metadata() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    let build = tmp.path().join("build");
    let engine_meta = tmp.path().join("engine_metadata.json");
    fs::create_dir_all(src.join("quest")).unwrap();

    fs::write(
        &engine_meta,
        r##"{
  "functions": {
    "QuestManager::update_objective_progress": {
      "params": [
        "Custom(\"Self\")",
        "Custom(\"QuestId\")",
        "Custom(\"usize\")",
        "Custom(\"u32\")"
      ],
      "return_type": null,
      "is_associated": true,
      "parent_type": "QuestManager",
      "param_ownership": ["MutBorrowed", "Borrowed", "Owned", "Owned"],
      "has_self_receiver": true,
      "is_extern": false
    }
  },
  "structs": {
    "QuestManager": {
      "quests": "Parameterized(\"HashMap\", [Custom(\"QuestId\"), Custom(\"Quest\")])"
    },
    "QuestId": {
      "id": "String"
    }
  }
}"##,
    )
    .unwrap();

    fs::write(
        src.join("quest/quest_id.wj"),
        r##"
pub struct QuestId {
    id: string,
}

impl QuestId {
    pub fn from_u32(n: u32) -> QuestId {
        QuestId { id: "x" }
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("quest/manager.wj"),
        r##"
use crate::quest::quest_id::QuestId

pub struct QuestManager {
    quests: Map<QuestId, Quest>,
}

pub struct Quest {
    active: bool,
}

impl QuestManager {
    pub fn new() -> QuestManager {
        QuestManager { quests: Map::new() }
    }

    pub fn update_objective_progress(self, quest_id: QuestId, objective_index: usize, amount: u32) {
        if let Some(q) = self.quests.get_mut(quest_id) {
            let _ = objective_index
            let _ = amount
            q.active = true
        }
    }
}
"##,
    )
    .unwrap();

    fs::write(
        src.join("game_systems.wj"),
        r##"
use crate::quest::manager::QuestManager
use crate::quest::quest_id::QuestId

pub struct Game {
    quest_manager: QuestManager,
}

impl Game {
    pub fn new() -> Game {
        Game { quest_manager: QuestManager::new() }
    }

    pub fn tick(self) {
        self.quest_manager.update_objective_progress(QuestId::from_u32(42), 0, 1)
    }
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
    .expect("multipass build with engine metadata");

    let rs = fs::read_to_string(build.join("game_systems.rs")).expect("game_systems.rs");
    assert!(
        rs.contains("update_objective_progress(&QuestId::from_u32(42")
            || rs.contains("update_objective_progress( &QuestId::from_u32(42"),
        "Expected borrowed QuestId on update_objective_progress. Generated:\n{rs}"
    );
}
