#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use windjammer::codegen::rust::call_signature_resolution::{
    effective_param_ownership, effective_param_ownership_for_arg,
};
use windjammer::codegen::rust::signature_promotion::param_type_is_owned_non_text;
use windjammer::analyzer::{Analyzer, FunctionSignature, OwnershipMode};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::parser::Type;

#[test]
fn test_mannequin_generate_formal_owned_call_site_owned() {
    // Copy struct: Phase 3 skips Reference wrap — param_types stays bare like formal.
    let sig = FunctionSignature {
        name: "MannequinMesh::generate".into(),
        param_types: vec![Type::Custom("MannequinConfig".into())],
        formal_param_types: vec![Type::Custom("MannequinConfig".into())],
        param_ownership: vec![OwnershipMode::Borrowed],
        return_type: Some(Type::Custom("MannequinMesh".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
    };

    assert!(
        param_type_is_owned_non_text(&sig, 0),
        "formal Custom(MannequinConfig) is owned non-text"
    );
    assert_eq!(
        effective_param_ownership_for_arg(&sig, 0),
        OwnershipMode::Owned,
        "call site must pass owned config despite body-inferred Borrowed"
    );
}

#[test]
fn test_quest_id_converged_formal_bare_call_site_borrowed() {
    let sig = FunctionSignature {
        name: "QuestManager::is_quest_active".into(),
        param_types: vec![
            Type::Custom("Self".into()),
            Type::Reference(Box::new(Type::Custom("QuestId".into()))),
        ],
        formal_param_types: vec![
            Type::Custom("Self".into()),
            Type::Custom("QuestId".into()),
        ],
        param_ownership: vec![OwnershipMode::Borrowed, OwnershipMode::Borrowed],
        return_type: Some(Type::Custom("Bool".into())),
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: true,
        is_extern: false,
    };

    assert!(
        param_type_is_owned_non_text(&sig, 1),
        "bare formal QuestId is owned non-text at formal layer"
    );
    assert!(
        matches!(
            sig.param_types.get(1),
            Some(Type::Reference(_))
        ),
        "call-site param_types should carry Reference wrap"
    );
    assert_eq!(
        effective_param_ownership(&sig, 1),
        OwnershipMode::Borrowed,
        "converged borrow must pass &QuestId at call site"
    );
}

#[test]
fn build_signature_preserves_formal_when_param_types_get_reference_wrap() {
    let source = r#"
pub struct MannequinConfig { pub torso_height: f32 }

impl MannequinMesh {
    pub fn generate(config: MannequinConfig) -> MannequinMesh {
        let mut mesh = MannequinMesh { tag: 0 }
        mesh.build_skeleton(config)
        mesh.build_body(config)
        mesh
    }

    fn build_skeleton(self, config: MannequinConfig) {
        let _ = config.torso_height
    }

    fn build_body(self, config: MannequinConfig) {
        let _ = config.torso_height
    }
}

pub struct MannequinMesh { tag: i32 }
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse");
    let mut analyzer = Analyzer::new();
    let (_, registry, _) = analyzer.analyze_program(&program).expect("analyze");

    let sig = registry
        .get_signature("MannequinMesh::generate")
        .expect("generate signature");

    assert_eq!(
        sig.formal_param_type(0),
        Some(&Type::Custom("MannequinConfig".into())),
        "formal stays bare Custom; formal={:?} param_types={:?}",
        sig.formal_param_types,
        sig.param_types
    );
    assert_eq!(
        sig.formal_param_types,
        sig.param_types,
        "Copy params skip Phase 3 Reference wrap — formal and param_types match"
    );
    assert_eq!(
        effective_param_ownership_for_arg(sig, 0),
        OwnershipMode::Owned,
        "call site uses formal owned type"
    );
}

#[test]
fn formal_param_type_falls_back_to_param_types_when_empty() {
    let sig = FunctionSignature {
        name: "legacy::fn".into(),
        param_types: vec![Type::Custom("QuestId".into())],
        formal_param_types: vec![],
        param_ownership: vec![OwnershipMode::Owned],
        return_type: None,
        return_ownership: OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
    };
    assert_eq!(
        sig.formal_param_type(0),
        Some(&Type::Custom("QuestId".into()))
    );
}

#[test]
fn metadata_import_populates_formal_param_types_from_params() {
    use windjammer::metadata::{
        try_analyzer_signature_from_metadata, FunctionSignature as MetaFunctionSignature,
    };

    let meta = MetaFunctionSignature {
        params: vec![
            "Custom(\"Self\")".into(),
            "Custom(\"QuestId\")".into(),
        ],
        formal_params: vec![],
        return_type: Some("Custom(\"Bool\")".into()),
        is_associated: true,
        parent_type: Some("QuestManager".into()),
        param_ownership: vec!["Borrowed".into(), "Borrowed".into()],
        has_self_receiver: true,
        is_extern: false,
    };

    let sig = try_analyzer_signature_from_metadata("QuestManager::is_quest_active", &meta)
        .expect("metadata import");
    assert_eq!(
        sig.formal_param_types,
        sig.param_types,
        "legacy metadata without formal_params mirrors params"
    );
}
