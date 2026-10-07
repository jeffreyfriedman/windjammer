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
    feature = "codegen_tests",
))]

//! WDB-460: Copy `Vec3` into **method** owned formal must not `.clone()`.
//!
//! Product tip-out:
//! - `math/quat.rs` `rotate_vec3`: `qv.cross(vec.clone())` / `qv.cross(uv.clone())`
//! - `math/mat4.rs` `look_at`: `s.cross(f.clone())` / `s.dot(eye.clone())`
//! - `audio_3d/spatial.rs`: `is_audible(listener_pos.clone())`
//! WJ uses bare `vec` / `uv` / `f` / `eye` / `listener_pos`.
//!
//! Distinct from WDB-355 (Copy Vec3 into **free-fn** owned formal), WDB-458
//! (Copy **newtype** into method formal), WDB-459 (Copy struct into Vec::push).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub fn rotate_vec3(self, vec: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z)
        let uv = qv.cross(vec)
        let uuv = qv.cross(uv)
        uv
    }
}

pub fn look_at_helper(eye: Vec3, f: Vec3, s: Vec3) -> f32 {
    let u = s.cross(f)
    s.dot(eye) + u.x
}
"#;

#[test]
fn wdb460_module_file_copy_vec3_method_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-460 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-460 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("vec.clone()")
        || rs.contains("uv.clone()")
        || rs.contains("f.clone()")
        || rs.contains("eye.clone()");
    assert!(
        !bad,
        "WDB-460 RED: Copy Vec3 cloned into method owned formal:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb460_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn wdb460_tip_out_game_core_vec3_method_formal_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb460_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/math/quat.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/math/mat4.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/audio_3d/spatial.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/math/quat.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/math/mat4.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/audio_3d/spatial.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("cross(vec.clone())")
                    || t.contains("cross(uv.clone())")
                    || t.contains("cross(f.clone())")
                    || t.contains("dot(eye.clone())")
                    || t.contains("is_audible(listener_pos.clone())"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-460: quat/mat4/spatial product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-460 RED: tip/product Copy Vec3 into method owned formal clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
