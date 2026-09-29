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
    feature = "integration_tests",
))]

//! P3.532 isolate: free `dispatch(req: ServerRequest, …)` must stay owned when a
//! same-crate method `App::dispatch(&mut self, …)` shares the bare name.
//!
//! Product `wj-auth-api` false-GREEN isolates omitted the method homonym; multipass
//! then applied the method's MutBorrowed self slot to the free formal (`&mut ServerRequest`
//! + `dispatch(&mut req)` → E0596). Notes-api stays green because NotesApp::dispatch
//! is owned `mut self`, so the collided slot is Owned.

use std::process::Command;
use tempfile::TempDir;

#[test]
fn free_dispatch_must_not_inherit_method_mut_self() {
    let dir = TempDir::new().expect("temp");
    let src = dir.path().join("src");
    std::fs::create_dir_all(src.join("adapters")).unwrap();
    std::fs::create_dir_all(src.join("domain")).unwrap();

    std::fs::write(src.join("mod.wj"), "pub mod adapters\npub mod domain\n").unwrap();
    std::fs::write(src.join("adapters").join("mod.wj"), "pub mod http_server\n").unwrap();
    std::fs::write(src.join("domain").join("mod.wj"), "pub mod app\n").unwrap();

    std::fs::write(
        src.join("domain").join("app.wj"),
        r#"pub struct App {
    pub hits: int,
}

pub struct Reply {
    pub body: string,
}

impl App {
    pub fn new() -> App {
        App { hits: 0 }
    }

    // Mutating method shares the bare name `dispatch` with the free adapter fn.
    fn dispatch(self, path: string, body: string) -> Reply {
        self.hits = self.hits + 1
        Reply { body: "${path}:${body}" }
    }

    pub fn handle(self, path: string, body: string) -> Reply {
        self.dispatch(path, body)
    }
}
"#,
    )
    .unwrap();

    std::fs::write(
        src.join("adapters").join("http_server.wj"),
        r#"use std::sync::{Arc, Mutex}

use crate::domain::App
use crate::domain::Reply

struct ServerRequest {
    path: string,
    body: string,
    method: string,
}

struct ServerResponse {
    body: string,
}

fn dispatch(req: ServerRequest, state: Arc<Mutex<App>>) -> ServerResponse {
    let path = req.path
    let body = req.body
    match state.lock() {
        Ok(mut app) => {
            let reply = app.handle(path, body)
            ServerResponse { body: "${req.method}:${reply.body}" }
        },
        Err(_) => ServerResponse { body: "err" },
    }
}

pub fn serve_once(req: ServerRequest, shared: Arc<Mutex<App>>) -> ServerResponse {
    dispatch(req, shared)
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let http = std::fs::read_to_string(out.join("adapters").join("http_server.rs"))
        .expect("http_server.rs");
    assert!(
        !http.contains("fn dispatch(req: &mut "),
        "P3.532 RED: free dispatch inherited method MutBorrowed self:\n{http}"
    );
    assert!(
        !http.contains("dispatch(&mut req"),
        "P3.532 RED: call site passed &mut req from collided method sig:\n{http}"
    );
    assert!(
        http.contains("fn dispatch(req: ServerRequest")
            || http.contains("fn dispatch(req: crate::")
            || http.lines().any(|l| {
                l.contains("fn dispatch(req:") && !l.contains("&mut") && !l.contains("& ")
            }),
        "P3.532 RED: expected owned ServerRequest formal:\n{http}"
    );
}
