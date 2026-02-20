use anyhow::Result;
/// TDD Test: Glob Import Ambiguity Prevention (E0659)
///
/// PROBLEM: When a file has explicit glob imports (`use crate::gizmos::*`) AND the codegen
/// adds `use super::*`, both can bring in the same name (e.g., `GizmoMode`), causing
/// Rust error E0659: "`GizmoMode` is ambiguous".
///
/// The parent module's mod.rs auto-generates `pub use sibling::GizmoMode` re-exports,
/// which means `use super::*` brings in `GizmoMode` from the parent. If the file also
/// has `use crate::other_module::*` which exports its own `GizmoMode`, Rust can't
/// resolve the ambiguity between two glob imports.
///
/// FIX: When a file has explicit glob imports (`::*`), suppress the auto-generated
/// `use super::*` to prevent E0659 ambiguity errors.
///
/// Example:
/// ```
/// // src_wj/panels/transform_gizmo_ui.wj
/// use crate::gizmos::*    // Explicit glob import
/// use crate::scene::*     // Another explicit glob import
/// ```
///
/// Should generate:
/// ```rust
/// // NO `use super::*;` — because explicit globs could conflict
/// use crate::gizmos::*;
/// use crate::scene::*;
/// ```
///
/// Should NOT generate:
/// ```rust
/// use super::*;           // ❌ Could conflict with explicit globs!
/// use crate::gizmos::*;
/// use crate::scene::*;
/// ```
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_super_glob_when_explicit_glob_imports_present() -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_glob_ambiguity_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    // Create two modules that both export a type with the same name
    // Module A: gizmos/
    let gizmos_dir = temp_dir.join("src_wj").join("gizmos");
    fs::create_dir_all(&gizmos_dir)?;

    fs::write(
        gizmos_dir.join("transform_gizmo.wj"),
        r#"
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

pub struct TransformGizmo {
    pub mode: GizmoMode,
}

impl TransformGizmo {
    pub fn new() -> TransformGizmo {
        TransformGizmo { mode: GizmoMode::Translate }
    }
}
"#,
    )?;

    fs::write(
        gizmos_dir.join("mod.wj"),
        r#"
pub mod transform_gizmo

pub use transform_gizmo::GizmoMode
pub use transform_gizmo::TransformGizmo
"#,
    )?;

    // Module B: panels/
    let panels_dir = temp_dir.join("src_wj").join("panels");
    fs::create_dir_all(&panels_dir)?;

    // scene_view.wj defines its own GizmoMode (or re-uses one)
    fs::write(
        panels_dir.join("scene_view.wj"),
        r#"
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

pub struct SceneViewPanel {
    pub mode: GizmoMode,
}
"#,
    )?;

    // transform_gizmo_ui.wj uses explicit glob import from gizmos
    // This is the file that gets the ambiguity error
    fs::write(
        panels_dir.join("transform_gizmo_ui.wj"),
        r#"
use crate::gizmos::*

pub struct GizmoPanel {
    pub gizmo: TransformGizmo,
    pub enabled: bool,
}

impl GizmoPanel {
    pub fn new() -> GizmoPanel {
        GizmoPanel {
            gizmo: TransformGizmo::new(),
            enabled: true,
        }
    }

    pub fn set_mode(&mut self, mode: GizmoMode) {
        self.gizmo.mode = mode
    }
}
"#,
    )?;

    // mod.wj re-exports GizmoMode from scene_view (creating the conflict)
    fs::write(
        panels_dir.join("mod.wj"),
        r#"
pub mod scene_view
pub mod transform_gizmo_ui

pub use scene_view::GizmoMode
pub use scene_view::SceneViewPanel
"#,
    )?;

    // Create wj.toml
    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "glob_ambiguity_test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    // Build the library
    let wj_compiler = get_wj_compiler();
    let lib_output = temp_dir.join("lib");
    fs::create_dir_all(&lib_output)?;

    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(temp_dir.join("src_wj"))
        .arg("-o")
        .arg(&lib_output)
        .arg("--library")
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check the generated transform_gizmo_ui.rs file
    let gizmo_ui_rs_path = lib_output.join("panels").join("transform_gizmo_ui.rs");
    assert!(
        gizmo_ui_rs_path.exists(),
        "transform_gizmo_ui.rs should be generated at {:?}\nstdout: {}\nstderr: {}",
        gizmo_ui_rs_path,
        stdout,
        stderr
    );

    let gizmo_ui_content = fs::read_to_string(&gizmo_ui_rs_path)?;

    // The file has `use crate::gizmos::*;` (explicit glob import)
    assert!(
        gizmo_ui_content.contains("use crate::gizmos::*;"),
        "Generated file should contain explicit glob import 'use crate::gizmos::*;'.\nGenerated:\n{}",
        gizmo_ui_content
    );

    // The file should NOT have `use super::*;` because it would conflict
    // with the explicit glob import (both bring GizmoMode into scope)
    let has_super_glob = gizmo_ui_content.contains("use super::*;");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        !has_super_glob,
        "When a file has explicit glob imports, `use super::*` should be suppressed to prevent \
         E0659 ambiguity errors.\nGenerated transform_gizmo_ui.rs:\n{}",
        gizmo_ui_content
    );

    Ok(())
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_super_glob_still_added_without_explicit_glob_imports() -> Result<()> {
    // Files WITHOUT explicit glob imports should still get `use super::*`
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_super_glob_kept_test_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    // Module with sibling files that need `use super::*`
    let quest_dir = temp_dir.join("src_wj").join("quest");
    fs::create_dir_all(&quest_dir)?;

    fs::write(
        quest_dir.join("types.wj"),
        r#"
pub struct Quest {
    pub name: String,
    pub completed: bool,
}

impl Quest {
    pub fn new(name: String) -> Quest {
        Quest { name: name, completed: false }
    }
}
"#,
    )?;

    // manager.wj uses Quest via `use super::*` (no explicit glob imports)
    fs::write(
        quest_dir.join("manager.wj"),
        r#"
pub struct QuestManager {
    pub quests: Vec<Quest>,
}

impl QuestManager {
    pub fn new() -> QuestManager {
        QuestManager { quests: Vec::new() }
    }
}
"#,
    )?;

    fs::write(
        quest_dir.join("mod.wj"),
        r#"
pub mod types
pub mod manager

pub use types::Quest
"#,
    )?;

    fs::write(
        temp_dir.join("wj.toml"),
        r#"
[package]
name = "super_glob_kept_test"
version = "0.1.0"

[dependencies]
"#,
    )?;

    let wj_compiler = get_wj_compiler();
    let lib_output = temp_dir.join("lib");
    fs::create_dir_all(&lib_output)?;

    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(temp_dir.join("src_wj"))
        .arg("-o")
        .arg(&lib_output)
        .arg("--library")
        .output()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let manager_rs_path = lib_output.join("quest").join("manager.rs");
    assert!(
        manager_rs_path.exists(),
        "manager.rs should be generated at {:?}\nstdout: {}\nstderr: {}",
        manager_rs_path,
        stdout,
        stderr
    );

    let manager_content = fs::read_to_string(&manager_rs_path)?;

    // File should still have `use super::*` since there are no explicit glob imports
    let has_super_glob = manager_content.contains("use super::*;");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(
        has_super_glob,
        "Files WITHOUT explicit glob imports should still get `use super::*` for sibling type access.\nGenerated manager.rs:\n{}",
        manager_content
    );

    Ok(())
}
