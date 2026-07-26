#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

use windjammer::analyzer::Analyzer;
use windjammer::analyzer::OwnershipMode;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

#[test]
fn bump_param_calling_mut_self_method_is_mut_borrowed() {
    let source = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn increment(self) {
        self.value = self.value + 1
    }
}

pub fn bump(mut c: Counter) {
    c.increment()
}
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().expect("parse");
    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");

    let bump = analyzed.iter().find(|f| f.decl.name == "bump").expect("bump");
    let own = bump.inferred_ownership.get("c").copied();
    let mutated = bump.mutated_parameters.contains("c");

    let inc = registry.get_signature("Counter::increment");
    let inc_self = inc.and_then(|s| s.param_ownership.first().copied());

    let fc = windjammer::ir::constraint_gen::generate_constraints(bump, Some(&registry));
    let has_mutref = fc.constraints.iter().any(|c| {
        matches!(
            c,
            windjammer::ir::constraints::Constraint::OwnershipIs(
                _,
                windjammer::ir::safety_type::OwnedType::MutRef(_)
            )
        )
    });

    let generated = test_utils::compile_single(source);

    assert!(
        has_mutref,
        "IR constraints must include MutRef for c; own={:?} mutated={} inc_self={:?}\ngenerated:\n{}",
        own,
        mutated,
        inc_self,
        generated
    );
    assert!(
        generated.contains("bump(c: &mut Counter)") || generated.contains("bump(mut c: Counter)"),
        "codegen must emit mut receiver; own={:?} mutated={} inc_self={:?} has_mutref={}\nGenerated:\n{}",
        own,
        mutated,
        inc_self,
        has_mutref,
        generated
    );
}
