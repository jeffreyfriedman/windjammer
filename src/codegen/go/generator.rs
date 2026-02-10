//! Go code generator
//!
//! Generates idiomatic Go source code from the Windjammer AST.

use crate::codegen::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::parser::{
    BinaryOp, CompoundOp, Expression, FunctionDecl, Item, Literal, Program, Statement, Type,
    UnaryOp,
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
}

impl GoGenerator {
    fn new() -> Self {
        Self {
            indent_level: 0,
            needs_fmt_import: false,
            needs_math_import: false,
            declared_structs: Vec::new(),
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
                Item::Impl { block, .. } => {
                    for method in &block.functions {
                        items_code.push(self.generate_method(&block.type_name, method));
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

        self.indent_level += 1;
        let has_return_type = func.return_type.is_some();
        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == body_len - 1;
            // In Go, the last expression must be an explicit return
            if is_last && has_return_type {
                if let Statement::Expression { expr, .. } = stmt {
                    let indent = self.indent();
                    let expr_str = self.generate_expression(expr);
                    output.push_str(&format!("{}return {}\n", indent, expr_str));
                    continue;
                }
            }
            output.push_str(&self.generate_statement(stmt));
        }
        self.indent_level -= 1;

        output.push_str("}\n");
        output
    }

    fn generate_method(&mut self, type_name: &str, func: &FunctionDecl) -> String {
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

        self.indent_level += 1;

        // Generate body, replacing `self.` with receiver
        let has_return_type = func.return_type.is_some();
        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == body_len - 1;
            // In Go, the last expression must be an explicit return
            if is_last && has_return_type {
                if let Statement::Expression { expr, .. } = stmt {
                    let indent = self.indent();
                    let expr_str = self.generate_expression(expr);
                    let expr_str = expr_str.replace("self.", &format!("{}.", receiver_name));
                    output.push_str(&format!("{}return {}\n", indent, expr_str));
                    continue;
                }
            }
            let code = self.generate_statement(stmt);
            let code = code.replace("self.", &format!("{}.", receiver_name));
            output.push_str(&code);
        }

        self.indent_level -= 1;
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

                if *mutable {
                    // Go uses `var` for mutable variables (or just :=)
                    format!("{}var {} = {}\n", indent, var_name, value_str)
                } else {
                    // Immutable: still use := in Go (Go doesn't have const for runtime values)
                    // We could use `const` for compile-time constants but that's limited
                    let _ = type_; // type annotation ignored; Go infers
                    format!("{}{} := {}\n", indent, var_name, value_str)
                }
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

            Expression::Identifier { name, .. } => name.clone(),

            Expression::Binary {
                left, op, right, ..
            } => {
                let left_str = self.generate_expression(left);
                let right_str = self.generate_expression(right);
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
                    UnaryOp::Not => format!("!{}", operand_str),
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
                let method_name = capitalize_first(method);
                let args: Vec<String> = arguments
                    .iter()
                    .map(|(_, arg)| self.generate_expression(arg))
                    .collect();
                format!("{}.{}({})", obj_str, method_name, args.join(", "))
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
