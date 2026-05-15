//! E0432 regression: `use crate::parent::Type` must resolve when `Type` lives in `parent/submodule.wj`
//! (Rust has no implicit re-export from directory `mod.rs`).

use std::fs;
use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_crate_parent_type_import_expands_to_defining_submodule() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let out = temp.path().join("build");
    fs::create_dir_all(src.join("autotile")).unwrap();

    fs::write(
        src.join("autotile/tile_id.wj"),
        r#"
pub struct TileId {
    pub value: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("autotile/wang_tile.wj"),
        r#"
use crate::autotile::TileId

pub struct WangTile {
    pub id: TileId,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("autotile/mod.wj"),
        r#"
pub mod tile_id
pub mod wang_tile
"#,
    )
    .unwrap();

    build_project_ext(&src, &out, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build should succeed");

    let generated = fs::read_to_string(out.join("autotile/wang_tile.rs")).unwrap();
    assert!(
        generated.contains("crate::autotile::tile_id::TileId"),
        "expected codegen to emit crate::autotile::tile_id::TileId; got:\n{}",
        generated
    );
}

#[test]
fn test_duplicate_type_name_resolves_by_import_parent_prefix() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let out = temp.path().join("build");
    fs::create_dir_all(src.join("autotile")).unwrap();
    fs::create_dir_all(src.join("tilemap")).unwrap();

    fs::write(
        src.join("autotile/tile_id.wj"),
        r#"
pub struct TileId {
    pub v: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("tilemap/tile.wj"),
        r#"
pub struct TileId {
    pub u: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("autotile/consumer.wj"),
        r#"
use crate::autotile::TileId

pub fn f() -> TileId {
    TileId { v: 1 }
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("autotile/mod.wj"),
        r#"
pub mod tile_id
pub mod consumer
"#,
    )
    .unwrap();

    fs::write(
        src.join("tilemap/mod.wj"),
        r#"
pub mod tile
"#,
    )
    .unwrap();

    build_project_ext(&src, &out, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build should succeed");

    let generated = fs::read_to_string(out.join("autotile/consumer.rs")).unwrap();
    assert!(
        generated.contains("crate::autotile::tile_id::TileId"),
        "autotile consumer should resolve TileId to autotile::tile_id, not tilemap; got:\n{}",
        generated
    );
}

#[test]
fn test_braced_crate_import_splits_to_submodule_paths() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let out = temp.path().join("build");
    fs::create_dir_all(src.join("demo")).unwrap();

    fs::write(
        src.join("demo/a_ty.wj"),
        r#"
pub struct ATy {
    pub x: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("demo/b_ty.wj"),
        r#"
pub struct BTy {
    pub y: i32,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("demo/user.wj"),
        r#"
use crate::demo::{ATy, BTy}

pub struct Holder {
    pub a: ATy,
    pub b: BTy,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("demo/mod.wj"),
        r#"
pub mod a_ty
pub mod b_ty
pub mod user
"#,
    )
    .unwrap();

    build_project_ext(&src, &out, CompilationTarget::Rust, false, true, &[])
        .expect("multipass build should succeed");

    let generated = fs::read_to_string(out.join("demo/user.rs")).unwrap();
    assert!(
        generated.contains("crate::demo::a_ty::ATy")
            && generated.contains("crate::demo::b_ty::BTy"),
        "expected separate expanded uses for braced import; got:\n{}",
        generated
    );
}
