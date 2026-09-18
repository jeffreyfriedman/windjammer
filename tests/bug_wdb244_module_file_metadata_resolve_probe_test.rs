#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Probe: after merge_external like library_multipass, Batch::sql_exec must stay demoted.

use std::collections::HashMap;
use std::path::PathBuf;
use windjammer::analyzer::SignatureRegistry;
use windjammer::codegen::rust::call_signature_resolution::resolve_method_for_call_site;
use windjammer::ir::emission_contract::callee_emits_shared_rust_ref_param;
use windjammer::ir::coercion::{compute_coercion, CoercionKind};
use windjammer::ir::safety_type::{
    BaseType, ConstEval, EffectSet, OwnedType, Region, SafetyType, TaintStatus,
};
use windjammer::ir::signature_bridge::safety_type_from_signature_param;
use windjammer::metadata::merge_external_crate_metadata_with_aliases;

#[test]
fn wdb244_merged_external_metadata_keeps_demoted_sql_exec() {
    let types_gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammerdb/crates/wdb-types/gen");
    assert!(
        types_gen.join("metadata.json").is_file(),
        "need wdb-types gen metadata"
    );
    let mut paths = HashMap::new();
    paths.insert("wdb_types".to_string(), types_gen);
    let mut registry = SignatureRegistry::new();
    merge_external_crate_metadata_with_aliases(&paths, &mut registry, None);
    let sig = registry
        .get_signature("ArrowColumnarBatch::sql_exec")
        .expect("ArrowColumnarBatch::sql_exec must load from external metadata");
    eprintln!(
        "loaded emitted={:?} ownership={:?} params={:?}",
        sig.emitted_rust_ref_params, sig.param_ownership, sig.param_types
    );
    assert!(
        callee_emits_shared_rust_ref_param(sig, 2),
        "left_table must be demoted shared-ref after metadata merge"
    );
    let resolved =
        resolve_method_for_call_site(&registry, None, "ArrowColumnarBatch", "sql_exec", 4)
            .expect("resolve");
    assert!(
        callee_emits_shared_rust_ref_param(&resolved.sig, 2),
        "resolve_method_for_call_site must keep demotion after finalize; emitted={:?}",
        resolved.sig.emitted_rust_ref_params
    );
    assert_ne!(
        compute_coercion(
            &SafetyType {
                base: BaseType::String,
                ownership: OwnedType::Ref(Region::fresh(0)),
                effects: EffectSet::pure(),
                taint: TaintStatus::Clean,
                const_eval: ConstEval::Runtime,
                exec_mode: None,
            },
            &safety_type_from_signature_param(&resolved.sig, resolved.sig.arg_param_index(1)),
        ),
        CoercionKind::ToOwnedString,
        "string lit → demoted &str must not ToOwnedString after resolve finalize"
    );
}

