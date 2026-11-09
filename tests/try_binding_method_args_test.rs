//! A binding made with `?` has the success type of the call: `let mut client = Client::connect(..)?` is a
//! `Client`, so `client.send(["SET", "a"])` sees the formal `Vec<string>` and the list literal is a `Vec`.
//! Independent oracle: `rustc` through `wj test` (the generated test crate must compile and the test pass).

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[test]
fn list_literal_argument_to_a_method_of_a_question_mark_binding_compiles_in_a_test_crate() {
    let d = Scratch::new("try-binding");
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::create_dir_all(d.join("tests")).unwrap();
    std::fs::write(d.join("wj.toml"), "[package]\nname = \"mini-client\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::write(d.join("src/lib.wj"), "pub mod client\n").unwrap();
    std::fs::write(
        d.join("src/client.wj"),
        "pub struct Client {\n    pub sent: int,\n}\n\nimpl Client {\n    pub fn connect(port: int) -> Result<Client, string> {\n        Ok(Client { sent: port - port })\n    }\n\n    pub fn send(self, args: Vec<string>) -> Result<(), string> {\n        self.sent = self.sent + args.len() as int\n        Ok(())\n    }\n}\n",
    )
    .unwrap();
    std::fs::write(
        d.join("tests/client_test.wj"),
        "use mini_client::client::Client\n\nfn run() -> Result<int, string> {\n    let mut client = Client::connect(80)?\n    client.send([\"SET\", \"a\", \"1\"])?\n    client.send([\"GET\", \"a\"])?\n    Ok(client.sent)\n}\n\n@test\nfn sends_lists() {\n    assert_eq!(run().unwrap_or(-1), 5)\n}\n",
    )
    .unwrap();
    let o = Command::new(WJ).arg("test").env("WJ_CAPS", "warn").current_dir(d.path()).output().unwrap();
    let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success() && out.contains("1 passed"), "{out}");
}
