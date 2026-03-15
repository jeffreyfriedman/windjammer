//! TDD: E0507 "cannot move out of" ownership inference
//!
//! Patterns fixed:
//! 1. Vec indexing non-Copy types → auto-borrow (&vec[i]) or auto-clone when owned needed
//! 2. Vec indexing + method with owned self → auto-clone (vec[i].clone().method())
//! 3. Option if let with &self → &self.field
//! 4. Option if let with &mut self + mutation → &mut self.field, mut binding
//! 5. Option match with &self → &self.field
//! 6. Option::map with &self → .as_ref().map(...)
//! 7. Moving from &self (method takes owned self) → infer &mut self or clone
//! 8. Struct literal field from Vec index → .clone()

use std::process::Command;

fn get_wj_binary() -> String {
    env!("CARGO_BIN_EXE_wj").to_string()
}

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    std::fs::write(&wj_path, wj_source).expect("Failed to write test file");
    std::fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_binary())
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let src_main = out_dir.join("src").join("main.rs");
    let test_rs = out_dir.join("test.rs");
    let content = if src_main.exists() {
        std::fs::read_to_string(src_main)
    } else if test_rs.exists() {
        std::fs::read_to_string(test_rs)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No generated Rust file found",
        ))
    };
    content.map_err(|e| e.to_string())
}

fn rust_compiles(rust_code: &str) -> bool {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let rs_path = temp_dir.path().join("test.rs");
    std::fs::write(&rs_path, rust_code).expect("write");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .arg(&rs_path)
        .output()
        .expect("rustc");
    output.status.success()
}

