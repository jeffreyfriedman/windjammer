//! Expression-level JavaScript codegen for `JavaScriptGenerator`

use crate::parser::*;

use super::generator::JavaScriptGenerator;
use super::type_conversion::{binary_op_to_js, escape_js_keyword, unary_op_to_js};

impl JavaScriptGenerator {
    pub(crate) fn generate_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::Literal { value: lit, .. } => self.generate_literal(lit),

            Expression::Identifier { name: id, .. } => {
                // Convert Rust-style path syntax (Type::Variant) to JS dot notation (Type.Variant)
                if id.contains("::") {
                    // TDD FIX: Escape keywords in path components (Type::default → Type.default_)
                    let parts: Vec<String> = id.split("::").map(escape_js_keyword).collect();
                    parts.join(".")
                } else {
                    // TDD FIX: Escape JavaScript keywords
                    // Resolve shadowed variable names first, then escape if needed
                    let resolved = self.resolve_var(id);
                    escape_js_keyword(&resolved)
                }
            }

            Expression::Binary {
                left, op, right, ..
            } => {
                format!(
                    "({} {} {})",
                    self.generate_expression(left),
                    binary_op_to_js(op),
                    self.generate_expression(right)
                )
            }

            Expression::Unary { op, operand, .. } => {
                format!(
                    "({}{})",
                    unary_op_to_js(op),
                    self.generate_expression(operand)
                )
            }

            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let func_expr = self.generate_expression(function);

                // Handle special functions
                match func_expr.as_str() {
                    "println!" | "println" => {
                        let args: Vec<String> = arguments
                            .iter()
                            .map(|(_, arg)| self.generate_expression(arg))
                            .collect();
                        return self.generate_js_println(&args);
                    }
                    "print!" | "print" => {
                        let args: Vec<String> = arguments
                            .iter()
                            .map(|(_, arg)| self.generate_expression(arg))
                            .collect();
                        return self.generate_js_print(&args);
                    }
                    _ => {}
                }

                let func_expr = if func_expr.ends_with(".new") {
                    format!("{}.create", &func_expr[..func_expr.len() - 4])
                } else if func_expr.ends_with(".new_") {
                    format!("{}.create", &func_expr[..func_expr.len() - 5])
                } else {
                    func_expr
                };

