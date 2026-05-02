// TDD: Trait impl must not pick analyzed data from an inherent method with the same name.
// Dogfooding: VoxelGPURenderer had `set_lighting(&mut self, LightingConfig)` and
// `impl RenderPort for VoxelGPURenderer { fn set_lighting(..., LightingData) }`;
// codegen matched the first analyzed function and emitted E0053.

#[path = "test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn rustc_check_lib(rs: &str) -> (bool, String) {
    let temp_dir = TempDir::new().expect("temp dir");
    let path = temp_dir.path().join("lib.rs");
    fs::write(&path, rs).unwrap();
    let out = temp_dir.path().join("libtest.rlib");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            path.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--edition",
            "2021",
        ])
        .output()
        .expect("rustc");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_uses_trait_method_body_not_inherent_same_name() {
    let code = r#"
pub struct PayloadA {}
pub struct PayloadB {}
pub struct Worker {}

pub trait Port {
    fn deliver(p: PayloadA)
}

impl Worker {
    pub fn deliver(self, p: PayloadB) {
        let _x = 0
    }
}

impl Port for Worker {
    fn deliver(p: PayloadA) {
        let _y = 1
    }
}
"#;

    let (gen, ok) = test_utils::compile_single_check(code);
    assert!(ok, "wj build failed:\n{}", gen);

    let start = gen
        .find("impl Port for Worker")
        .expect("impl Port for Worker missing");
    let tail = &gen[start..];
    let next_impl = tail[1..]
        .find("\nimpl ")
        .map(|i| i + 1)
        .unwrap_or(tail.len());
    let port_block = &tail[..next_impl];
    assert!(
        port_block.contains("PayloadA"),
        "trait impl should name PayloadA. Block:\n{}",
        port_block
    );
    assert!(
        !port_block.contains("PayloadB"),
        "trait impl must not use inherent method's PayloadB. Block:\n{}",
        port_block
    );

    let (rust_ok, rerr) = rustc_check_lib(&gen);
    assert!(
        rust_ok,
        "generated Rust must compile (no E0053). stderr:\n{}",
        rerr
    );
    assert!(!rerr.contains("E0053"), "unexpected E0053:\n{}", rerr);
}
