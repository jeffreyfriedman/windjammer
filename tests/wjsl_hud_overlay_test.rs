// TDD Test: HUD Overlay Shader
//
// Verifies that hud_overlay.wjsl transpiles correctly and contains
// all required HUD elements.

use std::fs;

fn transpile(wjsl_path: &str) -> Result<String, String> {
    let source = fs::read_to_string(wjsl_path)
        .map_err(|e| format!("Failed to read {}: {}", wjsl_path, e))?;
    windjammer::wjsl::transpile_wjsl(&source)
        .map_err(|e| format!("WJSL transpilation failed: {}", e))
}

#[test]
fn test_hud_shader_transpiles() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/hud_overlay.wjsl")
        .expect("hud_overlay.wjsl should transpile");
    assert!(!wgsl.is_empty(), "Generated WGSL should not be empty");
    assert!(wgsl.contains("fn main"), "Should contain main entry point");
}

#[test]
fn test_hud_has_crosshair() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/hud_overlay.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_crosshair"),
        "HUD shader should contain crosshair drawing function"
    );
}

#[test]
fn test_hud_has_health_bar() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/hud_overlay.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_health_bar"),
        "HUD shader should contain health bar drawing function"
    );
}

#[test]
fn test_hud_has_ammo_display() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/hud_overlay.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_ammo_bar"),
        "HUD shader should contain ammo bar drawing function"
    );
}

#[test]
fn test_hud_has_damage_flash() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/hud_overlay.wjsl")
        .expect("transpilation should succeed");
    assert!(
        wgsl.contains("draw_damage_flash"),
        "HUD shader should contain damage flash function"
    );
}

#[test]
fn test_hud_params_struct() {
    let wgsl = transpile("../windjammer-game/windjammer-game-core/shaders/hud_overlay.wjsl")
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
