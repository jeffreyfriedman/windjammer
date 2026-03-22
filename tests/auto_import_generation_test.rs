// TDD: Automatic import generation for cross-module type references (E0425).
//
// Phase 1–3: type collection + classification live in `analyzer::type_collector`;
// `CodeGenerator::format_auto_super_type_imports` emits `use super::Type` for module builds.

use std::fs;
use tempfile::tempdir;
use windjammer::build_project_ext;
use windjammer::CompilationTarget;

#[test]
fn test_library_build_generates_use_super_for_sibling_struct_type() {
    let src = tempdir().expect("tempdir");
    fs::write(
        src.path().join("user.wj"),
        "pub struct User {\n    pub name: String\n}\n",
    )
    .expect("write user.wj");
    fs::write(
        src.path().join("manager.wj"),
        "pub struct UserManager {\n    pub users: Vec<User>\n}\n",
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("manager.rs")).expect("read manager.rs");
    assert!(
        manager_rs.contains("use super::*"),
        "expected injected `use super::*` in generated manager.rs:\n{}",
        manager_rs
    );
    assert!(
        !manager_rs.contains("use super::User"),
        "must not emit redundant `use super::User` when `use super::*` is injected:\n{}",
        manager_rs
    );
}

#[test]
fn test_library_build_generates_multiple_super_uses_for_hashmap_fields() {
    let src = tempdir().expect("tempdir");
    fs::write(
        src.path().join("achievement_id.wj"),
        "pub struct AchievementId {\n    pub raw: i32\n}\n",
    )
    .expect("write achievement_id.wj");
    fs::write(
        src.path().join("achievement.wj"),
        "pub struct Achievement {\n    pub title: String\n}\n",
    )
    .expect("write achievement.wj");
    fs::write(
        src.path().join("manager.wj"),
        "pub struct AchievementManager {\n    pub achievements: HashMap<AchievementId, Achievement>\n}\n",
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("manager.rs")).expect("read manager.rs");
    assert!(
        manager_rs.contains("use super::*"),
        "expected `use super::*` for sibling types:\n{}",
        manager_rs
    );
    assert!(
        !manager_rs.contains("use super::AchievementId"),
        "must not emit broken flat `use super::AchievementId` when glob covers mod.rs re-exports:\n{}",
        manager_rs
    );
}

#[test]
fn test_library_build_generates_use_super_for_impl_param_type() {
    let src = tempdir().expect("tempdir");
    fs::write(
        src.path().join("item.wj"),
        "pub struct Item {\n    pub id: i32\n}\n",
    )
    .expect("write item.wj");
    fs::write(
        src.path().join("manager.wj"),
        r#"
pub struct Manager {
    items: Vec<Item>
}

impl Manager {
    pub fn add_item(self, item: Item) { }
}
"#,
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("manager.rs")).expect("read manager.rs");
    assert!(
        manager_rs.contains("use super::*"),
        "expected `use super::*` for sibling Item type:\n{}",
        manager_rs
    );
}

#[test]
fn test_nested_module_directory_preserves_build_and_emits_imports() {
    let src = tempdir().expect("tempdir");
    let achievement = src.path().join("achievement");
    fs::create_dir_all(&achievement).expect("mkdir");
    fs::write(
        achievement.join("achievement_id.wj"),
        "pub struct AchievementId {\n    pub v: i32\n}\n",
    )
    .expect("write achievement_id.wj");
    fs::write(
        achievement.join("manager.wj"),
        "pub struct M {\n    pub id: AchievementId\n}\n",
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("achievement/manager.rs"))
        .expect("read achievement/manager.rs");
    assert!(
        manager_rs.contains("use super::*"),
        "expected `use super::*` in nested manager.rs:\n{}",
        manager_rs
    );
    assert!(
        !manager_rs.contains("use super::AchievementId"),
        "nested sibling types must not use flat `use super::AchievementId` (E0432):\n{}",
        manager_rs
    );
}

/// When the user writes `use super::*`, sibling types are already in scope; auto-imports would
/// duplicate them (Rust E0252).
#[test]
fn test_skip_auto_super_import_when_super_glob_present() {
    let src = tempdir().expect("tempdir");
    fs::write(
        src.path().join("user.wj"),
        "pub struct User {\n    pub name: String\n}\n",
    )
    .expect("write user.wj");
    fs::write(
        src.path().join("manager.wj"),
        "use super::*\n\npub struct UserManager {\n    pub user: User\n}\n",
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("manager.rs")).expect("read manager.rs");
    assert!(
        manager_rs.contains("use super::*"),
        "expected parent glob in manager.rs:\n{}",
        manager_rs
    );
    assert!(
        !manager_rs.contains("use super::User"),
        "must not emit `use super::User` when `use super::*` is present (E0252):\n{}",
        manager_rs
    );
}

/// Without a user-written `use super::*`, the compiler injects `use super::*` for library modules.
#[test]
fn test_generates_auto_super_import_without_super_glob() {
    let src = tempdir().expect("tempdir");
    fs::write(
        src.path().join("user.wj"),
        "pub struct User {\n    pub name: String\n}\n",
    )
    .expect("write user.wj");
    fs::write(
        src.path().join("manager.wj"),
        "pub struct UserManager {\n    pub user: User\n}\n",
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("manager.rs")).expect("read manager.rs");
    assert!(
        manager_rs.contains("use super::*"),
        "expected injected `use super::*`:\n{}",
        manager_rs
    );
}

/// Explicit `use crate::...::Type` covers that name; sibling `User` comes from injected `use super::*`.
#[test]
fn test_mixed_explicit_crate_import_and_sibling_auto_import() {
    let src = tempdir().expect("tempdir");
    fs::write(
        src.path().join("math.wj"),
        "pub struct Vec3 {\n    pub x: f32\n}\n",
    )
    .expect("write math.wj");
    fs::write(
        src.path().join("user.wj"),
        "pub struct User {\n    pub name: String\n}\n",
    )
    .expect("write user.wj");
    fs::write(
        src.path().join("manager.wj"),
        "use crate::math::Vec3\n\npub struct Manager {\n    pub position: Vec3,\n    pub user: User\n}\n",
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("manager.rs")).expect("read manager.rs");
    assert!(
        manager_rs.contains("use super::*"),
        "expected `use super::*` for sibling User:\n{}",
        manager_rs
    );
    assert!(
        !manager_rs.contains("use super::Vec3"),
        "must not emit `use super::Vec3` when `use crate::...::Vec3` exists:\n{}",
        manager_rs
    );
}

/// When a glob import is present (`::*`), we skip injecting `use super::*` and emit resolved paths.
#[test]
fn test_crate_glob_suppresses_super_star_uses_resolved_sibling_path() {
    let src = tempdir().expect("tempdir");
    fs::write(
        src.path().join("user.wj"),
        "pub struct User {\n    pub name: String\n}\n",
    )
    .expect("write user.wj");
    fs::write(
        src.path().join("manager.wj"),
        "use std::fmt::*\n\npub struct UserManager {\n    pub user: User\n}\n",
    )
    .expect("write manager.wj");

    let out = tempdir().expect("out");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");

    let manager_rs = fs::read_to_string(out.path().join("manager.rs")).expect("read manager.rs");
    assert!(
        !manager_rs.contains("use super::*"),
        "must not inject `use super::*` when another glob is present:\n{}",
        manager_rs
    );
    assert!(
        manager_rs.contains("use super::user::User"),
        "expected resolved sibling import `use super::user::User`:\n{}",
        manager_rs
    );
}
