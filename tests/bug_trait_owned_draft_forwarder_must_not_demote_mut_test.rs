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

//! FAILING REPRO — thin trait-impl Draft forwarder must not demote to `&mut Draft`.
//!
//! Product cold gen (LedgerKit `env_channel_adapter` / `env_inventory_repository`):
//!   trait: `fn create(..., draft: Draft)`
//!   impl:  `fn create(..., draft: &mut Draft)` → E0053
//! Composition may also emit `deps: &Self` when Draft is demoted (E0411).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/trait_owned_draft_forwarder_must_not_demote_mut.wj");

fn assert_owned_draft_formals(rs: &str) {
    assert!(
        !rs.contains("draft: &mut Draft") && !rs.contains("draft: &mut Draft,"),
        "RED P3.263: trait/impl Draft formal must stay owned, not &mut. Generated:\n{rs}"
    );
    assert!(
        !rs.contains("deps: &Self"),
        "RED P3.263: free fn must keep AppDeps-shaped formal, not &Self. Generated:\n{rs}"
    );
}

#[test]
fn trait_owned_draft_forwarder_must_not_demote_mut() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_owned_draft_formals(&rs);
    assert!(
        ok,
        "RED P3.263: owned Draft trait forwarder must cargo-check. Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_trait_owned_draft_forwarder_must_not_demote_mut() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod ports
pub mod adapters
pub mod composition
"#,
    );
    project.add_file(
        "ports/mod.wj",
        r#"
pub mod draft_port
"#,
    );
    project.add_file(
        "ports/draft_port.wj",
        r#"
pub struct Draft {
    pub name: string,
}

pub struct View {
    pub name: string,
}

pub trait DraftPort {
    fn create(self, tenant_id: string, draft: Draft) -> Result<View, string>
}
"#,
    );
    project.add_file(
        "adapters/mod.wj",
        r#"
pub mod seed
pub mod env
"#,
    );
    project.add_file(
        "adapters/seed.wj",
        r#"
use crate::ports::draft_port::{Draft, DraftPort, View}

pub struct SeedPort {}

impl DraftPort for SeedPort {
    fn create(self, tenant_id: string, draft: Draft) -> Result<View, string> {
        let _ = tenant_id
        Ok(View {
            name: draft.name + "",
        })
    }
}
"#,
    );
    project.add_file(
        "adapters/env.wj",
        r#"
use crate::ports::draft_port::{Draft, DraftPort, View}
use crate::adapters::seed::SeedPort

pub struct EnvPort {}

impl DraftPort for EnvPort {
    fn create(self, tenant_id: string, draft: Draft) -> Result<View, string> {
        let port = SeedPort {}
        port.create(tenant_id, draft)
    }
}
"#,
    );
    project.add_file(
        "composition/mod.wj",
        r#"
pub mod use_cases
"#,
    );
    project.add_file(
        "composition/use_cases.wj",
        r#"
use crate::ports::draft_port::{Draft, DraftPort, View}
use crate::adapters::env::EnvPort

pub struct AppDeps {
    pub port: EnvPort,
}

pub fn create_item(deps: AppDeps, tenant_id: string, draft: Draft) -> Result<View, string> {
    deps.port.create(tenant_id, draft)
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal Draft forwarder compile should succeed");
    for (key, rs) in &map {
        if key.ends_with("env.rs") || key.ends_with("use_cases.rs") {
            assert_owned_draft_formals(rs);
        }
    }
    let env_key = map
        .keys()
        .find(|k| k.ends_with("env.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing env.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let env_rs = map.get(&env_key).expect("env.rs");
    assert!(
        env_rs.contains("draft: Draft") || env_rs.contains("draft: Draft,"),
        "env impl must keep owned Draft formal; got:\n{env_rs}"
    );
}
