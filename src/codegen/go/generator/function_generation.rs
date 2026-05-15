impl GoGenerator {
    fn generate_program(&mut self, program: &Program) -> String {
        // First pass: collect struct names and check for fmt usage
        for item in &program.items {
            match item {
                Item::Struct { decl, .. } => {
                    self.declared_structs.push(decl.name.clone());
                }
                Item::Function { decl, .. } => {
                    self.scan_for_imports(decl);
                }
                Item::Impl { block, .. } => {
                    for method in &block.functions {
                        self.scan_for_imports(method);
                    }
                }
                Item::Enum { decl, .. } => {
                    self.declared_structs.push(decl.name.clone());
                    // TDD: Track enum variants for interface casting
                    let variant_names: Vec<String> = decl
                        .variants
                        .iter()
                        .map(|v| format!("{}{}", decl.name, v.name))
                        .collect();
                    self.declared_enums.insert(decl.name.clone(), variant_names);
                }
                _ => {}
            }
        }

        // Generate items
        let mut items_code = Vec::new();

        for item in &program.items {
            match item {
                Item::Function { decl, .. } => {
                    items_code.push(self.generate_function(decl));
                }
                Item::Struct { decl, .. } => {
                    items_code.push(self.generate_struct(decl));
                }
                Item::Enum { decl, .. } => {
                    items_code.push(self.generate_enum(decl));
                }
                Item::Trait { decl, .. } => {
                    items_code.push(self.generate_trait(decl));
                }
                Item::Impl { block, .. } => {
                    for method in &block.functions {
                        items_code.push(self.generate_method(&block.type_name, method));
                    }
                }
                Item::Const {
                    name, value, type_, ..
                } => {
                    let type_str = self.type_to_go(type_);
                    let val_str = self.generate_expression(value);
                    if type_str.is_empty() {
                        items_code.push(format!("const {} = {}\n", name, val_str));
                    } else {
                        items_code.push(format!("const {} {} = {}\n", name, type_str, val_str));
                    }
                }
                _ => {
                    // Skip unsupported items for now
                }
            }
        }

        // Assemble final output
        let mut output = String::new();
        output.push_str("package main\n\n");

        // Imports
        let mut imports = Vec::new();
        if self.needs_fmt_import {
            imports.push("\"fmt\"");
        }
        if self.needs_math_import {
            imports.push("\"math\"");
        }

        if !imports.is_empty() {
            if imports.len() == 1 {
                output.push_str(&format!("import {}\n\n", imports[0]));
            } else {
                output.push_str("import (\n");
                for imp in &imports {
                    output.push_str(&format!("\t{}\n", imp));
                }
                output.push_str(")\n\n");
            }
        }

        output.push_str(&items_code.join("\n"));
        output
    }

    /// Scan a function for import needs (e.g., does it use println?)
    fn scan_for_imports(&mut self, func: &FunctionDecl) {
        self.scan_statements_for_imports(&func.body);
    }

    fn scan_statements_for_imports(&mut self, stmts: &[&Statement]) {
        for stmt in stmts {
            self.scan_statement_for_imports(stmt);
        }
    }

    fn scan_statement_for_imports(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.scan_expression_for_imports(expr);
            }
            Statement::Let { value, .. } => {
                self.scan_expression_for_imports(value);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.scan_expression_for_imports(condition);
                self.scan_statements_for_imports(then_block);
                if let Some(else_block) = else_block {
                    self.scan_statements_for_imports(else_block);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.scan_expression_for_imports(condition);
                self.scan_statements_for_imports(body);
            }
            Statement::For { body, .. } => {
                self.scan_statements_for_imports(body);
            }
            Statement::Loop { body, .. } => {
                self.scan_statements_for_imports(body);
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                self.scan_expression_for_imports(expr);
            }
            Statement::Assignment { value, .. } => {
                self.scan_expression_for_imports(value);
            }
            _ => {}
        }
    }

    fn scan_expression_for_imports(&mut self, expr: &Expression) {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**function {
                    if name == "println" || name == "print" {
                        self.needs_fmt_import = true;
                    }
                }
                self.scan_expression_for_imports(function);
                for (_, arg) in arguments {
                    self.scan_expression_for_imports(arg);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.scan_expression_for_imports(left);
                self.scan_expression_for_imports(right);
            }
            Expression::Unary { operand, .. } => {
                self.scan_expression_for_imports(operand);
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.scan_expression_for_imports(object);
                for (_, arg) in arguments {
                    self.scan_expression_for_imports(arg);
                }
            }
            _ => {}
        }
    }

    fn generate_struct(&mut self, s: &crate::parser::StructDecl) -> String {
        let mut output = String::new();
        output.push_str(&format!("type {} struct {{\n", &s.name));
        for field in &s.fields {
            let go_type = self.type_to_go(&field.field_type);
            // Go exported fields start with uppercase
            let field_name = capitalize_first(&field.name);
            output.push_str(&format!("\t{} {}\n", field_name, go_type));
        }
        output.push_str("}\n");
        output
    }

    fn generate_enum(&mut self, e: &crate::parser::EnumDecl) -> String {
        use crate::parser::EnumVariantData;
        let mut output = String::new();

        // Generate interface type for the enum
        output.push_str(&format!("type {} interface {{\n", e.name));
        output.push_str(&format!("\tIs{}()\n", e.name));
        output.push_str("}\n\n");

        // Generate variant structs
        for variant in &e.variants {
            let variant_type = format!("{}{}", e.name, variant.name);
            match &variant.data {
                EnumVariantData::Unit => {
                    output.push_str(&format!("type {} struct{{}}\n", variant_type));
                }
                EnumVariantData::Tuple(types) => {
                    output.push_str(&format!("type {} struct {{\n", variant_type));
                    for (i, t) in types.iter().enumerate() {
                        output.push_str(&format!("\tField{} {}\n", i, self.type_to_go(t)));
                    }
                    output.push_str("}\n");
                }
                EnumVariantData::Struct(fields) => {
                    output.push_str(&format!("type {} struct {{\n", variant_type));
                    for (name, t) in fields {
                        output.push_str(&format!(
                            "\t{} {}\n",
                            capitalize_first(name),
                            self.type_to_go(t)
                        ));
                    }
                    output.push_str("}\n");
                }
            }
            // Implement the interface marker method
            output.push_str(&format!(
                "func ({} {}) Is{}() {{}}\n\n",
                variant_type.chars().next().unwrap().to_lowercase(),
                variant_type,
                e.name
            ));
        }

        output
    }

    fn generate_trait(&mut self, t: &crate::parser::TraitDecl) -> String {
        let mut output = String::new();
        output.push_str(&format!("type {} interface {{\n", t.name));
        for method in &t.methods {
            let params: Vec<String> = method
                .parameters
                .iter()
                .filter(|p| p.name != "self")
                .map(|p| format!("{} {}", p.name, self.type_to_go(&p.type_)))
                .collect();
            let ret = match &method.return_type {
                Some(t) => format!(" {}", self.type_to_go(t)),
                None => String::new(),
            };
            output.push_str(&format!(
                "\t{}({}){}\n",
                capitalize_first(&method.name),
                params.join(", "),
                ret
            ));
        }
        output.push_str("}\n");
        output
    }

    fn generate_function(&mut self, func: &FunctionDecl) -> String {
        let mut output = String::new();

        // Function name (capitalize for Go export if it's main)
        let func_name = &func.name;

        // Parameters (TDD FIX: Escape Go keywords in param names)
        let params: Vec<String> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .map(|p| {
                format!(
                    "{} {}",
                    Self::escape_go_keyword(&p.name),
                    self.type_to_go(&p.type_)
                )
            })
            .collect();

        // Return type
        let return_type = match &func.return_type {
            Some(t) => format!(" {}", self.type_to_go(t)),
            None => String::new(),
        };

        output.push_str(&format!(
            "func {}({}){} {{\n",
            func_name,
            params.join(", "),
            return_type
        ));

        self.push_scope();
        // Register parameters as declared variables
        for p in &func.parameters {
            if p.name != "self" {
                self.declare_var(&p.name);
            }
        }

        self.indent_level += 1;
        let has_return_type = func.return_type.is_some();
        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == body_len - 1;
            // In Go, the last expression must be an explicit return
            if is_last && has_return_type {
                if let Statement::Expression { expr, .. } = stmt {
                    output.push_str(&self.generate_expression_as_return(expr));
                    continue;
                }
                // Match as last statement in a returning function
                if let Statement::Match { value, arms, .. } = stmt {
                    output.push_str(&self.generate_match_with_returns(value, arms));
                    continue;
                }
            }
            output.push_str(&self.generate_statement(stmt));
        }
        self.indent_level -= 1;
        self.pop_scope();

        output.push_str("}\n");
        output
    }

    fn generate_expression_as_return(&mut self, expr: &Expression) -> String {
        // For match expressions used as return values, we need to generate
        // a switch where each arm has its own `return`
        if let Expression::Block { statements, .. } = expr {
            // If the block's last statement is a match, handle it
            let mut output = String::new();
            let len = statements.len();
            for (i, stmt) in statements.iter().enumerate() {
                if i == len - 1 {
                    if let Statement::Match { value, arms, .. } = stmt {
                        output.push_str(&self.generate_match_with_returns(value, arms));
                        return output;
                    }
                    if let Statement::Expression { expr, .. } = stmt {
                        return format!("{}{}\n", output, self.generate_expression_as_return(expr));
                    }
                }
                output.push_str(&self.generate_statement(stmt));
            }
            return output;
        }

        let indent = self.indent();
        let expr_str = self.generate_expression(expr);
        format!("{}return {}\n", indent, expr_str)
    }

    fn generate_static_method(&mut self, type_name: &str, func: &FunctionDecl) -> String {
        let mut output = String::new();

        let params: Vec<String> = func
            .parameters
            .iter()
            .map(|p| format!("{} {}", &p.name, self.type_to_go(&p.type_)))
            .collect();

        let return_type = match &func.return_type {
            Some(t) => format!(" {}", self.type_to_go(t)),
            None => String::new(),
        };

        // Go convention: NewType for constructors, TypeMethod for other statics
        let go_name = if func.name == "new" {
            format!("New{}", Self::capitalize(type_name))
        } else {
            format!(
                "{}{}",
                Self::capitalize(type_name),
                Self::capitalize(&func.name)
            )
        };

        output.push_str(&format!(
            "func {}({}){} {{\n",
            go_name,
            params.join(", "),
            return_type
        ));

        self.push_scope();
        for p in &func.parameters {
            self.declare_var(&p.name);
        }
        self.indent_level += 1;

        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            if i == body_len - 1 && func.return_type.is_some() {
                if let Statement::Expression { expr, .. } = stmt {
                    output.push_str(&self.generate_expression_as_return(expr));
                    continue;
                }
            }
            output.push_str(&self.generate_statement(stmt));
        }

        self.indent_level -= 1;
        self.pop_scope();
        output.push_str("}\n");
        output
    }

    fn generate_method(&mut self, type_name: &str, func: &FunctionDecl) -> String {
        // Check if this is a static method (no self parameter)
        let has_self = func.parameters.iter().any(|p| p.name == "self");
        if !has_self {
            return self.generate_static_method(type_name, func);
        }

        let mut output = String::new();

        // Receiver: use pointer receiver for methods that mutate
        let receiver_name = type_name.chars().next().unwrap().to_lowercase().to_string();
        let receiver = format!("{} *{}", receiver_name, type_name);

        // Parameters (skip self)
        let params: Vec<String> = func
            .parameters
            .iter()
            .filter(|p| p.name != "self")
            .map(|p| format!("{} {}", &p.name, self.type_to_go(&p.type_)))
            .collect();

        // Return type
        let return_type = match &func.return_type {
            Some(t) => format!(" {}", self.type_to_go(t)),
            None => String::new(),
        };

        // Capitalize method name for Go export
        let method_name = capitalize_first(&func.name);

        output.push_str(&format!(
            "func ({}) {}({}){} {{\n",
            receiver,
            method_name,
            params.join(", "),
            return_type
        ));

        self.push_scope();
        for p in &func.parameters {
            if p.name != "self" {
                self.declare_var(&p.name);
            }
        }
        self.indent_level += 1;

        // Generate body, replacing `self.` with receiver
        let has_return_type = func.return_type.is_some();
        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == body_len - 1;
            // In Go, the last expression must be an explicit return
            if is_last && has_return_type {
                if let Statement::Expression { expr, .. } = stmt {
                    let code = self.generate_expression_as_return(expr);
                    let code = code.replace("self.", &format!("{}.", receiver_name));
                    output.push_str(&code);
                    continue;
                }
                if let Statement::Match { value, arms, .. } = stmt {
                    let code = self.generate_match_with_returns(value, arms);
                    let code = code.replace("self.", &format!("{}.", receiver_name));
                    output.push_str(&code);
                    continue;
                }
            }
            let code = self.generate_statement(stmt);
            let code = code.replace("self.", &format!("{}.", receiver_name));
            output.push_str(&code);
        }

        self.indent_level -= 1;
        self.pop_scope();
        output.push_str("}\n");
        output
    }
}
