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

#[path = "common/wjsl_shader_fixtures.rs"]
mod wjsl_shader_fixtures;

// TDD Test: HUD Overlay Shader
//
// Verifies that hud_overlay.wjsl transpiles correctly and contains
// all required HUD elements.

fn transpile_hud() -> String {
    wjsl_shader_fixtures::transpile_fixture_shader("hud_overlay.wjsl")
        .expect("hud_overlay.wjsl should transpile")
}

#[test]
fn test_hud_shader_transpiles() {
    let wgsl = transpile_hud();
    assert!(!wgsl.is_empty(), "Generated WGSL should not be empty");
    assert!(wgsl.contains("fn main"), "Should contain main entry point");
}

#[test]
fn test_hud_has_crosshair() {
    let wgsl = transpile_hud();
    assert!(
        wgsl.contains("draw_crosshair"),
        "HUD shader should contain crosshair drawing function"
    );
}

#[test]
fn test_hud_has_health_bar() {
    let wgsl = transpile_hud();
    assert!(
        wgsl.contains("draw_health_bar"),
        "HUD shader should contain health bar drawing function"
    );
}

#[test]
fn test_hud_has_ammo_display() {
    let wgsl = transpile_hud();
    assert!(
        wgsl.contains("draw_ammo_bar"),
        "HUD shader should contain ammo bar drawing function"
    );
}

#[test]
fn test_hud_has_damage_flash() {
    let wgsl = transpile_hud();
    assert!(
        wgsl.contains("draw_damage_flash"),
        "HUD shader should contain damage flash function"
    );
}

#[test]
fn test_hud_params_struct() {
    let wgsl = transpile_hud();
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
