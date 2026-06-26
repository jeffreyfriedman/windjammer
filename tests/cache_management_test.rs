//! Cache management and compiler stamp invalidation tests.

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use windjammer::compiler::cache_management;

#[test]
fn test_is_output_fresh_when_rs_newer_than_wj() {
    let dir = TempDir::new().unwrap();
    let wj = dir.path().join("foo.wj");
    let rs = dir.path().join("foo.rs");
    fs::write(&wj, "fn main() {}\n").unwrap();
    thread::sleep(Duration::from_millis(10));
    fs::write(&rs, "// generated\n").unwrap();
    assert!(cache_management::is_output_fresh(&wj, &rs));
}

#[test]
fn test_is_output_fresh_when_wj_newer_than_rs() {
    let dir = TempDir::new().unwrap();
    let wj = dir.path().join("foo.wj");
    let rs = dir.path().join("foo.rs");
    fs::write(&rs, "// generated\n").unwrap();
    thread::sleep(Duration::from_millis(10));
    fs::write(&wj, "fn main() {}\n").unwrap();
    assert!(!cache_management::is_output_fresh(&wj, &rs));
}

#[test]
fn test_compiler_stamp_invalidation() {
    let dir = TempDir::new().unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();
    assert!(cache_management::is_compiler_stamp_fresh(dir.path()));
}

#[test]
fn test_compute_dirty_files_marks_all_when_no_output() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let wj = src.join("a.wj");
    fs::write(&wj, "fn a() {}\n").unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let sources = vec![(wj, "fn a() {}\n".to_string())];
    let (dirty, skipped) =
        cache_management::compute_dirty_files(&sources, &src, dir.path(), &[]);
    assert_eq!(skipped, 0);
    assert_eq!(dirty.len(), 1);
}

#[test]
fn test_all_sources_fresh_false_without_outputs() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let wj = src.join("a.wj");
    fs::write(&wj, "fn a() {}\n").unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let sources = vec![(wj, "fn a() {}\n".to_string())];
    assert!(!cache_management::all_sources_fresh(
        &sources,
        &src,
        dir.path(),
        &[] as &[PathBuf],
    ));
}

/// Mtime-only freshness is insufficient: if `.rs` is newer than `.wj` but
/// `.wj.meta` fingerprint does not match current source, file must be dirty.
#[test]
fn test_compute_dirty_when_rs_mtime_newer_but_source_content_changed() {
    use windjammer::compiler::incremental::fingerprint_for_emit;
    use windjammer::metadata::ModuleMetadata;

    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let wj = src.join("foo.wj");
    let rs = dir.path().join("foo.rs");
    let original = "fn original() {}\n";
    let updated = "fn updated() {}\n";
    fs::write(&wj, original).unwrap();

    // Emit meta fingerprint for ORIGINAL source
    let fp = fingerprint_for_emit(original, &[]);
    let meta = ModuleMetadata {
        module_path: "foo".to_string(),
        analysis_fingerprint: Some(fp.into()),
        ..ModuleMetadata::new("foo".to_string())
    };
    let meta_path = windjammer::metadata::meta_cache_path(&wj);
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&meta_path, serde_json::to_string(&meta).unwrap()).unwrap();

    let stale_rs = "// stale generated from original\n";
    fs::write(&rs, stale_rs).unwrap();

    // Edit .wj source (semantic change)
    thread::sleep(Duration::from_millis(10));
    fs::write(&wj, updated).unwrap();

    // Simulate stale output with newer mtime (touch without re-transpiling)
    thread::sleep(Duration::from_millis(10));
    fs::write(&rs, stale_rs).unwrap();

    // Mtime-only check would incorrectly consider this fresh
    assert!(cache_management::is_output_fresh(&wj, &rs));
    assert!(!cache_management::is_codegen_cache_valid(
        updated,
        &wj,
        &rs,
        &[] as &[PathBuf],
    ));

    let sources = vec![(wj, updated.to_string())];
    let (dirty, skipped) =
        cache_management::compute_dirty_files(&sources, &src, dir.path(), &[]);
    assert_eq!(skipped, 0, "must not silently skip when content changed");
    assert_eq!(dirty.len(), 1);
}

