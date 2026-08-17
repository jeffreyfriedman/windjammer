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

//! FAILING REPRO — multipass trait `append(event: EventDraft)` with a **multi-field**
//! owned struct must keep trait/impl formals owned (not `&EventDraft`) across an
//! env-style wrapper module that forwards to a seed impl.
//!
//! Isolated single-field `DomainEventWriter::append` gates can pass while dogfood
//! still hits E0053 when the draft has several `string` fields and the impl lives
//! behind an env dispatcher.
//!
//! Language-only; no product/repo names.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn trait_owned_multifield_event_append_env_wrapper_keeps_owned_formal() {
    let files = [
        (
            "mod.wj",
            r#"
pub mod event
pub mod writer
pub mod seed
pub mod env_writer

use env_writer::EnvEventWriter
use event::EventDraft
use writer::EventWriter

fn main() {
    let w = EnvEventWriter {}
    let draft = EventDraft {
        tenant_id: "t1" + "",
        event_type: "posted" + "",
        payload_json: "{}" + "",
        request_id: "r1" + "",
    }
    let _ = w.append(draft)
}
"#,
        ),
        (
            "event.wj",
            r#"
pub struct EventDraft {
    pub tenant_id: string,
    pub event_type: string,
    pub payload_json: string,
    pub request_id: string,
}
"#,
        ),
        (
            "writer.wj",
            r#"
use crate::event::EventDraft

trait EventWriter {
    fn append(self, event: EventDraft) -> Result<(), string>
}
"#,
        ),
        (
            "seed.wj",
            r#"
use crate::event::EventDraft
use crate::writer::EventWriter

pub struct SeedEventWriter {}

impl EventWriter for SeedEventWriter {
    fn append(self, event: EventDraft) -> Result<(), string> {
        let _ = event.tenant_id + ""
        let _ = event.event_type + ""
        let _ = event.payload_json + ""
        let _ = event.request_id + ""
        Ok(())
    }
}
"#,
        ),
        (
            "env_writer.wj",
            r#"
use crate::event::EventDraft
use crate::seed::SeedEventWriter
use crate::writer::EventWriter

pub struct EnvEventWriter {}

impl EventWriter for EnvEventWriter {
    fn append(self, event: EventDraft) -> Result<(), string> {
        let inner = SeedEventWriter {}
        inner.append(event)
    }
}
"#,
        ),
    ];

    let generated = test_utils::compile_project(&files);
    let seed = generated
        .get("seed.rs")
        .cloned()
        .unwrap_or_default();
    let env = generated
        .get("env_writer.rs")
        .cloned()
        .unwrap_or_default();
    let writer = generated
        .get("writer.rs")
        .cloned()
        .unwrap_or_default();
    let combined = format!("{writer}\n{seed}\n{env}");

    assert!(
        !combined.contains("event: &EventDraft")
            && !combined.contains("fn append(self, event: &EventDraft")
            && !combined.contains("fn append(&self, event: &EventDraft"),
        "impl must not flip multi-field EventDraft formal to &T across env wrapper. Got:\n{combined}"
    );
    assert!(
        combined.contains("event: EventDraft"),
        "expected owned EventDraft formal on trait/impl. Got:\n{combined}"
    );
}
