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

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// TDD test: When a HashMap.get() result is destructured in a tuple match pattern
/// and the bound variable is mutated, the compiler should upgrade get → get_mut.
///
/// Bug: `let skel_opt = self.skeletons.get(id)` followed by
///      `match (clip_opt, skel_opt) { (Some(clip), Some(skel)) => { skel.update() } }`
/// generates `self.skeletons.get(...)` instead of `self.skeletons.get_mut(...)`.
///
/// This causes E0596: cannot borrow `*skel` as mutable, as it is behind a `&` reference.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_mut_upgrade_in_tuple_match() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_hashmap_get_mut_tuple_match");

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_source = r#"
struct Clip {
    duration: f32
}

struct Skeleton {
    bones: Vec<f32>
}

impl Skeleton {
    pub fn update(self) {
    }

    pub fn set_bone_position(self, idx: u32, x: f32) {
    }
}

struct Renderer {
    clips: HashMap<string, Clip>,
    skeletons: HashMap<string, Skeleton>,
    active_clip_id: string,
    active_skeleton_id: string
}

impl Renderer {
    pub fn animate(self) {
        let clip_opt = self.clips.get(self.active_clip_id)
        let skel_opt = self.skeletons.get(self.active_skeleton_id)
        match (clip_opt, skel_opt) {
            (Some(clip), Some(skel)) => {
                skel.set_bone_position(0, 1.0)
                skel.update()
            },
            _ => {},
        }
    }
}
"#;

    let wj_file = test_dir.join("tuple_match.wj");
    fs::write(&wj_file, wj_source).unwrap();

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(test_dir.join("out"))
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(test_dir.join("out").join("tuple_match.rs")).unwrap();

    // The skel_opt binding should use get_mut because skel is mutated downstream
    assert!(
        generated.contains("self.skeletons.get_mut("),
        "Expected self.skeletons.get_mut() for mutated binding in tuple match.\n\
         Generated code:\n{}",
        generated
    );

    // clip_opt should remain get() since clip is only read
    assert!(
        generated.contains("self.clips.get("),
        "Expected self.clips.get() for read-only binding.\nGenerated code:\n{}",
        generated
    );
}

/// Same bug but with if-let tuple destructuring instead of match
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_mut_upgrade_in_if_let_tuple() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_hashmap_get_mut_if_let_tuple");

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_source = r#"
struct Data {
    value: f32
}

impl Data {
    pub fn reset(self) {
        self.value = 0.0
    }
}

struct Container {
    items: HashMap<string, Data>,
    names: HashMap<string, string>,
    key: string
}

impl Container {
    pub fn process(self) {
        let item_opt = self.items.get(self.key)
        let name_opt = self.names.get(self.key)
        if let (Some(item), Some(name)) = (item_opt, name_opt) {
            item.reset()
        }
    }
}
"#;

    let wj_file = test_dir.join("if_let_tuple.wj");
    fs::write(&wj_file, wj_source).unwrap();

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(test_dir.join("out"))
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated = fs::read_to_string(test_dir.join("out").join("if_let_tuple.rs")).unwrap();

    // item_opt should use get_mut because item.reset() mutates
    assert!(
        generated.contains("self.items.get_mut("),
        "Expected self.items.get_mut() for mutated binding in if-let tuple.\n\
         Generated code:\n{}",
        generated
    );

    // name_opt should remain get() since name is only read
    assert!(
        generated.contains("self.names.get("),
        "Expected self.names.get() for read-only binding.\nGenerated code:\n{}",
        generated
    );
}
