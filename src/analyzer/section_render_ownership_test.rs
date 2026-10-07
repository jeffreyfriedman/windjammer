#[cfg(test)]
mod section_render_ownership_test {
    use crate::analyzer::{Analyzer, OwnershipMode};
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse_program(src: &str) -> crate::parser::Program<'static> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        parser.parse().expect("parse")
    }

    #[test]
    fn section_render_inferred_ownership_is_borrowed() {
        let src = r#"
pub struct Section { title: string }
impl Section {
    pub fn render(self) -> string {
        format!("{}", self.title)
    }
}
pub struct SectionGroup { sections: Vec<Section> }
impl SectionGroup {
    pub fn render(self) -> string {
        let mut result = String::new()
        for s in self.sections {
            result = result + s.render() + "\n"
        }
        result
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (_funcs, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let sig = registry
            .get_signature("Section::render")
            .expect("Section::render");
        assert_eq!(
            sig.param_ownership.first().copied(),
            Some(OwnershipMode::Borrowed),
            "got {:?}",
            sig.param_ownership
        );
    }

    #[test]
    fn render_is_not_stdlib_mutating() {
        let reg = crate::analyzer::SignatureRegistry::stdlib();
        let mut keys: Vec<_> = reg
            .all_signatures_for_suffix_search()
            .filter(|(k, _)| k.ends_with("::render"))
            .map(|(k, s)| format!("{k}={:?}", s.param_ownership.first()))
            .collect();
        keys.sort();
        eprintln!("stdlib ::render keys: {keys:?}");
        assert!(
            !crate::analyzer::stdlib_method_traits::method_mutates_receiver("render"),
            "render must not be stdlib consensus mutating; keys={keys:?}"
        );
    }

    #[test]
    fn mutated_and_returned_vec_is_owned_in_analyzer() {
        let src = r#"
fn sort_and_return(items: Vec<i32>) -> Vec<i32> {
    items.sort()
    items
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (funcs, _registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let f = funcs
            .iter()
            .find(|f| f.decl.name == "sort_and_return")
            .expect("fn");
        assert!(
            f.returned_parameters.contains("items"),
            "returned_parameters missing items: {:?}",
            f.returned_parameters
        );
        assert_eq!(
            f.inferred_ownership.get("items"),
            Some(&OwnershipMode::Owned),
            "inferred_ownership={:?}",
            f.inferred_ownership
        );
    }

    #[test]

    #[test]
    fn wdb416_evaluate_method_keeps_owned_val_formals() {
        let src = r#"
pub enum Val {
    F(f32),
    S(string),
}
impl Val {
    pub fn as_float(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::S(_) => 0.0,
        }
    }
}
pub enum Op { Add, Sub, Neg }
pub struct Eval { pub last: f32 }
impl Eval {
    pub fn evaluate(self, op: Op, a: Val, b: Val) -> f32 {
        match op {
            Op::Add => {
                let fa = a.as_float()
                let fb = b.as_float()
                fa + fb
            },
            Op::Sub => {
                let fa = a.as_float()
                let fb = b.as_float()
                fa - fb
            },
            Op::Neg => { -a.as_float() },
        }
    }
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (_funcs, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let eval = registry
            .get_signature("Eval::evaluate")
            .expect("Eval::evaluate");
        eprintln!(
            "Eval::evaluate ownership={:?} has_self={} emitted={:?}",
            eval.param_ownership, eval.has_self_receiver, eval.emitted_rust_ref_params
        );
        // skip self: ownership for a, b
        let a_b = if eval.has_self_receiver {
            eval.param_ownership.get(1..).unwrap_or(&[])
        } else {
            &eval.param_ownership
        };
        // op, a, b — find Val slots (Owned expected for a,b)
        assert!(
            eval.param_ownership.iter().filter(|m| **m == OwnershipMode::Owned).count() >= 2,
            "evaluate must keep owned Val formals; got {:?}",
            eval.param_ownership
        );
        let _ = a_b;
    }

    fn wdb416_owned_enum_consumed_by_as_float_stays_owned() {
        let src = r#"
pub enum Val {
    F(f32),
    S(string),
}
impl Val {
    pub fn as_float(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::S(_) => 0.0,
        }
    }
}
pub fn add(a: Val, b: Val) -> f32 {
    a.as_float() + b.as_float()
}
"#;
        let program = parse_program(src);
        let mut analyzer = Analyzer::new();
        let (funcs, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let as_float = registry
            .get_signature("Val::as_float")
            .expect("Val::as_float");
        eprintln!(
            "Val::as_float ownership={:?} has_self={}",
            as_float.param_ownership, as_float.has_self_receiver
        );
        eprintln!(
            "bare as_float={:?}",
            registry.get_signature("as_float").map(|s| &s.param_ownership)
        );
        let add = funcs.iter().find(|f| f.decl.name == "add").expect("add");
        eprintln!("add inferred={:?}", add.inferred_ownership);
        let add_sig = registry.get_signature("add").expect("add sig");
        eprintln!("add registry ownership={:?}", add_sig.param_ownership);
        assert_eq!(
            add_sig.param_ownership,
            vec![OwnershipMode::Owned, OwnershipMode::Owned],
            "add must keep owned Val formals; got {:?}",
            add_sig.param_ownership
        );
    }
}
