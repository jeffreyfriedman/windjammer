#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// Signature-gated get→get_mut upgrade: only map shared-get calls upgrade;
/// user types with a method named `get` that is NOT HashMap::get must stay `.get(`.
#[path = "common/test_utils.rs"]
mod test_utils;
#[allow(unused_imports)]
use test_utils::compile_single;

#[test]
fn test_non_map_get_named_method_stays_get_even_with_mutation() {
    let input = r#"
struct Box {
    n: i32,
}

impl Box {
    pub fn new() -> Box {
        Box { n: 0 }
    }

    pub fn get(self) -> i32 {
        self.n
    }

    pub fn set(self, v: i32) {
        self.n = v
    }
}

fn use_box(b: Box) {
    let x = b.get()
    let _ = x + 1
}
"#;
    let output = compile_single(input);
    assert!(
        !output.contains(".get_mut("),
        "non-map `.get()` must not become `.get_mut()`.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_holder_get_on_self_field_stays_get_not_get_mut() {
    let input = r#"
use std::collections::HashMap

struct Holder {
    x: i32,
}

impl Holder {
    pub fn new() -> Holder {
        Holder { x: 0 }
    }

    pub fn get(self) -> i32 {
        self.x
    }

    pub fn bump(self) {
        self.x = self.x + 1
    }
}

struct Container {
    items: HashMap<u32, Holder>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: HashMap::new() }
    }

    pub fn read_holder(self, id: u32) -> i32 {
        let h_opt = self.items.get(id)
        match h_opt {
            Some(h) => h.get(),
            None => 0,
        }
    }
}
"#;
    let output = compile_single(input);
    // HashMap path uses .get( for lookup; Holder::get() is a plain method call, not get_mut
    assert!(
        output.contains(".get("),
        "HashMap lookup should still use .get(\nGenerated:\n{}",
        output
    );
    assert!(
        !output.contains(".get_mut("),
        "read-only HashMap.get + Holder.get must not upgrade to get_mut.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_hashmap_get_mut_upgrade_still_works_for_mutable_binding() {
    let input = r#"
use std::collections::HashMap

struct Skeleton {
    bones: Vec<f32>,
}

impl Skeleton {
    pub fn new() -> Skeleton {
        Skeleton { bones: Vec::new() }
    }

    pub fn update(self) {
        self.bones.push(1.0)
    }
}

struct Renderer {
    skeletons: HashMap<u32, Skeleton>,
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer { skeletons: HashMap::new() }
    }

    pub fn animate(self, id: u32) {
        let skel_opt = self.skeletons.get(id)
        match skel_opt {
            Some(skel) => {
                skel.update()
            },
            None => {},
        }
    }
}
"#;
    let output = compile_single(input);
    assert!(
        output.contains("get_mut("),
        "HashMap shared-get with downstream mutation should upgrade to get_mut.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_hashmap_get_stays_get_when_value_only_read() {
    let input = r#"
use std::collections::HashMap

struct Skeleton {
    bones: Vec<f32>,
}

impl Skeleton {
    pub fn new() -> Skeleton {
        Skeleton { bones: Vec::new() }
    }

    pub fn bone_count(self) -> usize {
        self.bones.len()
    }
}

struct Renderer {
    skeletons: HashMap<u32, Skeleton>,
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer { skeletons: HashMap::new() }
    }

    pub fn count_bones(self, id: u32) -> usize {
        let skel_opt = self.skeletons.get(id)
        match skel_opt {
            Some(skel) => skel.bone_count(),
            None => 0,
        }
    }
}
"#;
    let output = compile_single(input);
    assert!(
        output.contains(".get(") && !output.contains(".get_mut("),
        "read-only HashMap.get should stay get().\nGenerated:\n{}",
        output
    );
}