#[test]
fn test_find_stale_codegen_outputs_detects_fingerprint_mismatch() {
    use windjammer::compiler::incremental::fingerprint_for_emit;
    use windjammer::metadata::ModuleMetadata;

    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let wj = src.join("foo.wj");
    let rs = dir.path().join("foo.rs");
    let original = "fn original() {}\n";
    let updated = "fn updated() {}\n";
    fs::write(&wj, original).unwrap();

    let fp = fingerprint_for_emit(original, &[]);
    let meta = ModuleMetadata {
        module_path: "foo".to_string(),
        analysis_fingerprint: Some(fp.into()),
        ..ModuleMetadata::new("foo".to_string())
    };
    let meta_path = windjammer::metadata::meta_cache_path(&wj);
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&meta_path, serde_json::to_string(&meta).unwrap()).unwrap();

    fs::write(&wj, updated).unwrap();
    thread::sleep(Duration::from_millis(10));
    fs::write(&rs, "// stale\n").unwrap();

    let sources = vec![(wj.clone(), updated.to_string())];
    let stale = cache_management::find_stale_codegen_outputs(&sources, &src, dir.path(), &[]);
    assert_eq!(stale, vec![wj]);
}

#[test]
fn test_mod_wj_merged_into_mod_rs_is_not_stale() {
    use windjammer::compiler::incremental::fingerprint_for_emit;
    use windjammer::metadata::ModuleMetadata;

    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let mod_wj = src.join("mod.wj");
    let source = "pub use player::Player;\n";
    fs::write(&mod_wj, source).unwrap();

    thread::sleep(Duration::from_millis(10));
    let mod_rs = dir.path().join("mod.rs");
    fs::write(&mod_rs, "// Auto-generated mod.rs\npub mod player;\n").unwrap();

    let fp = fingerprint_for_emit(source, &[]);
    let meta = ModuleMetadata {
        module_path: "mod".to_string(),
        analysis_fingerprint: Some(fp.into()),
        ..ModuleMetadata::new("mod".to_string())
    };
    let meta_path = windjammer::metadata::meta_cache_path(&mod_wj);
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&meta_path, serde_json::to_string(&meta).unwrap()).unwrap();

    let items_path = dir.path().join("_mod_items.rs");
    assert!(
        !items_path.exists(),
        "merged mod.wj should not require _mod_items.rs on disk"
    );

    let sources = vec![(mod_wj.clone(), source.to_string())];
    assert!(
        cache_management::find_stale_codegen_outputs(&sources, &src, dir.path(), &[]).is_empty(),
        "declaration-only mod.wj merged into mod.rs must not be reported stale"
    );
}

/// When `.rs` on disk diverges from what meta recorded at emit time, treat as stale.
#[test]
fn test_codegen_cache_invalid_when_output_hash_mismatch() {
    use windjammer::compiler::incremental::fingerprint_for_emit;
    use windjammer::metadata::ModuleMetadata;

    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let wj = src.join("foo.wj");
    let rs = dir.path().join("foo.rs");
    let source = "fn foo() {}\n";
    let emitted = "// correct generated\n";
    fs::write(&wj, source).unwrap();
    fs::write(&rs, emitted).unwrap();

    let mut fp = fingerprint_for_emit(source, &[]);
    fp.output_hash = windjammer::compiler::incremental::hash_output(emitted);
    let meta = ModuleMetadata {
        module_path: "foo".to_string(),
        analysis_fingerprint: Some(fp.into()),
        ..ModuleMetadata::new("foo".to_string())
    };
    let meta_path = windjammer::metadata::meta_cache_path(&wj);
    if let Some(parent) = meta_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&meta_path, serde_json::to_string(&meta).unwrap()).unwrap();

    // Stale .rs with wrong content but fresh mtime
    thread::sleep(Duration::from_millis(10));
    fs::write(&rs, "// stale wrong output\n").unwrap();

    assert!(!cache_management::is_codegen_cache_valid(
        source,
        &wj,
        &rs,
        &[] as &[PathBuf],
    ));

    let sources = vec![(wj.clone(), source.to_string())];
    let (dirty, skipped) =
        cache_management::compute_dirty_files(&sources, &src, dir.path(), &[]);
    assert_eq!(skipped, 0);
    assert_eq!(dirty.len(), 1);
}

