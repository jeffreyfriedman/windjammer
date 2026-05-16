impl GoGenerator {
    fn generate_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::Let {
                pattern,
                mutable,
                value,
                type_: _,
                ..
            } => {
                let indent = self.indent();
                let var_name = self.pattern_to_go(pattern);
                let mut value_str = self.generate_expression(value);

                let is_bare_int_literal = matches!(
                    value,
                    Expression::Literal {
                        value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                        ..
                    }
                );
                if is_bare_int_literal {
                    value_str = format!("int64({})", value_str);
                }

                let needs_interface_type = matches!(value, Expression::Call { function, .. }
                    if matches!(&**function, Expression::Identifier { name, .. }
                        if name.split_once("::").map(|(enum_name, _)| self.declared_enums.contains_key(enum_name)).unwrap_or(false)
                    )
                );

                let result = if self.is_var_declared(&var_name) {
                    format!("{}{} = {}\n", indent, var_name, value_str)
                } else if needs_interface_type {
                    let (enum_name, _) = match value {
                        Expression::Call { function, .. } => match &**function {
                            Expression::Identifier { name, .. } => name
                                .split_once("::")
                                .expect("needs_interface_type guarantees :: exists"),
                            _ => unreachable!("needs_interface_type guarantees Identifier"),
                        },
                        _ => unreachable!("needs_interface_type guarantees Call"),
                    };
                    self.declare_var(&var_name);
                    format!("{}var {} {} = {}\n", indent, var_name, enum_name, value_str)
                } else if *mutable {
                    self.declare_var(&var_name);
                    format!("{}var {} = {}\n", indent, var_name, value_str)
                } else {
                    self.declare_var(&var_name);
                    format!("{}{} := {}\n", indent, var_name, value_str)
                };
                result
            }

            Statement::Assignment {
                target,
                value,
                compound_op,
                ..
            } => {
                let indent = self.indent();
                let target_str = self.generate_expression(target);
                let value_str = self.generate_expression(value);

                if let Some(op) = compound_op {
                    let op_str = match op {
                        CompoundOp::Add => "+=",
                        CompoundOp::Sub => "-=",
                        CompoundOp::Mul => "*=",
                        CompoundOp::Div => "/=",
                        CompoundOp::Mod => "%=",
                        CompoundOp::BitAnd => "&=",
                        CompoundOp::BitOr => "|=",
                        CompoundOp::BitXor => "^=",
                        CompoundOp::Shl => "<<=",
                        CompoundOp::Shr => ">>=",
                    };
                    format!("{}{} {} {}\n", indent, target_str, op_str, value_str)
                } else {
                    format!("{}{} = {}\n", indent, target_str, value_str)
                }
            }

            Statement::Expression { expr, .. } => {
                let indent = self.indent();
                if let Expression::MethodCall {
                    object,
                    method,
                    arguments,
                    ..
                } = expr
                {
                    if method == "push" && arguments.len() == 1 {
                        let obj_str = self.generate_expression(object);
                        let arg_str = self.generate_expression(arguments[0].1);
                        return format!(
                            "{}{} = append({}, {})\n",
                            indent, obj_str, obj_str, arg_str
                        );
                    }
                }
                let expr_str = self.generate_expression(expr);
                format!("{}{}\n", indent, expr_str)
            }

            Statement::Return { value, .. } => {
                let indent = self.indent();
                match value {
                    Some(expr) => {
                        let expr_str = self.generate_expression(expr);
                        format!("{}return {}\n", indent, expr_str)
                    }
                    None => format!("{}return\n", indent),
                }
            }

            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let indent = self.indent();
                let condition_str = self.generate_expression(condition);
                let mut output = format!("{}if {} {{\n", indent, condition_str);

                self.indent_level += 1;
                for stmt in then_block {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                if let Some(else_stmts) = else_block {
                    if else_stmts.len() == 1 {
                        if let Statement::If { .. } = else_stmts[0] {
                            output.push_str(&format!("{}}} else ", indent));
                            let inner = self.generate_statement(else_stmts[0]);
                            output.push_str(inner.trim_start());
                            return output;
                        }
                    }
                    output.push_str(&format!("{}}} else {{\n", indent));
                    self.indent_level += 1;
                    for stmt in else_stmts {
                        output.push_str(&self.generate_statement(stmt));
                    }
                    self.indent_level -= 1;
                    output.push_str(&format!("{}}}\n", indent));
                } else {
                    output.push_str(&format!("{}}}\n", indent));
                }

                output
            }

            Statement::While {
                condition, body, ..
            } => {
                let indent = self.indent();
                let condition_str = self.generate_expression(condition);
                let mut output = format!("{}for {} {{\n", indent, condition_str);

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&format!("{}}}\n", indent));
                output
            }

            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                let indent = self.indent();
                let var = self.pattern_to_go(pattern);
                let iterable_str = self.generate_expression(iterable);

                let output = if let Expression::Range { start, end, .. } = iterable {
                    let mut start_str = self.generate_expression(start);
                    let end_str = self.generate_expression(end);

                    let is_int_literal = matches!(
                        start,
                        Expression::Literal {
                            value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                            ..
                        }
                    );
                    if is_int_literal {
                        start_str = format!("int64({})", start_str);
                    }

                    format!(
                        "{}for {} := {}; {} < {}; {}++ {{\n",
                        indent, var, start_str, var, end_str, var
                    )
                } else {
                    format!("{}for _, {} := range {} {{\n", indent, var, iterable_str)
                };

                let mut result = output;
                self.indent_level += 1;
                for stmt in body {
                    result.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                result.push_str(&format!("{}}}\n", indent));
                result
            }

            Statement::Loop { body, .. } => {
                let indent = self.indent();
                let mut output = format!("{}for {{\n", indent);

                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;

                output.push_str(&format!("{}}}\n", indent));
                output
            }

            Statement::Match { value, arms, .. } => self.generate_match_statement(value, arms),

            Statement::Break { .. } => {
                format!("{}break\n", self.indent())
            }

            Statement::Continue { .. } => {
                format!("{}continue\n", self.indent())
            }

            _ => {
                format!("{}// TODO: unsupported statement\n", self.indent())
            }
        }
    }

    fn generate_match_statement(&mut self, value: &Expression, arms: &[MatchArm]) -> String {
        let indent = self.indent();
        let val_str = self.generate_expression(value);
        let mut output = String::new();

        let is_type_switch = arms.iter().any(|arm| {
            matches!(&arm.pattern, Pattern::EnumVariant(..))
                || matches!(&arm.pattern, Pattern::Identifier(name) if name.contains("::"))
        });

        let has_guards = arms.iter().any(|arm| arm.guard.is_some());

        if is_type_switch {
            output.push_str(&format!("{}switch _v := {}.(type) {{\n", indent, val_str));
            self.indent_level += 1;
            for arm in arms {
                output.push_str(&self.generate_match_arm_type_switch(arm));
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        } else if has_guards {
            output.push_str(&format!("{}{{\n", indent));
            self.indent_level += 1;
            output.push_str(&format!("{}__match_val := {}\n", self.indent(), val_str));
            for (i, arm) in arms.iter().enumerate() {
                let body_str = self.generate_expression(arm.body);
                let condition = match &arm.pattern {
                    Pattern::Wildcard => "true".to_string(),
                    Pattern::Literal(lit) => {
                        let lit_str = match lit {
                            Literal::Int(n) | Literal::IntSuffixed(n, _) => n.to_string(),
                            Literal::Float(f) => f.to_string(),
                            Literal::Bool(b) => b.to_string(),
                            Literal::String(s) => format!("\"{}\"", s),
                            Literal::Char(c) => format!("'{}'", c),
                        };
                        format!("__match_val == {}", lit_str)
                    }
                    Pattern::Identifier(_) => "true".to_string(),
                    _ => "true".to_string(),
                };

                let full_condition = if let Some(guard) = arm.guard {
                    let guard_str = self.generate_expression(guard);
                    if let Pattern::Identifier(name) = &arm.pattern {
                        format!(
                            "func() bool {{ {} := __match_val; return {} }}()",
                            name, guard_str
                        )
                    } else {
                        format!("{} && {}", condition, guard_str)
                    }
                } else {
                    condition
                };

                if i == 0 {
                    output.push_str(&format!("{}if {} {{\n", self.indent(), full_condition));
                } else if full_condition == "true" {
                    output.push_str(&format!("{}}} else {{\n", self.indent()));
                } else {
                    output.push_str(&format!(
                        "{}}} else if {} {{\n",
                        self.indent(),
                        full_condition
                    ));
                }
                self.indent_level += 1;
                if let Pattern::Identifier(name) = &arm.pattern {
                    output.push_str(&format!("{}{} := __match_val\n", self.indent(), name));
                    output.push_str(&format!("{}_ = {}\n", self.indent(), name));
                }
                output.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;

                if i == arms.len() - 1 {
                    output.push_str(&format!("{}}}\n", self.indent()));
                }
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        } else {
            output.push_str(&format!("{}switch {} {{\n", indent, val_str));
            self.indent_level += 1;
            for arm in arms {
                output.push_str(&self.generate_match_arm(arm));
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        }

        output
    }

    fn generate_match_arm(&mut self, arm: &MatchArm) -> String {
        let indent = self.indent();
        let body_str = self.generate_expression(arm.body);

        match &arm.pattern {
            Pattern::Wildcard => {
                let mut out = format!("{}default:\n", indent);
                self.indent_level += 1;
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            Pattern::Literal(lit) => {
                let pat_str = match lit {
                    Literal::Int(n) | Literal::IntSuffixed(n, _) => n.to_string(),
                    Literal::Float(f) => f.to_string(),
                    Literal::Bool(b) => b.to_string(),
                    Literal::String(s) => format!("\"{}\"", s),
                    Literal::Char(c) => format!("'{}'", c),
                };
                let mut out = format!("{}case {}:\n", indent, pat_str);
                self.indent_level += 1;
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            Pattern::Identifier(name) => {
                let mut out = format!("{}default:\n", indent);
                self.indent_level += 1;
                out.push_str(&format!(
                    "{}{} := {}\n",
                    self.indent(),
                    name,
                    "/* matched value */"
                ));
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            Pattern::Or(patterns) => {
                let cases: Vec<String> = patterns
                    .iter()
                    .map(|p| self.pattern_to_case_label(p))
                    .collect();
                let mut out = format!("{}case {}:\n", indent, cases.join(", "));
                self.indent_level += 1;
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            _ => {
                let mut out = format!("{}// TODO: unsupported match pattern\n", indent);
                out.push_str(&format!("{}default:\n", indent));
                self.indent_level += 1;
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
        }
    }

    fn generate_match_arm_type_switch(&mut self, arm: &MatchArm) -> String {
        let indent = self.indent();
        let body_str = self.generate_expression(arm.body);

        match &arm.pattern {
            Pattern::EnumVariant(variant_name, binding) => {
                let go_type = self.enum_variant_to_go_type(variant_name);
                let mut out = format!("{}case {}:\n", indent, go_type);
                self.indent_level += 1;

                match binding {
                    EnumPatternBinding::Single(var_name) => {
                        out.push_str(&format!("{}{} := _v.Field0\n", self.indent(), var_name));
                    }
                    EnumPatternBinding::Tuple(patterns) => {
                        for (i, pat) in patterns.iter().enumerate() {
                            if let Pattern::Identifier(var_name) = pat {
                                out.push_str(&format!(
                                    "{}{} := _v.Field{}\n",
                                    self.indent(),
                                    var_name,
                                    i
                                ));
                            }
                        }
                    }
                    EnumPatternBinding::Wildcard | EnumPatternBinding::None => {
                        out.push_str(&format!("{}_ = _v\n", self.indent()));
                    }
                    EnumPatternBinding::Struct(_, _) => {
                        out.push_str(&format!("{}_ = _v\n", self.indent()));
                    }
                }

                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            Pattern::Identifier(name) if name.contains("::") => {
                let go_type = self.enum_variant_to_go_type(name);
                let mut out = format!("{}case {}:\n", indent, go_type);
                self.indent_level += 1;
                out.push_str(&format!("{}_ = _v\n", self.indent()));
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            Pattern::Wildcard => {
                let mut out = format!("{}default:\n", indent);
                self.indent_level += 1;
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            _ => {
                let mut out = format!("{}default:\n", indent);
                self.indent_level += 1;
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
        }
    }

    fn generate_match_with_returns(&mut self, value: &Expression, arms: &[MatchArm]) -> String {
        let indent = self.indent();
        let val_str = self.generate_expression(value);
        let mut output = String::new();

        let is_type_switch = arms.iter().any(|arm| {
            matches!(&arm.pattern, Pattern::EnumVariant(..))
                || matches!(&arm.pattern, Pattern::Identifier(name) if name.contains("::"))
        });

        let has_default = arms
            .iter()
            .any(|arm| matches!(&arm.pattern, Pattern::Wildcard));
        let has_guards = arms.iter().any(|arm| arm.guard.is_some());

        if is_type_switch {
            output.push_str(&format!("{}switch _v := {}.(type) {{\n", indent, val_str));
            self.indent_level += 1;
            for arm in arms {
                output.push_str(&self.generate_match_arm_with_return_type_switch(arm));
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        } else if has_guards {
            output.push_str(&format!("{}{{\n", indent));
            self.indent_level += 1;
            output.push_str(&format!("{}__match_val := {}\n", self.indent(), val_str));

            for (i, arm) in arms.iter().enumerate() {
                let body_str = self.generate_expression(arm.body);
                let condition = match &arm.pattern {
                    Pattern::Wildcard => "true".to_string(),
                    Pattern::Literal(lit) => {
                        let lit_str = match lit {
                            Literal::Int(n) | Literal::IntSuffixed(n, _) => n.to_string(),
                            Literal::Float(f) => f.to_string(),
                            Literal::Bool(b) => b.to_string(),
                            Literal::String(s) => format!("\"{}\"", s),
                            Literal::Char(c) => format!("'{}'", c),
                        };
                        format!("__match_val == {}", lit_str)
                    }
                    Pattern::Identifier(_) => "true".to_string(),
                    _ => "true".to_string(),
                };

                let full_condition = if let Some(guard) = &arm.guard {
                    if let Pattern::Identifier(name) = &arm.pattern {
                        let guard_str = self.generate_expression(guard);
                        format!(
                            "func() bool {{ {} := __match_val; return {} }}()",
                            name, guard_str
                        )
                    } else {
                        let guard_str = self.generate_expression(guard);
                        format!("{} && {}", condition, guard_str)
                    }
                } else {
                    condition
                };

                let is_last = i == arms.len() - 1;
                let is_catchall = matches!(&arm.pattern, Pattern::Wildcard)
                    || (matches!(&arm.pattern, Pattern::Identifier(_)) && arm.guard.is_none());

                if is_last && is_catchall {
                    let keyword = if i == 0 { "if true" } else { "} else" };
                    output.push_str(&format!("{}{} {{\n", self.indent(), keyword));
                } else {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    output.push_str(&format!(
                        "{}{} {} {{\n",
                        self.indent(),
                        keyword,
                        full_condition
                    ));
                }
                self.indent_level += 1;
                output.push_str(&format!("{}return {}\n", self.indent(), body_str));
                self.indent_level -= 1;
            }

            output.push_str(&format!("{}}}\n", self.indent()));
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        } else {
            output.push_str(&format!("{}switch {} {{\n", indent, val_str));
            self.indent_level += 1;
            for arm in arms {
                output.push_str(&self.generate_match_arm_with_return(arm));
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        }

        if !has_default && !has_guards {
            output.push_str(&format!("{}panic(\"unreachable match\")\n", indent));
        }

        output
    }

    fn generate_match_arm_with_return(&mut self, arm: &MatchArm) -> String {
        let indent = self.indent();
        let body_str = self.generate_expression(arm.body);

        let case_label = match &arm.pattern {
            Pattern::Wildcard => "default".to_string(),
            Pattern::Literal(lit) => {
                let val = match lit {
                    Literal::Int(n) | Literal::IntSuffixed(n, _) => n.to_string(),
                    Literal::Float(f) => f.to_string(),
                    Literal::Bool(b) => b.to_string(),
                    Literal::String(s) => format!("\"{}\"", s),
                    Literal::Char(c) => format!("'{}'", c),
                };
                format!("case {}", val)
            }
            Pattern::Or(patterns) => {
                let cases: Vec<String> = patterns
                    .iter()
                    .map(|p| self.pattern_to_case_label(p))
                    .collect();
                format!("case {}", cases.join(", "))
            }
            _ => "default".to_string(),
        };

        let mut out = format!("{}{}:\n", indent, case_label);
        self.indent_level += 1;
        out.push_str(&format!("{}return {}\n", self.indent(), body_str));
        self.indent_level -= 1;
        out
    }

    fn generate_match_arm_with_return_type_switch(&mut self, arm: &MatchArm) -> String {
        let indent = self.indent();
        let body_str = self.generate_expression(arm.body);

        let case_label = match &arm.pattern {
            Pattern::EnumVariant(variant_name, _) => {
                format!("case {}", self.enum_variant_to_go_type(variant_name))
            }
            Pattern::Identifier(name) if name.contains("::") => {
                format!("case {}", self.enum_variant_to_go_type(name))
            }
            Pattern::Wildcard => "default".to_string(),
            _ => "default".to_string(),
        };

        let mut out = format!("{}{}:\n", indent, case_label);
        self.indent_level += 1;

        match &arm.pattern {
            Pattern::EnumVariant(_, binding) => match binding {
                EnumPatternBinding::Single(var_name) => {
                    out.push_str(&format!("{}{} := _v.Field0\n", self.indent(), var_name));
                }
                EnumPatternBinding::Tuple(patterns) => {
                    for (i, pat) in patterns.iter().enumerate() {
                        if let Pattern::Identifier(var_name) = pat {
                            out.push_str(&format!(
                                "{}{} := _v.Field{}\n",
                                self.indent(),
                                var_name,
                                i
                            ));
                        }
                    }
                }
                EnumPatternBinding::Wildcard | EnumPatternBinding::None => {
                    out.push_str(&format!("{}_ = _v\n", self.indent()));
                }
                EnumPatternBinding::Struct(_, _) => {
                    out.push_str(&format!("{}_ = _v\n", self.indent()));
                }
            },
            _ => {
                out.push_str(&format!("{}_ = _v\n", self.indent()));
            }
        }

        out.push_str(&format!("{}return {}\n", self.indent(), body_str));
        self.indent_level -= 1;
        out
    }

    fn pattern_to_case_label(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Literal(lit) => match lit {
                Literal::Int(n) | Literal::IntSuffixed(n, _) => n.to_string(),
                Literal::Float(f) => f.to_string(),
                Literal::Bool(b) => b.to_string(),
                Literal::String(s) => format!("\"{}\"", s),
                Literal::Char(c) => format!("'{}'", c),
            },
            Pattern::Identifier(name) => name.clone(),
            _ => "/* unsupported pattern */".to_string(),
        }
    }
}
