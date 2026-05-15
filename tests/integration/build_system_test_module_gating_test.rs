// TDD Tests: Compiler generates correct mod.rs / lib.rs / Cargo.toml for
// projects with test modules and wj.toml configuration.
//
// Verifies:
// 1. Test modules get #[cfg(test)] gating on pub mod declarations
// 2. Test modules are excluded from pub use re-exports
// 3. wj.toml dev-dependencies are merged into generated Cargo.toml
// 4. Self-referencing dependencies are filtered out
// 5. Nested Cargo.toml files are cleaned up

use std::fs;
use tempfile::TempDir;

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_test_modules_get_cfg_test_gate() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let out_dir = tmp.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("mod.wj"),
        "pub mod player\npub mod player_test\n",
    )
    .unwrap();

    fs::write(
        src_dir.join("player.wj"),
        "pub struct Player { pub hp: int }\n",
    )
    .unwrap();

    fs::write(
        src_dir.join("player_test.wj"),
        r#"
use crate::player::Player

pub fn test_player_created() {
    let p = Player { hp: 100 }
    assert(p.hp == 100, "hp should be 100")
}
"#,
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = std::process::Command::new(&wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("wj build failed to start");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mod_rs = fs::read_to_string(out_dir.join("mod.rs")).expect("mod.rs should exist");
    println!("Generated mod.rs:\n{}", mod_rs);

    assert!(
        mod_rs.contains("#[cfg(test)]\npub mod player_test;"),
        "player_test should be gated with #[cfg(test)].\nActual mod.rs:\n{}",
        mod_rs
    );

    assert!(
        mod_rs.contains("pub mod player;"),
        "player module should NOT be gated"
    );

    assert!(
        !mod_rs.contains("pub use player_test"),
        "player_test should NOT be re-exported.\nActual mod.rs:\n{}",
        mod_rs
    );

    // lib.rs should propagate the gate from mod.rs
    let lib_rs = fs::read_to_string(out_dir.join("lib.rs")).expect("lib.rs should exist");
    assert!(
        lib_rs.contains("#[cfg(test)]\npub mod player_test;"),
        "lib.rs should also gate player_test with #[cfg(test)]"
    );
}

#[test]
fn test_self_referencing_dep_filtered() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let out_dir = tmp.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("lib.wj"),
        "pub fn hello() -> string { \"hello\" }\n",
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = std::process::Command::new(&wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build failed to start");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cargo_toml =
        fs::read_to_string(out_dir.join("Cargo.toml")).expect("Cargo.toml should exist");

    let package_name = cargo_toml
        .lines()
        .find(|l| l.trim().starts_with("name"))
        .expect("should have package name");
    let pkg = package_name
        .split('=')
        .nth(1)
        .unwrap()
        .trim()
        .trim_matches('"');

    for line in cargo_toml.lines() {
        if line.trim().starts_with('[') || line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let dep_name = line.split('=').next().unwrap_or("").trim();
        assert_ne!(
            dep_name.replace('-', "_"),
            pkg.replace('-', "_"),
            "Cargo.toml should not contain a self-referencing dependency: {}",
            line
        );
    }
}

#[test]
fn test_wj_toml_dev_deps_merged_into_cargo_toml() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let out_dir = tmp.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();

    // Create wj.toml in project root (parent of src)
    fs::write(
        tmp.path().join("wj.toml"),
        r#"
[project]
name = "test-project"

[dev-dependencies]
criterion = "0.5"
proptest = { version = "1.0", features = ["attr-macro"] }
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("lib.wj"),
        "pub fn add(a: int, b: int) -> int { a + b }\n",
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = std::process::Command::new(&wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build failed to start");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cargo_toml =
        fs::read_to_string(out_dir.join("Cargo.toml")).expect("Cargo.toml should exist");
    println!("Generated Cargo.toml:\n{}", cargo_toml);

    assert!(
        cargo_toml.contains("[dev-dependencies]"),
        "Cargo.toml should have [dev-dependencies] section"
    );
    assert!(
        cargo_toml.contains("criterion"),
        "Cargo.toml should contain criterion dev-dep"
    );
    assert!(
        cargo_toml.contains("proptest"),
        "Cargo.toml should contain proptest dev-dep"
    );
    assert!(
        cargo_toml.contains("attr-macro"),
        "Cargo.toml should preserve proptest features"
    );
}

#[test]
fn test_nested_cargo_toml_cleaned() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let out_dir = tmp.path().join("build");
    let nested_dir = out_dir.join("rendering");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&nested_dir).unwrap();

    // Plant a stale nested Cargo.toml
    fs::write(
        nested_dir.join("Cargo.toml"),
        "[package]\nname = \"stale-nested\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    fs::write(src_dir.join("mod.wj"), "pub mod rendering\n").unwrap();

    fs::create_dir_all(src_dir.join("rendering")).unwrap();
    fs::write(src_dir.join("rendering/mod.wj"), "").unwrap();

    fs::write(
        src_dir.join("rendering/camera.wj"),
        "pub struct Camera { pub fov: float }\n",
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = std::process::Command::new(&wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("wj build failed to start");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        !nested_dir.join("Cargo.toml").exists(),
        "Nested Cargo.toml should be cleaned up by the compiler"
    );

    assert!(
        out_dir.join("Cargo.toml").exists(),
        "Root Cargo.toml should still exist"
    );
}

#[test]
fn test_multiple_test_module_patterns_gated() {
    let tmp = TempDir::new().unwrap();
    let src_dir = tmp.path().join("src");
    let out_dir = tmp.path().join("build");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("mod.wj"),
        r#"
pub mod core
pub mod core_test
pub mod core_tests
pub mod test_helpers
pub mod tests
"#,
    )
    .unwrap();

    fs::write(src_dir.join("core.wj"), "pub fn init() {}\n").unwrap();
    fs::write(src_dir.join("core_test.wj"), "pub fn test_init() {}\n").unwrap();
    fs::write(src_dir.join("core_tests.wj"), "pub fn test_all() {}\n").unwrap();
    fs::write(src_dir.join("test_helpers.wj"), "pub fn setup() {}\n").unwrap();
    fs::write(src_dir.join("tests.wj"), "pub fn run_tests() {}\n").unwrap();

    let wj = test_utils::wj_binary();
    let output = std::process::Command::new(&wj)
        .args([
            "build",
            src_dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--library",
            "--no-cargo",
        ])
        .output()
        .expect("wj build failed to start");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mod_rs = fs::read_to_string(out_dir.join("mod.rs")).expect("mod.rs should exist");
    println!("Generated mod.rs:\n{}", mod_rs);

    // All test-like modules should be gated
    for test_mod in &["core_test", "core_tests", "test_helpers", "tests"] {
        let expected = format!("#[cfg(test)]\npub mod {};", test_mod);
        assert!(
            mod_rs.contains(&expected),
            "{} should be gated with #[cfg(test)].\nExpected: {}\nActual mod.rs:\n{}",
            test_mod,
            expected,
            mod_rs
        );
    }

    // Non-test module should NOT be gated
    assert!(
        !mod_rs.contains("#[cfg(test)]\npub mod core;"),
        "core module should NOT be gated"
    );

    // Test modules should NOT be re-exported
    for test_mod in &["core_test", "core_tests", "test_helpers", "tests"] {
        assert!(
            !mod_rs.contains(&format!("pub use {}::", test_mod)),
            "{} should NOT be re-exported",
            test_mod
        );
    }
}
