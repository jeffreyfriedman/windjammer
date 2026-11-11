//! A test crate calls a library method whose receiver the library infers as `&mut self` (the body
//! pushes onto a field). The caller's `let hub` must be emitted `let mut hub`: the receiver mode is a
//! published fact of the library (`metadata.json`), like its parameter modes.
//! Independent oracle: `rustc` through `wj test` (the generated test crate must compile and the test pass).

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[test]
fn binding_mutated_through_a_library_receiver_is_mut_in_a_test_crate() {
    let d = Scratch::new("mut-receiver");
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::create_dir_all(d.join("tests")).unwrap();
    std::fs::write(d.join("wj.toml"), "[package]\nname = \"line-hub\"\nversion = \"0.1.0\"\n\n[lib]\n").unwrap();
    std::fs::write(
        d.join("src/lib.wj"),
        "pub struct Hub {\n    pub log: Vec<string>,\n}\n\nimpl Hub {\n    pub fn new() -> Hub {\n        Hub { log: Vec::new() }\n    }\n\n    pub fn handle(self, line: string) -> int {\n        self.log.push(line)\n        self.log.len() as int\n    }\n}\n",
    )
    .unwrap();
    std::fs::write(
        d.join("tests/hub_test.wj"),
        "use crate::Hub\n\nfn test_handle_counts_lines() {\n    let mut hub = Hub::new()\n    let first = hub.handle(\"a\")\n    let second = hub.handle(\"b\")\n    assert_eq(first, 1)\n    assert_eq(second, 2)\n}\n",
    )
    .unwrap();
    let o = Command::new(WJ).arg("test").env("WJ_CAPS", "warn").current_dir(d.path()).output().unwrap();
    let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success() && out.contains("1 passed"), "{out}");
}
