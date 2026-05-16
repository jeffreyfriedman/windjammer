//! Integration: `.vgraph.json` → Windjammer source · see `windjammer-game/VISUAL_SCRIPTING_DESIGN.md`.

use std::path::PathBuf;

use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::visual_script::{compile_vgraph_json_to_windjammer, VsDocument};

#[test]
fn fixture_add_chain_compiles_through_parser() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(root.join("tests/fixtures/visual_script/add_literals.vgraph.json"))
        .expect("read fixture");

    let wj = compile_vgraph_json_to_windjammer(&json).expect("lower");
    let mut lexer = Lexer::new(&wj);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().expect("Windjammer parser accepts lowered source");
}

#[test]
fn roundtrip_fixture_matches_doc_shape() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let json =
        std::fs::read_to_string(root.join("tests/fixtures/visual_script/add_literals.vgraph.json")).unwrap();
    let doc: VsDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(doc.format, "windjammer-vgraph");
    assert_eq!(doc.version, 1);
}

#[test]
fn pong_hybrid_fixture_parses_after_lower() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(
        root.join("../windjammer-game/examples/visual_scripts/pong_ai_hybrid.vgraph.json"),
    )
    .expect("read pong hybrid example (sibling windjammer-game repo)");
    let wj = compile_vgraph_json_to_windjammer(&json).expect("lower pong sample");
    assert!(wj.contains("multiply_float"));
    let mut lexer = Lexer::new(&wj);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().expect("parser ok");
}
