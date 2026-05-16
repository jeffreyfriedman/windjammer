//! Statement-level JavaScript codegen for `JavaScriptGenerator`

use crate::parser::*;

use super::generator::JavaScriptGenerator;

impl JavaScriptGenerator {
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn pattern_to_js(&self, pattern: &crate::parser::Pattern) -> String {
        match pattern {
            crate::parser::Pattern::Wildcard => "_".to_string(),
            crate::parser::Pattern::Identifier(name) => name.clone(),
            crate::parser::Pattern::Reference(inner) => {
                // JavaScript doesn't have references, just use the inner pattern
                self.pattern_to_js(inner)
            }
            crate::parser::Pattern::Ref(name)
            | crate::parser::Pattern::RefMut(name)
            | crate::parser::Pattern::MutBinding(name) => name.clone(),
            crate::parser::Pattern::Tuple(patterns) => {
                let js_patterns: Vec<String> =
                    patterns.iter().map(|p| self.pattern_to_js(p)).collect();
                format!("[{}]", js_patterns.join(", "))
            }
            crate::parser::Pattern::EnumVariant(variant, binding) => {
                use crate::parser::EnumPatternBinding;
                // JavaScript doesn't have pattern matching like Rust, simplify
                match binding {
                    EnumPatternBinding::Single(bind) => bind.clone(),
                    EnumPatternBinding::Wildcard | EnumPatternBinding::None => variant.clone(),
                    _ => variant.clone(), // Tuple and Struct patterns also simplified to variant name
                }
            }
            crate::parser::Pattern::Literal(lit) => {
                // This is unusual for a for loop pattern, but handle it
                format!("{:?}", lit)
            }
            crate::parser::Pattern::Or(_) => {
                // Or patterns don't work in for loops, use wildcard
                "_".to_string()
            }
        }
    }

    pub(crate) fn generate_statement(&mut self, stmt: &Statement) -> String {
        let mut output = String::new();

        match stmt {
            Statement::Let {
                pattern,
                value,
                mutable: _,
                ..
            } => {
                // Always use `let` in JS: Windjammer enforces immutability at compile time.
                // Handle shadowing by renaming variables (JS can't re-declare let in same scope).
                let val_str = self.generate_expression(value);
                output.push_str(&self.indent());
                output.push_str("let ");
                // For simple identifier patterns, use the shadow-safe name
                if let Pattern::Identifier(name) = pattern {
                    let js_name = self.declare_var(name);
                    output.push_str(&js_name);
                } else {
                    output.push_str(&self.generate_pattern(pattern));
                }
                output.push_str(" = ");
                output.push_str(&val_str);
                output.push_str(";\n");
            }

            Statement::Const { name, value, .. } => {
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "const {} = {};\n",
                    name,
                    self.generate_expression(value)
                ));
            }

            Statement::Assignment {
                target,
                value,
                compound_op,
                ..
            } => {
                output.push_str(&self.indent());
                let op_str = match compound_op {
                    Some(CompoundOp::Add) => "+=",
                    Some(CompoundOp::Sub) => "-=",
                    Some(CompoundOp::Mul) => "*=",
                    Some(CompoundOp::Div) => "/=",
                    Some(CompoundOp::Mod) => "%=",
                    Some(CompoundOp::BitAnd) => "&=",
                    Some(CompoundOp::BitOr) => "|=",
                    Some(CompoundOp::BitXor) => "^=",
                    Some(CompoundOp::Shl) => "<<=",
                    Some(CompoundOp::Shr) => ">>=",
                    None => "=",
                };
                output.push_str(&format!(
                    "{} {} {};\n",
                    self.generate_expression(target),
                    op_str,
                    self.generate_expression(value)
                ));
            }

            Statement::Return { value: expr, .. } => {
                output.push_str(&self.indent());
                output.push_str("return");
                if let Some(e) = expr {
                    output.push(' ');
                    output.push_str(&self.generate_expression(e));
                }
                output.push_str(";\n");
            }

            Statement::Expression { expr, .. } => {
                output.push_str(&self.indent());
                output.push_str(&self.generate_expression(expr));
                output.push_str(";\n");
            }

            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                output.push_str(&self.indent());
                output.push_str("if (");
                output.push_str(&self.generate_expression(condition));
                output.push_str(") {\n");

                self.indent_level += 1;
                for s in then_block {
                    output.push_str(&self.generate_statement(s));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push('}');

                if let Some(else_stmts) = else_block {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    for s in else_stmts {
                        output.push_str(&self.generate_statement(s));
                    }
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }
                output.push('\n');
            }