#[test]
fn test_vec_string_index_generates_borrow() {
    let source = r#"
pub fn get_line(lines: Vec<string>, index: i32) -> string {
    let line = lines[index]
    return line
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust.contains("&lines[") || rust.contains("& lines["), "Vec<String> index needs &: {}", rust);
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_vec_index_method_owned_self_generates_clone() {
    // BoneTrack.sample(self, time) takes owned self - vec[i].sample(time) needs .clone()
    // BoneTrack has String field (non-Copy) so we need .clone() when method takes owned self
    let source = r#"
pub struct Keyframe { pub time: f32 }
pub struct BoneTrack { pub bone_id: u32, pub name: string }
impl BoneTrack {
    pub fn sample(self, time: f32) -> Keyframe { Keyframe { time } }
}
pub struct Clip { pub tracks: Vec<BoneTrack> }
impl Clip {
    pub fn sample_bone(self, bone_id: u32, time: f32) -> Option<Keyframe> {
        let mut i = 0
        while i < self.tracks.len() {
            if self.tracks[i].bone_id == bone_id {
                return Some(self.tracks[i].sample(time))
            }
            i = i + 1
        }
        None
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("tracks[i].clone().sample") || rust.contains("tracks[i].clone().sample("),
        "Vec index + method(owned self) needs .clone(): {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_if_let_borrows_self_field() {
    let source = r#"
pub struct Item { pub damage: i32 }
pub struct Equipment { pub weapon: Option<Item> }
impl Equipment {
    pub fn get_damage(self) -> i32 {
        let mut total = 0
        if let Some(stack) = self.weapon {
            total = stack.damage
        }
        total
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust.contains("&self.weapon"), "Option if let needs &self.field: {}", rust);
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_map_uses_as_ref() {
    let source = r#"
pub struct Node { pub value: i32, pub children: Option<Vec<i32>> }
impl Node {
    pub fn count(self) -> i32 {
        self.children.map(|c| c.len() as i32).unwrap_or(0)
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust.contains(".as_ref().map("), "Option::map needs .as_ref(): {}", rust);
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_vec_non_copy_index_let_binding() {
    // let preset = self.available_presets[i] - read-only, & is preferred
    let source = r#"
pub struct Preset { pub name: string }
pub struct Editor { pub presets: Vec<Preset> }
impl Editor {
    pub fn get_name(self, index: i32) -> string {
        let p = self.presets[index as usize]
        p.name
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    // Need & for vec[index] - can't move out of index
    assert!(
        rust.contains("&self.presets[") || rust.contains("& self.presets["),
        "Vec<Preset> index needs borrow: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_match_borrows_self_field() {
    let source = r#"
pub struct Health { pub current: i32 }
pub struct Entity { pub health: Option<Health> }
impl Entity {
    pub fn has_health(self) -> bool {
        match self.health {
            Some(h) => h.current > 0,
            None => false
        }
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust.contains("&self.health"), "Option match needs &self.field: {}", rust);
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_vec_index_field_access_no_clone() {
    // vec[i].field - Rust allows field access through &T, so &vec[i] works
    let source = r#"
pub struct Point { pub x: f32, pub y: f32 }
pub fn get_x(points: Vec<Point>, i: i32) -> f32 {
    points[i].x
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Field access through Vec index must compile: {}", rust);
}

#[test]
fn test_for_loop_param_used_multiple_times_borrows() {
    // E0382 fix: for comp in entity_components used in nested loops must use &entity_components
    let source = r#"
pub fn matches(required: Vec<string>, excluded: Vec<string>, entity_components: Vec<string>) -> bool {
    for req in required {
        for comp in entity_components {
            if comp == req {
                return true
            }
        }
    }
    for excl in excluded {
        for comp in entity_components {
            if comp == excl {
                return false
            }
        }
    }
    true
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("&entity_components") || rust.contains("& entity_components"),
        "Param used in multiple for-loops needs &: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_match_param_field_borrows() {
    // E0507 fix: match node.children { Some(c) => ... } when node is &T needs &node.children
    let source = r#"
pub struct OctreeNode { pub value: u8, pub children: Option<Vec<OctreeNode>> }
pub fn get_children(node: OctreeNode) -> Option<Vec<OctreeNode>> {
    match node.children {
        Some(c) => Some(c),
        None => None
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    // When node is passed by value, no borrow needed. When by ref, need &node.children.
    // The analyzer infers from usage - if we pass &node, we need the fix.
    assert!(rust_compiles(&rust), "Generated Rust must compile: {}", rust);
}

#[test]
fn test_struct_literal_from_vec_index_clones() {
    // let x = Foo { f: vec[i] } - needs owned value, so .clone()
    let source = r#"
pub struct Item { pub name: string }
pub struct Wrapper { pub item: Item }
pub fn wrap(items: Vec<Item>, i: i32) -> Wrapper {
    Wrapper { item: items[i] }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains(".clone()"),
        "Struct literal field from Vec index needs .clone(): {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_match_iterator_var_field_borrows() {
    // E0507 fix: match entity.health { Some(h) => ... } when entity from for entity in &self.entities
    // With &self, for entity in self.entities gets &self.entities (field access), entity is &Entity.
    let source = r#"
pub struct Health { pub current: i32 }
pub struct Entity { pub health: Option<Health> }
pub struct World { pub entities: Vec<Entity> }
impl World {
    pub fn count_healthy(self) -> i32 {
        let mut count = 0
        for entity in self.entities {
            match entity.health {
                Some(h) => if h.current > 0 { count = count + 1 },
                None => {}
            }
        }
        count
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Option match on iterator var field must compile: {}", rust);
}

#[test]
fn test_option_if_let_mut_self_borrows() {
    // E0507 fix: if let Some(search) = self.search when &mut self needs &mut self.search
    let source = r#"
pub struct SearchState { pub active: bool }
pub struct Npc { pub search: Option<SearchState> }
impl Npc {
    pub fn update(self, dt: f32) -> bool {
        if let Some(search) = self.search {
            search.active
        } else {
            false
        }
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Option if let with &mut self must compile: {}", rust);
}

#[test]
fn test_option_if_let_mut_self_with_mutation_borrows() {
    // E0507 fix: npc_behavior pattern - if let Some(search) = self.search when &mut self,
    // need &mut self.search to avoid "cannot move out of borrowed content"
    // SearchState::update mutates self.time_searching so analyzer infers &mut self
    let source = r#"
pub struct SearchState { pub time_searching: f32 }
pub struct Npc { pub search: Option<SearchState> }
impl SearchState {
    pub fn update(self, dt: f32) -> bool {
        self.time_searching = self.time_searching + dt
        self.time_searching < 10.0
    }
    pub fn is_complete(self) -> bool { self.time_searching >= 10.0 }
}
impl Npc {
    pub fn update(self, dt: f32) {
        if let Some(search) = self.search {
            if !search.update(dt) || search.is_complete() {
                return
            }
        }
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    // Must generate &self.search or &mut self.search to avoid E0507 (cannot move out of borrowed)
    assert!(
        rust.contains("&self.search") || rust.contains("&mut self.search"),
        "Option if let with &mut self needs borrow: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Option if let must compile: {}", rust);
}

#[test]
fn test_struct_literal_self_field_clones() {
    // E0507 fix: PassDef { bindings: self.bindings } when &self needs .clone()
    let source = r#"
pub struct Binding { pub id: i32 }
pub struct PassDef { pub bindings: Vec<Binding> }
pub struct Builder { pub bindings: Vec<Binding> }
impl Builder {
    pub fn build(self) -> PassDef {
        PassDef { bindings: self.bindings }
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    // When self is &self, self.bindings needs .clone() for struct literal
    assert!(rust_compiles(&rust), "Struct literal from borrowed self.field must compile: {}", rust);
}

#[test]
fn test_param_used_in_multiple_nested_loops_borrows() {
    // E0382 fix: entity_components used in nested for loops needs &entity_components
    let source = r#"
pub fn matches(required: Vec<string>, excluded: Vec<string>, entity_components: Vec<string>) -> bool {
    for req in required {
        for comp in entity_components {
            if comp == req {
                return true
            }
        }
    }
    for excl in excluded {
        for comp in entity_components {
            if comp == excl {
                return false
            }
        }
    }
    true
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("&entity_components") || rust.contains("& entity_components"),
        "Param in multiple nested loops needs &: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_vec_index_let_binding_borrows() {
    // E0507: let preset = self.available_presets[i] needs & when non-Copy
    let source = r#"
pub struct Preset { pub name: string }
pub struct Editor { pub presets: Vec<Preset> }
impl Editor {
    pub fn get_preset_name(self, index: i32) -> string {
        let p = self.presets[index as usize]
        p.name
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("&self.presets[") || rust.contains("& self.presets["),
        "Vec<Preset> index in let needs borrow: {}",
        rust
    );
    assert!(rust_compiles(&rust), "Generated Rust must compile");
}

#[test]
fn test_option_if_let_param_field_borrows() {
    // E0507: if let Some(vel) = entity.velocity when entity is & from iterator
    let source = r#"
pub struct Velocity { pub dx: f32, pub dy: f32 }
pub struct Transform { pub x: f32, pub y: f32 }
pub struct Entity { pub velocity: Option<Velocity>, pub transform: Transform }
pub struct World { pub entities: Vec<Entity> }
impl World {
    pub fn has_velocity(self) -> bool {
        for entity in self.entities {
            if let Some(_vel) = entity.velocity {
                return true
            }
        }
        false
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Option if let on iterator var field must compile: {}", rust);
}

#[test]
fn test_builder_pattern_self_clone_when_owned_method() {
    // E0507: self.input_uniform(buffer) when &self and method takes owned self (builder pattern)
    let source = r#"
pub struct Uniform<T> { _p: T }
pub struct PassBuilder { pub pass_id: i32 }
impl PassBuilder {
    pub fn input_uniform(self, buffer: Uniform<i32>) -> PassBuilder { self }
    pub fn build(self) -> i32 { self.pass_id }
}
pub fn create_pass() -> i32 {
    let buffer = Uniform { _p: 0 }
    let builder = PassBuilder { pass_id: 1 }
    builder.input_uniform(buffer).build()
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Builder pattern must compile: {}", rust);
}

#[test]
fn test_option_match_param_returns_owned_clones() {
    // E0507: match &node.children { Some(c) => Some(c) } when node is &T - c is &Vec, need Some(c.clone())
    let source = r#"
pub struct OctreeNode { pub value: u8, pub children: Option<Vec<OctreeNode>> }
pub fn get_children(node: OctreeNode) -> Option<Vec<OctreeNode>> {
    match node.children {
        Some(c) => Some(c),
        None => None
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Option match param returning owned must compile: {}", rust);
}

#[test]
fn test_struct_literal_nested_field_clones() {
    // E0507: Foo { field: self.graph.passes } when &self - nested field needs .clone()
    let source = r#"
pub struct PassDef { pub id: i32 }
pub struct Graph { pub passes: Vec<PassDef> }
pub struct Builder { pub graph: Graph }
pub struct Wrapper { pub passes: Vec<PassDef> }
impl Builder {
    pub fn wrap(self) -> Wrapper {
        Wrapper { passes: self.graph.passes }
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(rust_compiles(&rust), "Struct literal nested field must compile: {}", rust);
}

#[test]
fn test_let_binding_borrowed_field_clones() {
    // E0507: let mut new_passes = self.graph.passes when &self needs .clone()
    let source = r#"
pub struct PassDef { pub id: i32 }
pub struct Graph { pub passes: Vec<PassDef> }
pub struct Builder { pub graph: Graph }
impl Builder {
    pub fn add_pass(self, id: i32) -> Builder {
        let mut new_passes = self.graph.passes
        new_passes.push(PassDef { id })
        Builder { graph: Graph { passes: new_passes } }
    }
}
fn main() {}
"#;
    let rust = compile_to_rust(source).expect("compile");
    assert!(
        rust.contains("graph.passes.clone()") || rust.contains("graph.passes.clone()"),
        "Let binding from borrowed self.graph.passes needs .clone(): {}",
        rust
    );
    assert!(rust_compiles(&rust), "Let binding from borrowed field must compile: {}", rust);
}
