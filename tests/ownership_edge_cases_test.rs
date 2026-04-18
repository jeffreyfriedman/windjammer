//! TDD: E0507 / E0596 / E0382 edge cases from dogfooding (library multipass + registry aliases).
//!
//! 1. **Copy registry in `build_library_multipass`**: Cross-file newtypes must be Copy in the
//!    analyzer so getters use `&self` (not `self`) — avoids use-after-move and move-out of `&T`.
//! 2. **Read-only method names vs `SignatureRegistry`**: Unqualified `len` from one type must not
//!    force `self.map.len()` to look like a mutating call — avoids spurious `&mut self` on counters.

use std::fs;
use tempfile::TempDir;
use windjammer::CompilationTarget;
use windjammer::compiler::build_project_ext;

#[test]
fn test_library_multipass_copy_newtype_getter_uses_shared_ref_self() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(src.join("ids")).unwrap();

    fs::write(
        src.join("ids/widget_id.wj"),
        r#"
pub struct WidgetId {
    value: u32
}

impl WidgetId {
    pub fn new(value: u32) -> WidgetId {
        WidgetId { value: value }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("ids/widget.wj"),
        r#"
use super::widget_id::WidgetId

pub struct Widget {
    id: WidgetId
}

impl Widget {
    pub fn id(self) -> WidgetId {
        self.id
    }
}
"#,
    )
    .unwrap();

    build_project_ext(
        &src,
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("library multipass build");

    let rs = fs::read_to_string(build.join("ids/widget.rs")).expect("widget.rs");
    assert!(
        rs.contains("fn id(&self)"),
        "WidgetId is Copy-shaped across files; id() should take &self. Got:\n{}",
        rs
    );
}

#[test]
fn test_len_registry_collision_does_not_force_mut_self_on_map_len() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let build = temp.path().join("build");
    fs::create_dir_all(src.join("demo")).unwrap();

    // Type A: `len` inferred / registered as needing &mut self (like some buffer wrappers).
    fs::write(
        src.join("demo/buffer.wj"),
        r#"
pub struct ByteBuffer {
    data: Vec<u8>
}

impl ByteBuffer {
    pub fn new() -> ByteBuffer {
        ByteBuffer { data: Vec::new() }
    }

    pub fn len(self) -> usize {
        self.data.len()
    }
}
"#,
    )
    .unwrap();

    // Type B: only calls `self.counts.len()` — must stay read-only / &self on `total_keys`.
    fs::write(
        src.join("demo/registry.wj"),
        r#"
use std::collections::HashMap

pub struct KeyRegistry {
    counts: HashMap<u32, u32>
}

impl KeyRegistry {
    pub fn new() -> KeyRegistry {
        KeyRegistry { counts: HashMap::new() }
    }

    pub fn total_keys(self) -> usize {
        self.counts.len()
    }
}
"#,
    )
    .unwrap();

    build_project_ext(
        &src,
        &build,
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("library multipass build");

    let rs = fs::read_to_string(build.join("demo/registry.rs")).expect("registry.rs");
    assert!(
        rs.contains("fn total_keys(&self)"),
        "total_keys only reads .len(); must not become &mut self due to another type's len(). Got:\n{}",
        rs
    );
}
