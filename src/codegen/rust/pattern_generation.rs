//! Pattern code generation
//!
//! Handles code generation for patterns in let bindings, match arms, and for loops including:
//! - Wildcard patterns
//! - Identifier patterns
//! - Mutable bindings
//! - Reference patterns
//! - Tuple patterns
//! - Struct patterns
//! - Enum patterns
//! - Or patterns

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::MutBinding(name) => format!("mut {}", name),
            Pattern::Reference(inner) => format!("&{}", self.pattern_to_rust(inner)),
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
            Pattern::Tuple(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                format!("({})", rust_patterns.join(", "))
            }
            Pattern::EnumVariant(variant, binding) => match binding {
                EnumPatternBinding::Single(name) => format!("{}({})", variant, name),
                EnumPatternBinding::Wildcard => format!("{}(_)", variant),
                EnumPatternBinding::None => variant.clone(),
                EnumPatternBinding::Tuple(patterns) => {
                    let rust_patterns: Vec<String> =
                        patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                    format!("{}({})", variant, rust_patterns.join(", "))
                }
                EnumPatternBinding::Struct(fields, has_wildcard) => {
                    if fields.is_empty() {
                        format!("{} {{ .. }}", variant)
                    } else {
                        let field_strs: Vec<String> = fields
                            .iter()
                            .map(|(name, pat)| format!("{}: {}", name, self.pattern_to_rust(pat)))
                            .collect();
                        if *has_wildcard {
                            format!("{} {{ {}, .. }}", variant, field_strs.join(", "))
                        } else {
                            format!("{} {{ {} }}", variant, field_strs.join(", "))
                        }
                    }
                }
            },
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Or(patterns) => {
                let rust_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_rust(p)).collect();
                rust_patterns.join(" | ")
            }
        }
    }

    pub(crate) fn generate_pattern(&self, pattern: &Pattern) -> String {
        use crate::parser::EnumPatternBinding;
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::MutBinding(name) => format!("mut {}", name),
            Pattern::Reference(inner) => format!("&{}", self.generate_pattern(inner)),
            Pattern::Ref(name) => format!("ref {}", name),
            Pattern::RefMut(name) => format!("ref mut {}", name),
            Pattern::EnumVariant(name, binding) => match binding {
                EnumPatternBinding::Single(b) => format!("{}({})", name, b),
                EnumPatternBinding::Wildcard => format!("{}(_)", name),
                EnumPatternBinding::None => name.clone(),
                EnumPatternBinding::Tuple(patterns) => {
                    let rust_patterns: Vec<String> =
                        patterns.iter().map(|p| self.generate_pattern(p)).collect();
                    format!("{}({})", name, rust_patterns.join(", "))
                }
                EnumPatternBinding::Struct(fields, has_wildcard) => {
                    if fields.is_empty() {
                        format!("{} {{ .. }}", name)
                    } else {
                        let field_strs: Vec<String> = fields
                            .iter()
                            .map(|(n, pat)| {
                                if let Pattern::Identifier(binding) = pat {
                                    if binding == n {
                                        return n.clone();
                                    }
                                }
                                format!("{}: {}", n, self.generate_pattern(pat))
                            })
                            .collect();
                        if *has_wildcard {
                            format!("{} {{ {}, .. }}", name, field_strs.join(", "))
                        } else {
                            format!("{} {{ {} }}", name, field_strs.join(", "))
                        }
                    }
                }
            },
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                format!("({})", pattern_strs.join(", "))
            }
            Pattern::Or(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                pattern_strs.join(" | ")
            }
        }
    }

}
