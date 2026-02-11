//! Go code generator
//!
//! Generates idiomatic Go source code from the Windjammer AST.

use crate::codegen::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::parser::{
    BinaryOp, CompoundOp, EnumVariantData, Expression, FunctionDecl, Item, Literal, MatchArm,
    Pattern, Program, Statement, Type, UnaryOp,
};
use anyhow::Result;

/// Go code generation backend
pub struct GoBackend;

impl GoBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CodegenBackend for GoBackend {
    fn name(&self) -> &str {
        "Go"
    }

    fn target(&self) -> Target {
        Target::Go
    }

    fn generate(&self, program: &Program, _config: &CodegenConfig) -> Result<CodegenOutput> {
        let mut gen = GoGenerator::new();
        let code = gen.generate_program(program);
        Ok(CodegenOutput::new(code, "go".to_string()))
    }

    fn generate_additional_files(
        &self,
        _program: &Program,
        _config: &CodegenConfig,
    ) -> Vec<(String, String)> {
        // Generate go.mod
        let go_mod = "module windjammer-generated\n\ngo 1.21\n".to_string();
        vec![("go.mod".to_string(), go_mod)]
    }
}

/// Internal Go code generator
struct GoGenerator {
    indent_level: usize,
    needs_fmt_import: bool,
    needs_math_import: bool,
    /// Track structs that have been declared (for method generation)
    declared_structs: Vec<String>,
    /// Track variables declared in current scope (for shadowing detection)
    declared_vars: Vec<std::collections::HashSet<String>>,
}

impl GoGenerator {
    fn new() -> Self {
        Self {
            indent_level: 0,
            needs_fmt_import: false,
            needs_math_import: false,
            declared_structs: Vec::new(),
            declared_vars: vec![std::collections::HashSet::new()],
        }
    }