/// Injected stdlib sources (outside the library `src_base`) must not fail the post-build
/// stale guard — they are analysis-only and emit to the compiler tree, not the user output.
#[test]
fn test_find_stale_codegen_outputs_skips_out_of_tree_stdlib_injects() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let user_wj = src.join("game.wj");
    fs::write(&user_wj, "use std::collections::HashMap\n").unwrap();
    let user_rs = dir.path().join("game.rs");
    fs::write(&user_rs, "// generated\n").unwrap();

    let stdlib_wj = dir.path().join("stdlib/collections.wj");
    fs::create_dir_all(stdlib_wj.parent().unwrap()).unwrap();
    fs::write(&stdlib_wj, "pub struct HashMap {}\n").unwrap();
    // No collections.rs under output — would be stale if checked.

    let sources = vec![
        (stdlib_wj.clone(), "pub struct HashMap {}\n".to_string()),
        (user_wj.clone(), "use std::collections::HashMap\n".to_string()),
    ];
    let stale = cache_management::find_stale_codegen_outputs(&sources, &src, dir.path(), &[]);
    assert!(
        !stale.contains(&stdlib_wj),
        "out-of-tree stdlib inject must not be stale-checked, got {:?}",
        stale
    );
}

/// Workspace parent manifest must not shadow the member crate that owns the `.wj` file.
#[test]
fn test_meta_cache_path_uses_nearest_crate_not_workspace_root() {
    let workspace = TempDir::new().unwrap();
    fs::write(
        workspace.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"member\"]\n",
    )
    .unwrap();
    let member = workspace.path().join("member");
    fs::create_dir_all(member.join("src")).unwrap();
    fs::write(member.join("Cargo.toml"), "[package]\nname = \"member\"\n").unwrap();
    let wj = member.join("src/foo.wj");
    fs::write(&wj, "fn foo() {}\n").unwrap();

    let meta_path = windjammer::metadata::meta_cache_path(&wj);
    assert_eq!(
        meta_path,
        member.join(".wj-cache/foo.wj.meta"),
        "meta cache must live at the member crate root, not the workspace root"
    );
}

/// Auxiliary `src/<module>/Cargo.toml` must not shadow the enclosing crate root.
#[test]
fn test_meta_cache_path_skips_src_module_nested_cargo_toml() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"game-core\"\n").unwrap();
    let module = dir.path().join("src/rendering");
    fs::create_dir_all(&module).unwrap();
    fs::write(module.join("Cargo.toml"), "[package]\nname = \"rendering\"\n").unwrap();
    let wj = module.join("camera.wj");
    fs::write(&wj, "fn camera() {}\n").unwrap();

    let meta_path = windjammer::metadata::meta_cache_path(&wj);
    assert_eq!(
        meta_path,
        dir.path().join(".wj-cache/rendering/camera.wj.meta"),
        "meta cache must live at the enclosing crate root, not src/rendering/.wj-cache"
    );
}

/// Compiler-generated `gen/Cargo.toml` must not shadow the real crate root.
#[test]
fn test_meta_cache_path_skips_generated_gen_cargo_toml() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"game-core\"\n").unwrap();
    let gen = dir.path().join("gen/testing");
    fs::create_dir_all(&gen).unwrap();
    fs::write(gen.parent().unwrap().join("Cargo.toml"), "[package]\nname = \"generated\"\n").unwrap();
    let wj = gen.join("scenario.wj");
    fs::write(&wj, "fn scenario() {}\n").unwrap();

    let meta_path = windjammer::metadata::meta_cache_path(&wj);
    assert_eq!(
        meta_path,
        dir.path().join(".wj-cache/testing/scenario.wj.meta"),
        "gen/ output mirror must cache under the real crate root with src-relative layout"
    );
}

/// Nested `src/Cargo.toml` (generated by older builds) must not shadow the real crate root.
#[test]
fn test_meta_cache_path_skips_nested_src_cargo_toml() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"outer\"\n").unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("Cargo.toml"), "[package]\nname = \"nested\"\n").unwrap();
    let wj = src.join("foo.wj");
    fs::write(&wj, "fn foo() {}\n").unwrap();

    let meta_path = windjammer::metadata::meta_cache_path(&wj);
    assert_eq!(
        meta_path,
        dir.path().join(".wj-cache/foo.wj.meta"),
        "meta cache must live at outer crate root, not src/.wj-cache"
    );
}
