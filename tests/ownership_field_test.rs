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

//! Dogfooding ownership — fields, indexing, cross-type propagation.
#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// TEST 4: Copy type self.field passed to method - AI system (ai/state_machine.wj)
//
// Real game code: transition.matches(self.current_state)
// where current_state is i32 (Copy type).
// The compiler should NOT add & to self.field when it's a Copy type.
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_self_field_copy_type_passed_to_method() {
    let source = r#"
pub struct Transition {
    pub from_state: i32,
    pub to_state: i32,
}

impl Transition {
    pub fn matches(state: i32) -> bool {
        self.from_state == state
    }
}

pub struct StateMachine {
    pub current_state: i32,
    pub transitions: Vec<Transition>,
}

impl StateMachine {
    pub fn should_transition(to_state: i32) -> bool {
        for transition in &self.transitions {
            if transition.matches(self.current_state) && transition.to_state == to_state {
                return true
            }
        }
        false
    }
}
"#;

    let (generated, _) = test_utils::compile_via_cli_with_stderr(source);

    // self.current_state is i32 (Copy), should NOT be &self.current_state
    assert!(
        !generated.contains("transition.matches(&self.current_state)"),
        "COMPILER BUG: self.current_state is i32 (Copy), should NOT add &.\n\
         The user wrote: transition.matches(self.current_state)\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST 5: Vec::remove with usize variable - ECS system (ecs/components.wj)
//
// Real game code: self.dense.remove(sparse_idx_usize)
// where sparse_idx_usize is explicitly typed as usize.
// Vec::remove takes usize by value, NOT by reference.
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_usize_variable() {
    let source = r#"
pub struct SparseSet {
    sparse: Vec<int>,
    dense: Vec<int>,
    entities: Vec<int>,
}

impl SparseSet {
    pub fn remove(entity_index: usize) -> Option<int> {
        if entity_index >= self.sparse.len() {
            return None
        }

        let sparse_index: int = self.sparse[entity_index]
        if sparse_index < 0 {
            return None
        }

        let sparse_idx_usize: usize = sparse_index as usize
        let component = self.dense.remove(sparse_idx_usize)
        let removed_entity = self.entities.remove(sparse_idx_usize)

        self.sparse[entity_index] = -1

        Some(component)
    }
}
"#;

    let (generated, _) = test_utils::compile_via_cli_with_stderr(source);

    // sparse_idx_usize is usize, Vec::remove takes usize by value
    assert!(
        !generated.contains(".remove(&sparse_idx_usize)"),
        "COMPILER BUG: sparse_idx_usize is usize, Vec::remove takes by value.\n\
         The user wrote: self.dense.remove(sparse_idx_usize)\n\
         Generated:\n{}",
        generated
    );
}

// ============================================================================
// TEST 6: Cross-type mutation inference via self.field.method()
//         (terrain/terrain.wj + terrain/heightmap.wj)
//
// Real game code: Terrain::raise() calls self.heightmap.set(px, pz, value)
// HeightMap::set mutates self.data[...], so it requires &mut self.
// Since Terrain::raise calls self.heightmap.set(), it requires &mut self too.
// The compiler must infer &mut self for BOTH HeightMap::set and Terrain::raise.
//
// This tests cross-type mutation propagation through self.field.method().
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cross_type_mutation_via_field_method() {
    let source = r#"
pub struct HeightMap {
    width: usize,
    height: usize,
    data: Vec<f32>,
}

impl HeightMap {
    pub fn new(width: usize, height: usize) -> HeightMap {
        let data = Vec::with_capacity(width * height)
        HeightMap {
            width: width,
            height: height,
            data: data,
        }
    }

    pub fn get(self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.data[y * self.width + x]
        } else {
            0.0
        }
    }

    pub fn set(self, x: usize, y: usize, value: f32) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value
        }
    }
}

pub struct Terrain {
    heightmap: HeightMap,
    scale: f32,
}

impl Terrain {
    pub fn new(width: usize, height: usize, scale: f32) -> Terrain {
        Terrain {
            heightmap: HeightMap::new(width, height),
            scale: scale,
        }
    }

    pub fn get_height(self, x: f32, z: f32) -> f32 {
        let grid_x = (x / self.scale) as usize
        let grid_z = (z / self.scale) as usize
        self.heightmap.get(grid_x, grid_z)
    }

    pub fn raise(self, x: f32, z: f32, radius: f32, strength: f32) {
        let grid_x = (x / self.scale) as i32
        let grid_z = (z / self.scale) as i32
        let grid_radius = (radius / self.scale) as i32

        let mut dz = -grid_radius
        while dz <= grid_radius {
            let mut dx = -grid_radius
            while dx <= grid_radius {
                let px = (grid_x + dx) as usize
                let pz = (grid_z + dz) as usize

                let current = self.heightmap.get(px, pz)
                self.heightmap.set(px, pz, current + strength)

                dx = dx + 1
            }
            dz = dz + 1
        }
    }

    pub fn lower(self, x: f32, z: f32, radius: f32, strength: f32) {
        self.raise(x, z, radius, -strength)
    }

    pub fn clear(self) {
        self.heightmap.clear()
    }
}
"#;

    let (generated, _) = test_utils::compile_via_cli_with_stderr(source);

    // HeightMap::set mutates self.data[...], so it MUST be &mut self
    assert!(
        generated.contains("pub fn set(&mut self"),
        "COMPILER BUG: HeightMap::set mutates self.data[idx], should be &mut self.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::raise calls self.heightmap.set(), so it MUST be &mut self
    assert!(
        generated.contains("pub fn raise(&mut self"),
        "COMPILER BUG: Terrain::raise calls self.heightmap.set() which mutates.\n\
         The compiler must propagate mutation through self.field.method() calls.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::lower calls self.raise(), so it MUST be &mut self
    assert!(
        generated.contains("pub fn lower(&mut self"),
        "COMPILER BUG: Terrain::lower calls self.raise() which is &mut self.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::clear calls self.heightmap.clear(), which is a known mutating method
    assert!(
        generated.contains("pub fn clear(&mut self"),
        "COMPILER BUG: Terrain::clear calls self.heightmap.clear() which mutates.\n\
         Generated:\n{}",
        generated
    );

    // Terrain::get_height only reads, so it should be &self
    assert!(
        generated.contains("pub fn get_height(&self"),
        "REGRESSION: Terrain::get_height only reads, should be &self.\n\
         Generated:\n{}",
        generated
    );

    // HeightMap::get only reads, so it should be &self
    assert!(
        generated.contains("pub fn get(&self"),
        "REGRESSION: HeightMap::get only reads, should be &self.\n\
         Generated:\n{}",
        generated
    );
}
