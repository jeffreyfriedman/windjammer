impl GoGenerator {
    fn generate_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::Literal { value, .. } => match value {
                Literal::Int(n) | Literal::IntSuffixed(n, _) => n.to_string(),
                Literal::Float(f) => {
                    let s = f.to_string();
                    if s.contains('.') {
                        s
                    } else {
                        format!("{}.0", s)
                    }
                }
                Literal::Bool(b) => b.to_string(),
                Literal::String(s) => format!("\"{}\"", s),
                Literal::Char(c) => format!("'{}'", c),
            },

            Expression::Identifier { name, .. } => {
                if let Some((type_name, rest)) = name.split_once("::") {
                    if rest == "new" {
                        format!("New{}", Self::capitalize(type_name))
                    } else {
                        format!(
                            "{}{}{{}}",
                            Self::capitalize(type_name),
                            Self::capitalize(rest)
                        )
                    }
                } else {
                    Self::escape_go_keyword(name)
                }
            }

            Expression::Binary {
                left, op, right, ..
            } => {
                let left_str = if let Expression::Binary {
                    op: ref left_op, ..
                } = **left
                {
                    if Self::op_precedence(left_op) < Self::op_precedence(op) {
                        format!("({})", self.generate_expression(left))
                    } else {
                        self.generate_expression(left)
                    }
                } else {
                    self.generate_expression(left)
                };
                let right_str = if let Expression::Binary {
                    op: ref right_op, ..
                } = **right
                {
                    if Self::op_precedence(right_op) <= Self::op_precedence(op) {
                        format!("({})", self.generate_expression(right))
                    } else {
                        self.generate_expression(right)
                    }
                } else {
                    self.generate_expression(right)
                };
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::Eq => "==",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Le => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                    BinaryOp::BitAnd => "&",
                    BinaryOp::BitOr => "|",
                    BinaryOp::BitXor => "^",
                    BinaryOp::Shl => "<<",
                    BinaryOp::Shr => ">>",
                };
                format!("{} {} {}", left_str, op_str, right_str)
            }

            Expression::Unary { op, operand, .. } => {
                let operand_str = self.generate_expression(operand);
                match op {
                    UnaryOp::Neg => format!("-{}", operand_str),
                    UnaryOp::Not => {
                        if matches!(**operand, Expression::Binary { .. }) {
                            format!("!({})", operand_str)
                        } else {
                            format!("!{}", operand_str)
                        }
                    }
                    _ => operand_str,
                }
            }

            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**function {
                    match name.as_str() {
                        "println" => {
                            return self.generate_println_call(arguments);
                        }
                        "print" => {
                            return self.generate_print_call(arguments);
                        }
                        _ => {}
                    }

                    if name.contains("::") {
                        if let Some((type_name, method_or_variant)) = name.split_once("::") {
                            if self.declared_enums.contains_key(type_name) {
                                let go_type = self.enum_variant_to_go_type(name);
                                if !arguments.is_empty() {
                                    let fields: Vec<String> = arguments
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (_, arg))| {
                                            format!("Field{}: {}", i, self.generate_expression(arg))
                                        })
                                        .collect();
                                    return format!("{}{{{}}}", go_type, fields.join(", "));
                                } else {
                                    return format!("{}{{}}", go_type);
                                }
                            } else {
                                let go_func = if method_or_variant == "new" {
                                    format!("New{}", capitalize_first(type_name))
                                } else {
                                    format!(
                                        "{}{}",
                                        capitalize_first(type_name),
                                        capitalize_first(method_or_variant)
                                    )
                                };
                                let args: Vec<String> = arguments
                                    .iter()
                                    .map(|(_, arg)| self.generate_expression(arg))
                                    .collect();
                                return format!("{}({})", go_func, args.join(", "));
                            }
                        }
                    }
                }

                let func_str = self.generate_expression(function);
                let args: Vec<String> = arguments
                    .iter()
                    .map(|(_, arg)| self.generate_expression(arg))
                    .collect();
                format!("{}({})", func_str, args.join(", "))
            }

            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let obj_str = self.generate_expression(object);
                let args: Vec<String> = arguments
                    .iter()
                    .map(|(_, arg)| self.generate_expression(arg))
                    .collect();
                match method.as_str() {
                    "len" => format!("int64(len({}))", obj_str),
                    "is_empty" => format!("len({}) == 0", obj_str),
                    "push" if args.len() == 1 => {
                        format!("append({}, {})", obj_str, args[0])
                    }
                    "contains" if args.len() == 1 => {
                        "/* contains */ false /* TODO */".to_string()
                    }
                    "to_string" => format!("fmt.Sprintf(\"%v\", {})", obj_str),
                    _ => {
                        let method_name = capitalize_first(method);
                        format!("{}.{}({})", obj_str, method_name, args.join(", "))
                    }
                }
            }

            Expression::FieldAccess { object, field, .. } => {
                let obj_str = self.generate_expression(object);
                let field_name = capitalize_first(field);
                format!("{}.{}", obj_str, field_name)
            }

            Expression::Index { object, index, .. } => {
                let obj_str = self.generate_expression(object);
                let index_str = self.generate_expression(index);
                format!("{}[{}]", obj_str, index_str)
            }

            Expression::StructLiteral { name, fields, .. } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(fname, fexpr)| {
                        let val = self.generate_expression(fexpr);
                        format!("{}: {}", capitalize_first(fname), val)
                    })
                    .collect();
                format!("{}{{{}}}", name, field_strs.join(", "))
            }

            Expression::Range { start, end, .. } => {
                let start_str = self.generate_expression(start);
                let end_str = self.generate_expression(end);
                format!("/* range {}..{} */", start_str, end_str)
            }

            Expression::Closure {
                parameters, body, ..
            } => {
                let params: Vec<String> = parameters
                    .iter()
                    .map(|p| format!("{} interface{{}}", p))
                    .collect();
                let body_str = self.generate_expression(body);
                format!(
                    "func({}) interface{{}} {{\n{}return {}\n{}}}",
                    params.join(", "),
                    self.indent(),
                    body_str,
                    self.indent()
                )
            }

            Expression::Cast { expr, type_, .. } => {
                let expr_str = self.generate_expression(expr);
                let type_str = self.type_to_go(type_);
                format!("{}({})", type_str, expr_str)
            }

            Expression::Array { elements, .. } => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| self.generate_expression(e))
                    .collect();
                format!("[]{{{}}}", elems.join(", "))
            }

            Expression::Tuple { elements, .. } => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| self.generate_expression(e))
                    .collect();
                if elems.is_empty() {
                    "struct{}{}".to_string()
                } else {
                    format!("/* tuple */ []{{{}}}", elems.join(", "))
                }
            }

            Expression::MacroInvocation {
                name,
                args: macro_args,
                ..
            } => {
                match name.as_str() {
                    "println" | "print" => {
                        self.needs_fmt_import = true;
                        let args: Vec<String> = macro_args
                            .iter()
                            .map(|a| self.generate_expression(a))
                            .collect();
                        if name == "println" {
                            format!("fmt.Println({})", args.join(", "))
                        } else {
                            format!("fmt.Print({})", args.join(", "))
                        }
                    }
                    "vec" => {
                        let args: Vec<String> = macro_args
                            .iter()
                            .map(|a| self.generate_expression(a))
                            .collect();
                        let elem_type = if let Some(first) = macro_args.first() {
                            match first {
                                Expression::Literal {
                                    value: Literal::Float(_),
                                    ..
                                } => "float64",
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } => "string",
                                Expression::Literal {
                                    value: Literal::Bool(_),
                                    ..
                                } => "bool",
                                _ => "int64",
                            }
                        } else {
                            "int64"
                        };
                        format!("[]{}{{{}}}", elem_type, args.join(", "))
                    }
                    "format" => {
                        self.needs_fmt_import = true;
                        let args: Vec<String> = macro_args
                            .iter()
                            .map(|a| self.generate_expression(a))
                            .collect();
                        if args.is_empty() {
                            "\"\"".to_string()
                        } else {
                            let fmt_str = args[0].replace("{}", "%v");
                            if args.len() == 1 {
                                format!("fmt.Sprintf({})", fmt_str)
                            } else {
                                format!("fmt.Sprintf({}, {})", fmt_str, args[1..].join(", "))
                            }
                        }
                    }
                    _ => {
                        let args: Vec<String> = macro_args
                            .iter()
                            .map(|a| self.generate_expression(a))
                            .collect();
                        format!("{}({})", name, args.join(", "))
                    }
                }
            }

            Expression::Block { statements, .. } => {
                let mut output = String::from("func() {\n");
                self.indent_level += 1;
                for stmt in statements {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                output.push_str(&format!("{}}}()", self.indent()));
                output
            }

            _ => "/* unsupported expression */".to_string(),
        }
    }

    fn extract_string_literal<'a>(&self, expr: &'a Expression) -> Option<&'a str> {
        if let Expression::Literal {
            value: Literal::String(s),
            ..
        } = expr
        {
            Some(s.as_str())
        } else {
            None
        }
    }

    fn generate_println_call(&mut self, arguments: &[(Option<String>, &Expression)]) -> String {
        if arguments.is_empty() {
            return "fmt.Println()".to_string();
        }

        if let Some(fmt_str) = arguments
            .first()
            .and_then(|(_, e)| self.extract_string_literal(e))
        {
            let fmt_str = fmt_str.to_string();
            if arguments.len() == 1 && !fmt_str.contains("{}") {
                return format!("fmt.Println(\"{}\")", fmt_str);
            }

            let go_fmt = fmt_str.replace("{}", "%v");
            let args: Vec<String> = arguments
                .iter()
                .skip(1)
                .map(|(_, arg)| self.generate_expression(arg))
                .collect();

            if args.is_empty() {
                format!("fmt.Println(\"{}\")", go_fmt)
            } else {
                format!("fmt.Printf(\"{}\\n\", {})", go_fmt, args.join(", "))
            }
        } else {
            let args: Vec<String> = arguments
                .iter()
                .map(|(_, arg)| self.generate_expression(arg))
                .collect();
            format!("fmt.Println({})", args.join(", "))
        }
    }

    fn generate_print_call(&mut self, arguments: &[(Option<String>, &Expression)]) -> String {
        if arguments.is_empty() {
            return "fmt.Print()".to_string();
        }

        if let Some(fmt_str) = arguments
            .first()
            .and_then(|(_, e)| self.extract_string_literal(e))
        {
            let fmt_str = fmt_str.to_string();
            if arguments.len() == 1 && !fmt_str.contains("{}") {
                return format!("fmt.Print(\"{}\")", fmt_str);
            }

            let go_fmt = fmt_str.replace("{}", "%v");
            let args: Vec<String> = arguments
                .iter()
                .skip(1)
                .map(|(_, arg)| self.generate_expression(arg))
                .collect();

            if args.is_empty() {
                format!("fmt.Print(\"{}\")", go_fmt)
            } else {
                format!("fmt.Printf(\"{}\", {})", go_fmt, args.join(", "))
            }
        } else {
            let args: Vec<String> = arguments
                .iter()
                .map(|(_, arg)| self.generate_expression(arg))
                .collect();
            format!("fmt.Print({})", args.join(", "))
        }
    }
}