            Statement::While {
                condition, body, ..
            } => {
                output.push_str(&self.indent());
                output.push_str("while (");
                output.push_str(&self.generate_expression(condition));
                output.push_str(") {\n");

                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.generate_statement(s));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
            }

            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "for (const {} of {}) {{\n",
                    self.pattern_to_js(pattern),
                    self.generate_expression(iterable)
                ));

                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.generate_statement(s));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
            }

            Statement::Loop { body, .. } => {
                output.push_str(&self.indent());
                output.push_str("while (true) {\n");

                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.generate_statement(s));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
            }

            Statement::Match { value, arms, .. } => {
                // Translate match to if-else chain
                output.push_str(&self.indent());
                output.push_str("{\n");

                self.indent_level += 1;
                output.push_str(&self.indent());
                output.push_str(&format!(
                    "let __match_value = {};\n",
                    self.generate_expression(value)
                ));

                // TDD FIX: Pre-declare variables used in patterns (including enum variants)
                // JavaScript requires variables to be declared before assignment in expressions
                // Deduplicate: multiple arms may bind the same variable name.
                {
                    use crate::parser::EnumPatternBinding;
                    let mut declared = std::collections::HashSet::new();
                    for arm in arms.iter() {
                        match &arm.pattern {
                            Pattern::Identifier(id) if id != "_" && declared.insert(id.clone()) => {
                                output.push_str(&self.indent());
                                output.push_str(&format!("let {};\n", id));
                            }
                            Pattern::EnumVariant(_, binding) => {
                                // TDD FIX: Extract variables from enum variant patterns
                                match binding {
                                    EnumPatternBinding::Single(var)
                                        if declared.insert(var.clone()) =>
                                    {
                                        output.push_str(&self.indent());
                                        output.push_str(&format!("let {};\n", var));
                                    }
                                    EnumPatternBinding::Tuple(patterns) => {
                                        for pat in patterns {
                                            if let Pattern::Identifier(var) = pat {
                                                if declared.insert(var.clone()) {
                                                    output.push_str(&self.indent());
                                                    output.push_str(&format!("let {};\n", var));
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }

                for (i, arm) in arms.iter().enumerate() {
                    output.push_str(&self.indent());
                    if i == 0 {
                        output.push_str("if (");
                    } else {
                        output.push_str("else if (");
                    }
                    output.push_str(&self.generate_pattern_match(&arm.pattern, "__match_value"));
                    if let Some(guard) = arm.guard {
                        output.push_str(" && ");
                        output.push_str(&self.generate_expression(guard));
                    }
                    output.push_str(") {\n");

                    self.indent_level += 1;
                    output.push_str(&self.indent());
                    // TDD FIX: Statement::Match should NOT return from function
                    // This is a statement-level match (like if-let), just execute the body
                    output.push_str(&format!("{};\n", self.generate_expression(arm.body)));
                    self.indent_level -= 1;

                    output.push_str(&self.indent());
                    output.push_str("}\n");
                }

                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push_str("}\n");
            }

            Statement::Thread { body, .. } => {
                // Thread in JS = Web Worker or setTimeout (for demo)
                output.push_str(&self.indent());
                output.push_str("// Note: true threading requires Web Workers\n");
                output.push_str(&self.indent());
                output.push_str("setTimeout(() => {\n");

                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.generate_statement(s));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}, 0);\n");
            }

            Statement::Async { body, .. } => {
                // Async in JS = Promise
                output.push_str(&self.indent());
                output.push_str("Promise.resolve().then(async () => {\n");

                self.indent_level += 1;
                for s in body {
                    output.push_str(&self.generate_statement(s));
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("});\n");
            }

            Statement::Break { .. } => {
                output.push_str(&self.indent());
                output.push_str("break;\n");
            }

            Statement::Continue { .. } => {
                output.push_str(&self.indent());
                output.push_str("continue;\n");
            }

            Statement::Use { path, alias, .. } => {
                output.push_str(&self.indent());
                // In JavaScript, we use import or require, but for local scope we can just skip it
                // since JavaScript modules work differently. For now, add a comment.
                output.push_str(&format!(
                    "// use {} {}\n",
                    path.join("."),
                    alias
                        .as_ref()
                        .map_or(String::new(), |a| format!("as {}", a))
                ));
            }

            Statement::Defer { .. } => {
                output.push_str(&self.indent());
                output.push_str("// TODO: Defer not yet supported in JavaScript\n");
            }

            Statement::Static { .. } => {
                // Handled at item level
            }
        }

        output
    }

    /// TDD: Generate match statement with returns (for tail position in functions)
    pub(crate) fn generate_statement_match_with_return(&mut self, stmt: &Statement) -> String {
        if let Statement::Match { value, arms, .. } = stmt {
            let mut output = String::new();
            output.push_str(&self.indent());
            output.push_str("{\n");

            self.indent_level += 1;
            output.push_str(&self.indent());
            output.push_str(&format!(
                "let __match_value = {};\n",
                self.generate_expression(value)
            ));

            // Pre-declare variables
            {
                use crate::parser::EnumPatternBinding;
                let mut declared = std::collections::HashSet::new();
                for arm in arms.iter() {
                    match &arm.pattern {
                        Pattern::Identifier(id) if id != "_" && declared.insert(id.clone()) => {
                            output.push_str(&self.indent());
                            output.push_str(&format!("let {};\n", id));
                        }
                        Pattern::EnumVariant(_, binding) => match binding {
                            EnumPatternBinding::Single(var) if declared.insert(var.clone()) => {
                                output.push_str(&self.indent());
                                output.push_str(&format!("let {};\n", var));
                            }
                            EnumPatternBinding::Tuple(patterns) => {
                                for pat in patterns {
                                    if let Pattern::Identifier(var) = pat {
                                        if declared.insert(var.clone()) {
                                            output.push_str(&self.indent());
                                            output.push_str(&format!("let {};\n", var));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }

            for (i, arm) in arms.iter().enumerate() {
                output.push_str(&self.indent());
                if i == 0 {
                    output.push_str("if (");
                } else {
                    output.push_str("else if (");
                }
                output.push_str(&self.generate_pattern_match(&arm.pattern, "__match_value"));
                if let Some(guard) = arm.guard {
                    output.push_str(" && ");
                    output.push_str(&self.generate_expression(guard));
                }
                output.push_str(") {\n");

                self.indent_level += 1;
                output.push_str(&self.indent());
                // TDD: Tail position match - add return
                output.push_str(&format!("return {};\n", self.generate_expression(arm.body)));
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push_str("}\n");
            }

            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push_str("}\n");
            output
        } else {
            String::new()
        }
    }

    pub(crate) fn generate_pattern_match(&self, pattern: &Pattern, match_value: &str) -> String {
        match pattern {
            Pattern::Wildcard => "true".to_string(),
            Pattern::Identifier(id) => {
                // Bind the variable: `((id = __match_value) !== undefined || true)` ensures
                // the variable is assigned AND the condition is always true (catch-all binding).
                // We use `var` here since `let` cannot be used inside an expression.
                format!("((({} = {}) !== undefined) || true)", id, match_value)
            }
            Pattern::Reference(inner) => {
                // JavaScript doesn't have references, just match the inner pattern
                self.generate_pattern_match(inner, match_value)
            }
            Pattern::Ref(name) | Pattern::RefMut(name) | Pattern::MutBinding(name) => {
                format!("((({} = {}) !== undefined) || true)", name, match_value)
            }
            Pattern::Literal(lit) => format!("{} === {}", match_value, self.generate_literal(lit)),
            Pattern::EnumVariant(name, binding) => {
                use crate::parser::EnumPatternBinding;
                // Convert :: to . for JS: Color::Red → Color.Red
                let js_name = name.replace("::", ".");
                match binding {
                    EnumPatternBinding::Wildcard | EnumPatternBinding::None => {
                        format!("{} === {}", match_value, js_name)
                    }
                    EnumPatternBinding::Single(var) => {
                        // Enum with data: extract the inner value
                        format!(
                            "({}.type === '{}' && (({} = {}.value) !== undefined || true))",
                            match_value, js_name, var, match_value
                        )
                    }
                    EnumPatternBinding::Tuple(patterns) => {
                        let mut conditions =
                            vec![format!("{}.type === '{}'", match_value, js_name)];
                        for (i, pat) in patterns.iter().enumerate() {
                            if let Pattern::Identifier(bind) = pat {
                                conditions.push(format!(
                                    "(({} = {}.value[{}]) !== undefined || true)",
                                    bind, match_value, i
                                ));
                            }
                        }
                        format!("({})", conditions.join(" && "))
                    }
                    _ => format!("{} === {}", match_value, js_name),
                }
            }
            Pattern::Or(patterns) => {
                let conditions: Vec<String> = patterns
                    .iter()
                    .map(|p| self.generate_pattern_match(p, match_value))
                    .collect();
                format!("({})", conditions.join(" || "))
            }
            Pattern::Tuple(_) => format!("Array.isArray({})", match_value),
        }
    }

    pub(crate) fn generate_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> =
                    patterns.iter().map(|p| self.generate_pattern(p)).collect();
                format!("[{}]", pattern_strs.join(", "))
            }
            Pattern::Reference(inner) => self.generate_pattern(inner), // JS doesn't have references
            Pattern::Ref(name) | Pattern::RefMut(name) | Pattern::MutBinding(name) => name.clone(),
            Pattern::EnumVariant(name, _) => name.clone(), // Simplified for JS
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Or(_) => "_".to_string(), // Simplified for JS
        }
    }
}
