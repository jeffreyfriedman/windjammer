#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-460: Copy **unit enum** formal into `Vec::push` must not `.clone()`.
//!
//! Product `input/input_interface.rs` `press_key`:
//!   `self.keys_down.push(key.clone()); self.keys_pressed.push(key.clone())`
//! WJ uses bare `key`. Same file's `press_mouse_button` correctly emits
//! `push(button)` for Copy unit enum `MouseButton`.
//! Distinct from WDB-457 (Copy **newtype** into push), WDB-459 (Copy **struct**
//! into push), WDB-458 (newtype into owned method formal).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Key {
    W,
    A,
    S,
    D,
    Space,
    Escape,
    Unknown,
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub struct Input {
    pub keys_down: Vec<Key>,
    pub keys_pressed: Vec<Key>,
    pub mouse_buttons_down: Vec<MouseButton>,
}

impl Input {
    pub fn new() -> Input {
        Input {
            keys_down: Vec::new(),
            keys_pressed: Vec::new(),
            mouse_buttons_down: Vec::new(),
        }
    }

    pub fn press_key(self, key: Key) {
        self.keys_down.push(key)
        self.keys_pressed.push(key)
    }

    pub fn press_mouse_button(self, button: MouseButton) {
        self.mouse_buttons_down.push(button)
    }
}
"#;

#[test]
fn wdb460_module_file_copy_enum_formal_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-460 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-460 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("key.clone()");
    assert!(
        !bad,
        "WDB-460 RED: Copy unit enum formal cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb460_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn wdb460_tip_out_game_core_input_copy_enum_vec_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb460_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/input/input_interface.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/input/input_interface.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("keys_down.push(key.clone())")
                    || t.contains("keys_pressed.push(key.clone())"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-460: input_interface product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-460 RED: tip/product Copy unit enum formal into Vec::push clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
