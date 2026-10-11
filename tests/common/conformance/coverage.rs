//! Feature-coverage gate: which AST node kinds / std methods does the corpus exercise?
//!
//! Kinds are read from the parsed AST (Debug rendering of real nodes), never from source text.

use super::program::Program;
use std::collections::BTreeSet;
use std::path::Path;

pub fn parse_debug(source: &str) -> String {
    let mut lexer = windjammer::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = windjammer::parser::Parser::new(tokens);
    match parser.parse() {
        Ok(program) => format!("{:?}", program),
        Err(e) => panic!("corpus program does not parse: {e}"),
    }
}

/// CamelCase identifiers in the AST debug dump that are node kinds (followed by `{`, `(` or `,`).
pub fn node_kinds(debug: &str) -> BTreeSet<String> {
    let bytes = debug.as_bytes();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_uppercase() && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let rest = debug[i..].trim_start_matches(' ');
            if rest.starts_with('{') || rest.starts_with('(') || rest.starts_with(',') || rest.starts_with(']') {
                out.insert(debug[start..i].to_string());
            }
        } else {
            i += 1;
        }
    }
    out
}

/// Method names invoked (`method: "name"`) and macros (`name: "vec"`) in the dump.
pub fn used_methods(debug: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for key in ["method: \"", "name: \""] {
        let mut rest = debug;
        while let Some(p) = rest.find(key) {
            rest = &rest[p + key.len()..];
            if let Some(e) = rest.find('"') {
                out.insert(rest[..e].to_string());
            }
        }
    }
    out
}

pub struct Coverage {
    pub kinds: BTreeSet<String>,
    pub methods: BTreeSet<String>,
}

pub fn collect(programs: &[Program]) -> Coverage {
    let mut cov = Coverage { kinds: BTreeSet::new(), methods: BTreeSet::new() };
    for p in programs {
        let d = parse_debug(&p.source_for(p.seeds[0]));
        cov.kinds.extend(node_kinds(&d));
        cov.methods.extend(used_methods(&d));
    }
    cov
}

/// Required features from FEATURES.toml (`kinds = [...]`, `methods = [...]`) not exercised.
pub fn unexercised(cov: &Coverage, features_toml: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(features_toml).expect("FEATURES.toml");
    let v: toml::Value = text.parse().expect("valid FEATURES.toml");
    let list = |k: &str| -> Vec<String> {
        v.get(k).and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
    };
    let mut missing = Vec::new();
    for k in list("kinds").into_iter().filter(|k| !cov.kinds.contains(k)) {
        missing.push(format!("AST kind `{k}`"));
    }
    for m in list("methods").into_iter().filter(|m| !cov.methods.contains(m)) {
        missing.push(format!("method/macro `{m}`"));
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_come_from_ast() {
        let d = parse_debug("fn main() { let x = 1 + 2 }");
        let k = node_kinds(&d);
        assert!(k.contains("Binary"), "{k:?}");
        assert!(k.contains("Let"), "{k:?}");
    }
}
