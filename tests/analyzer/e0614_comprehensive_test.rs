//! E0614 Phase 11 Comprehensive Fix Tests
//!
//! Covers all 5 E0614 regression patterns:
//! - CameraData: if let Some(cam) = self.last_camera { camera_data_to_gpu_state(cam) }
//! - i64: if let Some(ref mut last_id) = &mut self.last_selected { *last_id == id }
//! - Entity: for entity in entities { process(entity) }
//! - SearchState: if let Some(search) = self.search { search.update() }
//! - InvestigationState: if let Some(inv) = self.investigation { inv.update() }
//!
//! Copy detection: copy_structs_registry (PASS 0) collects @derive(Copy) from source.
//! NEVER add application-specific types to is_known_copy_type - use generic detection.

#[path = "../common/test_utils.rs"]
mod test_utils;

/// CameraData - render_frame pattern
#[test]
fn test_camera_data_no_deref() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct CameraData {
    pub id: u32,
}

pub struct GpuState {}

pub fn camera_data_to_gpu_state(camera: CameraData) -> GpuState {
    GpuState {}
}

pub struct GameRenderer {
    last_camera: Option<CameraData>,
}

impl GameRenderer {
    pub fn new() -> GameRenderer {
        GameRenderer { last_camera: None }
    }
    fn render_frame(self) {
        if let Some(cam) = self.last_camera {
            let gpu_state = camera_data_to_gpu_state(cam);
            let _ = gpu_state;
        }
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(cam)"),
        "Should NOT add *(cam) for Copy CameraData. Generated:\n{}",
        rs
    );
}

/// i64 primitive - Option pattern
#[test]
fn test_i64_primitive_no_deref() {
    let source = r#"
pub fn process(value: i64) -> i64 {
    value + 1
}

pub struct Container {
    last_id: Option<i64>,
}

impl Container {
    pub fn new() -> Container {
        Container { last_id: None }
    }
    fn check(self, entity_id: i64) -> bool {
        if let Some(last_id) = self.last_id {
            last_id == entity_id
        } else {
            false
        }
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(last_id)"),
        "Should NOT add *(last_id) for i64. Generated:\n{}",
        rs
    );
}

/// InvestigationState - match Option pattern
#[test]
fn test_investigation_state_no_deref() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct InvestigationState {
    pub complete: bool,
}

impl InvestigationState {
    pub fn update(self, _dt: f32) -> bool {
        self.complete
    }
    pub fn is_complete(self) -> bool {
        self.complete
    }
}

pub struct NPC {
    investigation: Option<InvestigationState>,
}

impl NPC {
    pub fn new() -> NPC {
        NPC { investigation: None }
    }
    fn update(self, dt: f32) {
        if let Some(inv) = self.investigation {
            if !inv.update(dt) || inv.is_complete() {
            } else {
                let _ = inv;
            }
        }
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(inv)"),
        "Should NOT add *(inv) for Copy InvestigationState. Generated:\n{}",
        rs
    );
}

/// SearchState - match Option pattern
#[test]
fn test_search_state_no_deref() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct SearchState {
    pub complete: bool,
}

impl SearchState {
    pub fn update(self, _dt: f32) -> bool {
        self.complete
    }
    pub fn is_complete(self) -> bool {
        self.complete
    }
}

pub struct NPC {
    search: Option<SearchState>,
}

impl NPC {
    pub fn new() -> NPC {
        NPC { search: None }
    }
    fn update(self, dt: f32) {
        if let Some(search) = self.search {
            if !search.update(dt) || search.is_complete() {
            } else {
                let _ = search;
            }
        }
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(search)"),
        "Should NOT add *(search) for Copy SearchState. Generated:\n{}",
        rs
    );
}

/// Entity - for loop pattern
#[test]
fn test_entity_all_patterns() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct Entity {
    pub id: u32,
}

pub fn process(entity: Entity) {
    let _ = entity.id;
}

pub fn process_all(entities: Vec<Entity>) {
    for entity in entities {
        process(entity)
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(entity)"),
        "Should NOT add *(entity) for Copy Entity. Generated:\n{}",
        rs
    );
}