                let args: Vec<String> = arguments
                    .iter()
                    .map(|(_, arg)| self.generate_expression(arg))
                    .collect();
                format!("{}({})", func_expr, args.join(", "))
            }

            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let obj = self.generate_expression(object);
                let args: Vec<String> = arguments
                    .iter()
                    .map(|(_, arg)| self.generate_expression(arg))
                    .collect();
                // Map Windjammer/Rust methods to JS equivalents
                match method.as_str() {
                    "len" => format!("{}.length", obj), // .len() → .length (property, not method)
                    "is_empty" => format!("{}.length === 0", obj),
                    "contains" if args.len() == 1 => format!("{}.includes({})", obj, args[0]),
                    "to_string" => format!("String({})", obj),
                    _ => format!("{}.{}({})", obj, method, args.join(", ")),
                }
            }

            Expression::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.generate_expression(object), field)
            }

            Expression::StructLiteral { name, fields, .. } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(field_name, expr)| {
                        format!("{}: {}", field_name, self.generate_expression(expr))
                    })
                    .collect();
                format!(
                    "new {}({})",
                    name,
                    field_strs
                        .iter()
                        .map(|f| { f.split(": ").nth(1).unwrap_or("") })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }

            Expression::MapLiteral { pairs, .. } => {
                // Generate JavaScript object literal: {key: value, ...}
                if pairs.is_empty() {
                    "{}".to_string()
                } else {
                    let entries_str: Vec<String> = pairs
                        .iter()
                        .map(|(k, v)| {
                            let key_str = self.generate_expression(k);
                            let val_str = self.generate_expression(v);
                            // If key is a simple identifier or string literal, use it directly
                            // Otherwise, use computed property: [key]: value
                            if matches!(
                                k,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } | Expression::Identifier { .. }
                            ) {
                                format!("{}: {}", key_str, val_str)
                            } else {
                                format!("[{}]: {}", key_str, val_str)
                            }
                        })
                        .collect();
                    format!("{{ {} }}", entries_str.join(", "))
                }
            }

            Expression::Index { object, index, .. } => {
                format!(
                    "{}[{}]",
                    self.generate_expression(object),
                    self.generate_expression(index)
                )
            }

            Expression::Tuple { elements, .. } => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| self.generate_expression(e))
                    .collect();
                format!("[{}]", elems.join(", "))
            }

            Expression::Array { elements, .. } => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| self.generate_expression(e))
                    .collect();
                format!("[{}]", elems.join(", "))
            }

            Expression::MacroInvocation { name, args, .. } => {
                // Handle common macros
                match name.as_str() {
                    "vec" => {
                        let elems: Vec<String> =
                            args.iter().map(|e| self.generate_expression(e)).collect();
                        format!("[{}]", elems.join(", "))
                    }
                    "println" | "print" => {
                        let args_str: Vec<String> =
                            args.iter().map(|e| self.generate_expression(e)).collect();
                        format!("console.log({})", args_str.join(", "))
                    }
                    "format" => {
                        // Convert format!("Hello, {}!", name) → `Hello, ${name}!`
                        let args_str: Vec<String> =
                            args.iter().map(|e| self.generate_expression(e)).collect();
                        if args_str.len() <= 1 {
                            args_str
                                .first()
                                .cloned()
                                .unwrap_or_else(|| "''".to_string())
                        } else {
                            // First arg is the format string (quoted), rest are values
                            let fmt = &args_str[0];
                            // Strip quotes from format string
                            let inner = if (fmt.starts_with('\'') && fmt.ends_with('\''))
                                || (fmt.starts_with('"') && fmt.ends_with('"'))
                            {
                                &fmt[1..fmt.len() - 1]
                            } else {
                                fmt.as_str()
                            };
                            // Replace {} placeholders with ${arg} expressions
                            let mut result = String::new();
                            let mut arg_idx = 0;
                            let mut chars = inner.chars().peekable();
                            while let Some(ch) = chars.next() {
                                if ch == '{' && chars.peek() == Some(&'}') {
                                    chars.next(); // consume }
                                    if arg_idx + 1 < args_str.len() {
                                        result.push_str(&format!("${{{}}}", args_str[arg_idx + 1]));
                                        arg_idx += 1;
                                    } else {
                                        result.push_str("{}");
                                    }
                                } else {
                                    result.push(ch);
                                }
                            }
                            format!("`{}`", result)
                        }
                    }
                    _ => format!("/* macro: {}! */", name),
                }
            }

            Expression::Await { expr, .. } => {
                format!("await {}", self.generate_expression(expr))
            }

            Expression::TryOp { expr, .. } => {
                // Simplify: just return the expression (proper error handling needs runtime support)
                self.generate_expression(expr)
            }

            Expression::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                // Generate array from range
                if *inclusive {
                    format!(
                        "Array.from({{length: {} - {} + 1}}, (_, i) => {} + i)",
                        self.generate_expression(end),
                        self.generate_expression(start),
                        self.generate_expression(start)
                    )
                } else {
                    format!(
                        "Array.from({{length: {} - {}}}, (_, i) => {} + i)",
                        self.generate_expression(end),
                        self.generate_expression(start),
                        self.generate_expression(start)
                    )
                }
            }

            Expression::Closure {
                parameters, body, ..
            } => {
                format!(
                    "({}) => {}",
                    parameters.join(", "),
                    self.generate_expression(body)
                )
            }

            Expression::Cast { expr, .. } => {
                // Just return the expression (JS is dynamically typed)
                self.generate_expression(expr)
            }

            Expression::ChannelSend { channel, value, .. } => {
                // Simplified: treat as method call
                format!(
                    "{}.send({})",
                    self.generate_expression(channel),
                    self.generate_expression(value)
                )
            }

            Expression::ChannelRecv { channel, .. } => {
                format!("{}.receive()", self.generate_expression(channel))
            }

            Expression::Block { statements, .. } => {
                let mut output = String::from("(() => {\n");
                self.indent_level += 1;
                for stmt in statements {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push_str("})()");
                output
            }
        }
    }

    pub(crate) fn generate_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) | Literal::IntSuffixed(n, _) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => {
                // Check for string interpolation markers (${ or just $ with identifier)
                if s.contains("${") || s.contains("$") {
                    // Use template literal with backticks for interpolation
                    format!("`{}`", s)
                } else {
                    // Use single quotes for plain strings
                    format!("'{}'", s.replace('\'', "\\'"))
                }
            }
            Literal::Char(c) => format!("'{}'", c),
            Literal::Bool(b) => b.to_string(),
        }
    }

    /// Generate console.log with format string support: println("{}", x) → console.log(`${x}`)
    fn generate_js_println(&self, args: &[String]) -> String {
        if args.is_empty() {
            return "console.log()".to_string();
        }
        // Check if first arg is a format string (quoted)
        let first = &args[0];
        if first.starts_with('\'') || first.starts_with('"') {
            let unquoted = &first[1..first.len() - 1];
            if unquoted.contains("{}") && args.len() > 1 {
                // Convert {} placeholders to template literal ${} syntax
                let mut template = unquoted.to_string();
                for arg in &args[1..] {
                    template = template.replacen("{}", &format!("${{{}}}", arg), 1);
                }
                return format!("console.log(`{}`)", template);
            }
        }
        format!("console.log({})", args.join(", "))
    }

    /// Generate console.log without newline (JS doesn't have print without newline easily)
    fn generate_js_print(&self, args: &[String]) -> String {
        if args.is_empty() {
            return "process.stdout.write('')".to_string();
        }
        let first = &args[0];
        if first.starts_with('\'') || first.starts_with('"') {
            let unquoted = &first[1..first.len() - 1];
            if unquoted.contains("{}") && args.len() > 1 {
                let mut template = unquoted.to_string();
                for arg in &args[1..] {
                    template = template.replacen("{}", &format!("${{{}}}", arg), 1);
                }
                return format!("process.stdout.write(`{}`)", template);
            }
        }
        format!("process.stdout.write(String({}))", args.join(" + "))
    }
}
