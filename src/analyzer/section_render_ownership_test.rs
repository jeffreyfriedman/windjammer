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
}
