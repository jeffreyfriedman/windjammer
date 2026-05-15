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
))]

use std::path::Path;
use windjammer::project_paths::find_source_root;

#[test]
fn test_project_src_is_source_root() {
    let path = Path::new("/project/src/main.wj");
    let root = find_source_root(path);
    assert!(root.is_some());
    let root_path = root.unwrap();
    assert!(
        root_path.ends_with("src"),
        "Expected project src/ as source root, got {:?}",
        root_path
    );
}

#[test]
fn test_personal_src_not_matched_as_source_root() {
    // /Users/dev/src/wj/windjammer/target/test_dir/test.wj
    // The /Users/.../src/ directory is NOT a project source root.
    let path = Path::new("/Users/dev/src/wj/windjammer/target/test_dir/test.wj");
    let root = find_source_root(path);
    assert!(root.is_some());
    let root_path = root.unwrap();
    assert!(
        !root_path.ends_with("/src"),
        "Should NOT match personal /src/ as source root. Got {:?}",
        root_path
    );
    assert_eq!(
        root_path,
        Path::new("/Users/dev/src/wj/windjammer/target/test_dir"),
        "For standalone files, should return the file's parent directory"
    );
}

#[test]
fn test_standalone_file_returns_parent() {
    let path = Path::new("/tmp/test_dir/hello.wj");
    let root = find_source_root(path);
    assert!(root.is_some());
    assert_eq!(
        root.unwrap(),
        Path::new("/tmp/test_dir"),
        "Standalone file should use parent as source root"
    );
}

#[test]
fn test_deeply_nested_src() {
    let path = Path::new("/home/user/projects/game/src/engine/physics/rigid_body.wj");
    let root = find_source_root(path);
    assert!(root.is_some());
    // The src directory here has no Cargo.toml sibling or mod.wj,
    // so it won't be matched as a project source root.
    // Falls back to parent directory.
    let root_path = root.unwrap();
    assert_eq!(
        root_path,
        Path::new("/home/user/projects/game/src/engine/physics"),
        "Without project markers, should fall back to parent. Got {:?}",
        root_path
    );
}
