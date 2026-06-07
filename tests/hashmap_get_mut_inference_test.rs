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

/// TDD Tests: HashMap.get() should be inferred as get_mut() when the bound value
/// is used mutably (method calls that require &mut self).
///
/// Bug: `let skel_opt = self.skeletons.get(id)` generates `self.skeletons.get(&id)`
///      returning Option<&Skeleton>. Then `skel.update()` (requiring &mut self) fails
///      with E0596: cannot borrow `*skel` as mutable, as it is behind a `&` reference.
///
/// Fix: When the compiler detects the bound variable from .get() is used mutably,
///      it should emit .get_mut() instead.
#[path = "common/test_utils.rs"]
mod test_utils;
#[allow(unused_imports)]
use test_utils::compile_single;

/// When a variable bound from HashMap.get() is used mutably,
/// the compiler should emit get_mut() instead.
#[test]
fn test_hashmap_get_becomes_get_mut_when_value_mutated() {
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
    // Since skel.update() requires &mut self, the generated code should use get_mut
    assert!(
        output.contains("get_mut("),
        "HashMap.get() should become get_mut() when bound value is mutated.\nGenerated:\n{}",
        output
    );
}

/// When a variable bound from HashMap.get() is only read,
/// the compiler should keep get() as-is.
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
    // skel.bone_count() only reads, so get() should stay as get()
    assert!(
        output.contains(".get(") && !output.contains(".get_mut("),
        "HashMap.get() should stay get() when bound value is only read.\nGenerated:\n{}",
        output
    );
}