    /// Get operator precedence (higher = tighter binding)
    fn op_precedence(op: &BinaryOp) -> i32 {
        match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::BitOr => 3,
            BinaryOp::BitXor => 4,
            BinaryOp::BitAnd => 5,
            BinaryOp::Eq | BinaryOp::Ne => 6,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 7,
            BinaryOp::Shl | BinaryOp::Shr => 8,
            BinaryOp::Add | BinaryOp::Sub => 9,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 10,
        }
    }

    fn push_scope(&mut self) {
        self.declared_vars.push(std::collections::HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.declared_vars.pop();
    }

    /// Check if a variable name is already declared in the current scope stack
    fn is_var_declared(&self, name: &str) -> bool {
        self.declared_vars.iter().any(|scope| scope.contains(name))
    }

    /// Mark a variable as declared in the current scope
    fn declare_var(&mut self, name: &str) {
        if let Some(scope) = self.declared_vars.last_mut() {
            scope.insert(name.to_string());
        }
    }

    /// Capitalize first letter of a string (for Go exported names)
    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    /// Convert "Color::Red" → "ColorRed" for Go type names
    fn enum_variant_to_go_type(&self, variant_name: &str) -> String {
        if let Some((type_name, variant)) = variant_name.split_once("::") {
            format!("{}{}", Self::capitalize(type_name), Self::capitalize(variant))
        } else {
            variant_name.to_string()
        }
    }

    fn indent(&self) -> String {
        "\t".repeat(self.indent_level)
    }

    // =====================================================
    // Program Generation
    // =====================================================

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

    // =====================================================
    // Struct Generation
    // =====================================================

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

    // =====================================================
    // Enum Generation (tagged unions → interface + variant structs)
    // =====================================================

    fn generate_enum(&mut self, e: &crate::parser::EnumDecl) -> String {
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
                        output.push_str(&format!("\t{} {}\n", capitalize_first(name), self.type_to_go(t)));
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

    // =====================================================
    // Trait Generation (traits → interfaces)
    // =====================================================

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

    // =====================================================
    // Function Generation
    // =====================================================

    fn generate_function(&mut self, func: &FunctionDecl) -> String {
        let mut output = String::new();

        // Function name (capitalize for Go export if it's main)
        let func_name = &func.name;

        // Parameters
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

    /// Generate an expression as a return statement.
    /// If the expression is a match/block, handle it specially since Go can't return a switch.
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

    /// Generate a static/associated function as a Go package-level function.
    /// `Type::new(x, y)` → `func NewType(x, y) Type { ... }`
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
            format!("{}{}", Self::capitalize(type_name), Self::capitalize(&func.name))
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

    // =====================================================
    // Statement Generation
    // =====================================================

    fn generate_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::Let {
                pattern,
                mutable,
                value,
                type_,
                ..
            } => {
                let indent = self.indent();
                let var_name = self.pattern_to_go(pattern);
                let value_str = self.generate_expression(value);

                // Go doesn't allow re-declaration of a variable in the same scope.
                // For shadowing, we use a temporary variable + assignment pattern.
                let result = if self.is_var_declared(&var_name) {
                    // Variable shadowing: reassign using =
                    // Go allows assignment to an existing variable
                    let _ = type_;
                    format!("{}{} = {}\n", indent, var_name, value_str)
                } else if *mutable {
                    self.declare_var(&var_name);
                    format!("{}var {} = {}\n", indent, var_name, value_str)
                } else {
                    let _ = type_;
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
                // Special case: v.push(x) → v = append(v, x) in Go
                if let Expression::MethodCall { object, method, arguments, .. } = expr {
                    if method == "push" && arguments.len() == 1 {
                        let obj_str = self.generate_expression(object);
                        let arg_str = self.generate_expression(&arguments[0].1);
                        return format!("{}{} = append({}, {})\n", indent, obj_str, obj_str, arg_str);
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
                    // Check if else block is a single if statement (else if)
                    if else_stmts.len() == 1 {
                        if let Statement::If { .. } = else_stmts[0] {
                            output.push_str(&format!("{}}} else ", indent));
                            // Generate the inner if without extra indent
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

                // Check if this is a range expression
                let output = if let Expression::Range { start, end, .. } = iterable {
                    let start_str = self.generate_expression(start);
                    let end_str = self.generate_expression(end);
                    format!(
                        "{}for {} := {}; {} < {}; {}++ {{\n",
                        indent, var, start_str, var, end_str, var
                    )
                } else {
                    // Slice/collection iteration
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

            Statement::Match { value, arms, .. } => {
                self.generate_match_statement(value, arms)
            }

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

    // =====================================================
    // Match Generation
    // =====================================================

    fn generate_match_statement(
        &mut self,
        value: &Expression,
        arms: &[MatchArm],
    ) -> String {
        let indent = self.indent();
        let val_str = self.generate_expression(value);
        let mut output = String::new();

        // Check if this is matching on enum variants (type switch)
        // Unit enum variants are parsed as Identifier("Type::Variant"), not EnumVariant
        let is_type_switch = arms.iter().any(|arm| {
            matches!(&arm.pattern, Pattern::EnumVariant(..))
                || matches!(&arm.pattern, Pattern::Identifier(name) if name.contains("::"))
        });

        // Check if any arm has a guard — need if/else chain instead of switch
        let has_guards = arms.iter().any(|arm| arm.guard.is_some());

        if is_type_switch {
            // Go type switch
            output.push_str(&format!(
                "{}switch _v := {}.(type) {{\n",
                indent, val_str
            ));
            self.indent_level += 1;
            for arm in arms {
                output.push_str(&self.generate_match_arm_type_switch(arm));
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        } else if has_guards {
            // Guards require an if-else chain since Go switch can't have guards
            // and binding patterns would create multiple default: branches
            output.push_str(&format!("{}{{\n", indent));
            self.indent_level += 1;
            output.push_str(&format!("{}__match_val := {}\n", self.indent(), val_str));
            for (i, arm) in arms.iter().enumerate() {
                let body_str = self.generate_expression(arm.body);
                let condition = match &arm.pattern {
                    Pattern::Wildcard => "true".to_string(),
                    Pattern::Literal(lit) => {
                        let lit_str = match lit {
                            Literal::Int(n) => n.to_string(),
                            Literal::Float(f) => f.to_string(),
                            Literal::Bool(b) => b.to_string(),
                            Literal::String(s) => format!("\"{}\"", s),
                            Literal::Char(c) => format!("'{}'", c),
                        };
                        format!("__match_val == {}", lit_str)
                    }
                    Pattern::Identifier(_) => {
                        // Binding pattern: always matches, assign the variable
                        "true".to_string()
                    }
                    _ => "true".to_string(),
                };

                let full_condition = if let Some(guard) = arm.guard {
                    let guard_str = self.generate_expression(guard);
                    // For binding patterns with guards, bind the variable first
                    if let Pattern::Identifier(name) = &arm.pattern {
                        format!("func() bool {{ {} := __match_val; return {} }}()", name, guard_str)
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
                    output.push_str(&format!("{}}} else if {} {{\n", self.indent(), full_condition));
                }
                self.indent_level += 1;
                // For binding patterns, declare the variable in scope
                if let Pattern::Identifier(name) = &arm.pattern {
                    output.push_str(&format!("{}{} := __match_val\n", self.indent(), name));
                    output.push_str(&format!("{}_ = {}\n", self.indent(), name));
                }
                output.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;

                // Close if this is the last arm with "true" condition
                if i == arms.len() - 1 {
                    output.push_str(&format!("{}}}\n", self.indent()));
                }
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        } else {
            // Value-based switch
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
                    Literal::Int(n) => n.to_string(),
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
                // Binding pattern — acts like default with a variable binding
                let mut out = format!("{}default:\n", indent);
                self.indent_level += 1;
                out.push_str(&format!("{}{} := {}\n", self.indent(), name, "/* matched value */"));
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
            Pattern::EnumVariant(variant_name, _binding) => {
                // Convert Color::Red → ColorRed for Go type switch
                let go_type = self.enum_variant_to_go_type(variant_name);
                let mut out = format!("{}case {}:\n", indent, go_type);
                self.indent_level += 1;
                out.push_str(&format!("{}_ = _v\n", self.indent()));
                out.push_str(&format!("{}{}\n", self.indent(), body_str));
                self.indent_level -= 1;
                out
            }
            Pattern::Identifier(name) if name.contains("::") => {
                // Unit enum variant parsed as Identifier (e.g., "Color::Red")
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

    /// Generate a match where each arm body is wrapped in `return`
    /// Used when match is the last expression in a function with a return type
    fn generate_match_with_returns(
        &mut self,
        value: &Expression,
        arms: &[MatchArm],
    ) -> String {
        let indent = self.indent();
        let val_str = self.generate_expression(value);
        let mut output = String::new();

        // Unit enum variants are parsed as Identifier("Type::Variant"), not EnumVariant
        let is_type_switch = arms.iter().any(|arm| {
            matches!(&arm.pattern, Pattern::EnumVariant(..))
                || matches!(&arm.pattern, Pattern::Identifier(name) if name.contains("::"))
        });

        // Check if there's a wildcard/default arm (Go requires explicit return after switch)
        let has_default = arms.iter().any(|arm| matches!(&arm.pattern, Pattern::Wildcard));
        // Match guards require an if-else chain since Go switch can't have guard conditions
        let has_guards = arms.iter().any(|arm| arm.guard.is_some());

        if is_type_switch {
            output.push_str(&format!(
                "{}switch _v := {}.(type) {{\n",
                indent, val_str
            ));
            self.indent_level += 1;
            for arm in arms {
                output.push_str(&self.generate_match_arm_with_return_type_switch(arm));
            }
            self.indent_level -= 1;
            output.push_str(&format!("{}}}\n", indent));
        } else if has_guards {
            // Use if-else chain for matches with guards
            output.push_str(&format!("{}{{\n", indent));
            self.indent_level += 1;
            output.push_str(&format!("{}__match_val := {}\n", self.indent(), val_str));

            for (i, arm) in arms.iter().enumerate() {
                let body_str = self.generate_expression(arm.body);
                let condition = match &arm.pattern {
                    Pattern::Wildcard => "true".to_string(),
                    Pattern::Literal(lit) => {
                        let lit_str = match lit {
                            Literal::Int(n) => n.to_string(),
                            Literal::Float(f) => f.to_string(),
                            Literal::Bool(b) => b.to_string(),
                            Literal::String(s) => format!("\"{}\"", s),
                            Literal::Char(c) => format!("'{}'", c),
                        };
                        format!("__match_val == {}", lit_str)
                    }
                    Pattern::Identifier(_) => {
                        // Binding pattern: always matches, used with guards
                        "true".to_string()
                    }
                    _ => "true".to_string(),
                };

                // Add guard condition if present
                let full_condition = if let Some(guard) = &arm.guard {
                    // For identifier patterns, bind the variable first
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
                    // Last catch-all arm: use `else` so Go sees guaranteed return
                    let keyword = if i == 0 { "if true" } else { "} else" };
                    output.push_str(&format!("{}{} {{\n", self.indent(), keyword));
                } else {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    output.push_str(&format!("{}{} {} {{\n", self.indent(), keyword, full_condition));
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

        // Go requires a return after a switch even if all cases return.
        // Add a panic for exhaustive matches without a default/wildcard arm.
        if !has_default && !has_guards {
            output.push_str(&format!(
                "{}panic(\"unreachable match\")\n",
                indent
            ));
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
                    Literal::Int(n) => n.to_string(),
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
        out.push_str(&format!("{}_ = _v\n", self.indent()));
        out.push_str(&format!("{}return {}\n", self.indent(), body_str));
        self.indent_level -= 1;
        out
    }

    fn pattern_to_case_label(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Literal(lit) => match lit {
                Literal::Int(n) => n.to_string(),
                Literal::Float(f) => f.to_string(),
                Literal::Bool(b) => b.to_string(),
                Literal::String(s) => format!("\"{}\"", s),
                Literal::Char(c) => format!("'{}'", c),
            },
            Pattern::Identifier(name) => name.clone(),
            _ => "/* unsupported pattern */".to_string(),
        }
    }

    // =====================================================
    // Expression Generation
    // =====================================================

    fn generate_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::Literal { value, .. } => match value {
                Literal::Int(n) => n.to_string(),
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
                // Convert Rust-style paths to Go-idiomatic names:
                //   Color::Red → ColorRed{} (enum variant instantiation)
                //   Type::new  → NewType (Go constructor convention)
                if let Some((type_name, rest)) = name.split_once("::") {
                    if rest == "new" {
                        format!("New{}", Self::capitalize(type_name))
                    } else {
                        // Enum variant: Color::Red → ColorRed{}
                        // In Go, unit enum variants are empty structs, so instantiate with {}
                        format!("{}{}{{}}",
                            Self::capitalize(type_name),
                            Self::capitalize(rest))
                    }
                } else {
                    name.clone()
                }
            }

            Expression::Binary {
                left, op, right, ..
            } => {
                // Wrap child binary expressions in parens if they have lower precedence
                let left_str = if let Expression::Binary { op: ref left_op, .. } = **left {
                    if Self::op_precedence(left_op) < Self::op_precedence(op) {
                        format!("({})", self.generate_expression(left))
                    } else {
                        self.generate_expression(left)
                    }
                } else {
                    self.generate_expression(left)
                };
                let right_str = if let Expression::Binary { op: ref right_op, .. } = **right {
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
                        // Binary expressions need parens: !(a || b) not !a || b
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
                // Map Windjammer/Rust methods to Go equivalents
                match method.as_str() {
                    "len" => format!("int64(len({}))", obj_str), // .len() → len() in Go, cast to int64
                    "is_empty" => format!("len({}) == 0", obj_str),
                    "push" if args.len() == 1 => {
                        // .push(x) → append(v, x) — note: caller needs to assign result
                        format!("append({}, {})", obj_str, args[0])
                    }
                    "contains" if args.len() == 1 => {
                        // strings.Contains or manual search — use a simple helper
                        format!("/* contains */ false /* TODO */")
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
                // Ranges don't have a direct Go equivalent at expression level
                // They're handled in for-loop generation
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
                format!("func({}) interface{{}} {{\n{}return {}\n{}}}", params.join(", "), self.indent(), body_str, self.indent())
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
                // Go doesn't have tuples; generate as a struct literal or just use first element
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| self.generate_expression(e))
                    .collect();
                if elems.is_empty() {
                    "struct{}{}".to_string()
                } else {
                    // For now, generate as a comment + array
                    format!("/* tuple */ []{{{}}}", elems.join(", "))
                }
            }

            Expression::MacroInvocation {
                name, args: macro_args, ..
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
                        // Infer element type from first element; default to int64
                        let elem_type = if let Some(first) = macro_args.first() {
                            match first {
                                Expression::Literal { value: Literal::Float(_), .. } => "float64",
                                Expression::Literal { value: Literal::String(_), .. } => "string",
                                Expression::Literal { value: Literal::Bool(_), .. } => "bool",
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
                                format!(
                                    "fmt.Sprintf({}, {})",
                                    fmt_str,
                                    args[1..].join(", ")
                                )
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
                // Go doesn't have block expressions; generate as function literal call
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

    /// Extract string from a literal expression if it is one
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

    /// Generate a println call → fmt.Println or fmt.Printf
    fn generate_println_call(&mut self, arguments: &[(Option<String>, &Expression)]) -> String {
        if arguments.is_empty() {
            return "fmt.Println()".to_string();
        }

        // Check if first arg is a format string
        if let Some(fmt_str) = arguments
            .first()
            .and_then(|(_, e)| self.extract_string_literal(e))
        {
            let fmt_str = fmt_str.to_string();
            if arguments.len() == 1 && !fmt_str.contains("{}") {
                // Simple string, no formatting
                return format!("fmt.Println(\"{}\")", fmt_str);
            }

            // Convert {} to %v (Go format verb)
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
            // Non-string first argument
            let args: Vec<String> = arguments
                .iter()
                .map(|(_, arg)| self.generate_expression(arg))
                .collect();
            format!("fmt.Println({})", args.join(", "))
        }
    }

    /// Generate a print call → fmt.Print or fmt.Printf
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

    // =====================================================
    // Type Mapping
    // =====================================================

    fn type_to_go(&self, type_: &Type) -> String {
        match type_ {
            Type::Int | Type::Int32 => "int64".to_string(),
            Type::Uint => "uint64".to_string(),
            Type::Float => "float64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Custom(name) => {
                match name.as_str() {
                    "int" => "int64".to_string(),
                    "float" => "float64".to_string(),
                    "bool" => "bool".to_string(),
                    "string" => "string".to_string(),
                    "usize" => "int".to_string(),
                    "char" => "rune".to_string(),
                    _ => name.clone(), // User-defined types
                }
            }
            Type::Vec(inner) => format!("[]{}", self.type_to_go(inner)),
            Type::Array(inner, size) => format!("[{}]{}", size, self.type_to_go(inner)),
            Type::Option(inner) => format!("*{}", self.type_to_go(inner)),
            Type::Result(ok, _err) => {
                // Go typically uses (T, error) but for now just use T
                self.type_to_go(ok)
            }
            Type::Parameterized(name, args) => match name.as_str() {
                "Vec" => {
                    if let Some(inner) = args.first() {
                        format!("[]{}", self.type_to_go(inner))
                    } else {
                        "[]interface{}".to_string()
                    }
                }
                "HashMap" | "Map" => {
                    if args.len() >= 2 {
                        format!(
                            "map[{}]{}",
                            self.type_to_go(&args[0]),
                            self.type_to_go(&args[1])
                        )
                    } else {
                        "map[string]interface{}".to_string()
                    }
                }
                "Option" => {
                    if let Some(inner) = args.first() {
                        format!("*{}", self.type_to_go(inner))
                    } else {
                        "*interface{}".to_string()
                    }
                }
                _ => name.clone(),
            },
            Type::Generic(name) => {
                // Type parameter (T, U) — Go generics use `any`
                format!("{} /* generic */", name)
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                // Go doesn't have references; use the inner type
                self.type_to_go(inner)
            }
            Type::Tuple(types) => {
                if types.is_empty() {
                    String::new() // unit type → void
                } else {
                    format!("/* tuple */ {}", self.type_to_go(&types[0]))
                }
            }
            Type::TraitObject(name) => format!("{} /* interface */", name),
            Type::Associated(base, assoc) => format!("/* {}.{} */ interface{{}}", base, assoc),
            Type::Infer => "interface{}".to_string(),
            Type::FunctionPointer {
                params,
                return_type,
            } => {
                let param_types: Vec<String> = params.iter().map(|t| self.type_to_go(t)).collect();
                let ret = match return_type {
                    Some(t) => format!(" {}", self.type_to_go(t)),
                    None => String::new(),
                };
                format!("func({}){}", param_types.join(", "), ret)
            }
        }
    }

    // =====================================================
    // Pattern Helpers
    // =====================================================

    fn pattern_to_go(&self, pattern: &crate::parser::Pattern) -> String {
        match pattern {
            crate::parser::Pattern::Identifier(name) => name.clone(),
            crate::parser::Pattern::Wildcard => "_".to_string(),
            crate::parser::Pattern::Tuple(patterns) => {
                let parts: Vec<String> = patterns.iter().map(|p| self.pattern_to_go(p)).collect();
                parts.join(", ")
            }
            _ => "_".to_string(),
        }
    }
}

/// Capitalize the first letter of a string (for Go exported names)
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_backend_creation() {
        let backend = GoBackend::new();
        assert_eq!(backend.name(), "Go");
        assert_eq!(backend.target(), Target::Go);
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("x"), "X");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("Main"), "Main");
    }

    #[test]
    fn test_type_mapping() {
        let gen = GoGenerator::new();
        assert_eq!(gen.type_to_go(&Type::Int), "int64");
        assert_eq!(gen.type_to_go(&Type::Float), "float64");
        assert_eq!(gen.type_to_go(&Type::Bool), "bool");
        assert_eq!(gen.type_to_go(&Type::String), "string");
    }

    #[test]
    fn test_empty_program() {
        let program = Program { items: vec![] };
        let mut gen = GoGenerator::new();
        let code = gen.generate_program(&program);
        assert!(code.contains("package main"));
    }
}
