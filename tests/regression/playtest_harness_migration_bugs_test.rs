#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD regression tests from playtest harness migration (2026-05-22).
///
/// 1. Static impl methods must not get &self when locals shadow struct field names.
/// 2. pub use re-exports must survive import dedup when the same symbol is used privately.
/// 3. Reading a String field then reusing the parent struct must auto-clone the field.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_static_impl_method_no_self_when_local_shadows_field_name() {
    let generated = test_utils::compile_single(
        r#"
struct PlaytestContext {
    player_x: f32,
    player_y: f32,
    player_z: f32,
}

struct ScenarioExecutor {
    active: bool,
    aim_yaw: f32,
}

impl ScenarioExecutor {
    fn compute_look(ctx: PlaytestContext, tx: f32, ty: f32, tz: f32) -> (f32, f32) {
        let ddx = tx - ctx.player_x
        let ddz = tz - ctx.player_z
        let horiz = (ddx * ddx + ddz * ddz).sqrt()
        let aim_yaw = if horiz > 0.01 { ddz.atan2(ddx) } else { 1.5707963 }
        let dy = ty - ctx.player_y
        let aim_pitch = if horiz > 0.01 { dy.atan2(horiz) } else { 0.0 }
        (aim_yaw, aim_pitch)
    }
}
"#,
    );

    assert!(
        generated.contains("fn compute_look(ctx: PlaytestContext, tx: f32, ty: f32, tz: f32)"),
        "Static associated function must not get implicit &self when locals shadow field names.\n\
         Bug: `let aim_yaw = ...` was mistaken for bare struct field access.\n\
         Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn compute_look(&self"),
        "Static associated function must not get &self.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_pub_use_kept_when_same_symbol_imported_privately() {
    let generated = test_utils::compile_single(
        r#"
use crate::rendering::gpu_renderer_types::{
    GpuCameraState, LightingConfig,
}
pub use crate::rendering::gpu_renderer_types::GpuCameraState
pub use crate::rendering::gpu_renderer_types::MeshPrimitiveDef

struct Local {
    camera: GpuCameraState,
    lighting: LightingConfig,
}
"#,
    );

    assert!(
        generated.contains("pub use crate::rendering::gpu_renderer_types::GpuCameraState"),
        "pub use re-export must not be dropped when the same symbol is imported privately.\n\
         Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("pub use crate::rendering::gpu_renderer_types::MeshPrimitiveDef"),
        "Other pub use re-exports must still be emitted.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_string_field_read_then_reuse_parent_struct_auto_clones() {
    let generated = test_utils::compile_single(
        r#"
struct Scenario {
    name: string,
    value: int,
}

struct ScenarioExecutor {
    scenario: Scenario,
    active: bool,
}

fn scenario_by_name(name: string) -> Scenario {
    Scenario { name: name, value: 1 }
}

fn make_executor(name: string) -> ScenarioExecutor {
    let scenario = scenario_by_name(name)
    let sc_name = scenario.name
    ScenarioExecutor { scenario: scenario, active: true }
}
"#,
    );

    assert!(
        generated.contains("scenario.name.clone()"),
        "Reading scenario.name then using scenario again must auto-clone the field.\n\
         Generated:\n{}",
        generated
    );
}
