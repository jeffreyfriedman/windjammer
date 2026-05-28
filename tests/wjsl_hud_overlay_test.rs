#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

// TDD Test: HUD Overlay Shader
//
// Verifies that hud_overlay.wjsl transpiles correctly and contains
// all required HUD elements.

use std::fs;
use std::path::Path;

fn hud_overlay_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../windjammer-game/windjammer-game-core/src/shaders/hud_overlay.wjsl",
    )
}

fn game_shaders_available() -> bool {
    hud_overlay_path().exists()
}

fn transpile(wjsl_path: &str) -> Result<String, String> {
    let source = fs::read_to_string(wjsl_path)
        .map_err(|e| format!("Failed to read {}: {}", wjsl_path, e))?;
    windjammer::wjsl::transpile_wjsl(&source)
        .map_err(|e| format!("WJSL transpilation failed: {}", e))
}

#[test]
fn test_hud_shader_transpiles() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile(hud_overlay_path().to_str().unwrap())
        .expect("hud_overlay.wjsl should transpile");
    assert!(!wgsl.is_empty(), "Generated WGSL should not be empty");
    assert!(wgsl.contains("fn main"), "Should contain main entry point");
}

#[test]
fn test_hud_has_crosshair() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile(hud_overlay_path().to_str().unwrap())
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_crosshair"),
        "HUD shader should contain crosshair drawing function"
    );
}

#[test]
fn test_hud_has_health_bar() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile(hud_overlay_path().to_str().unwrap())
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_health_bar"),
        "HUD shader should contain health bar drawing function"
    );
}

#[test]
fn test_hud_has_ammo_display() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile(hud_overlay_path().to_str().unwrap())
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_ammo_bar"),
        "HUD shader should contain ammo bar drawing function"
    );
}

#[test]
fn test_hud_has_damage_flash() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile(hud_overlay_path().to_str().unwrap())
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_damage_flash"),
        "HUD shader should contain damage flash function"
    );
}

#[test]
fn test_hud_params_struct() {
    if !game_shaders_available() {
        eprintln!("SKIP: windjammer-game shaders not available");
        return;
    }
    let wgsl = transpile(hud_overlay_path().to_str().unwrap())
        .expect("transpilation should succeed");
    assert!(wgsl.contains("HudParams"), "Should have HudParams struct");
    assert!(
        wgsl.contains("health"),
        "HudParams should have health field"
    );
    assert!(wgsl.contains("ammo"), "HudParams should have ammo field");
    assert!(
        wgsl.contains("damage_flash"),
        "HudParams should have damage_flash field"
    );
}
