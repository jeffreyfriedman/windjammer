// Rust code generator
use crate::parser::*;
use crate::analyzer::*;
use crate::CompilationTarget;

pub struct CodeGenerator {
    indent_level: usize,
    signature_registry: SignatureRegistry,
    in_wasm_bindgen_impl: bool,
    needs_wasm_imports: bool,
    needs_web_imports: bool,
    needs_js_imports: bool,
    target: CompilationTarget,
    is_module: bool,  // true if generating code for a reusable module (not main file)
}

impl CodeGenerator {
    pub fn new(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        CodeGenerator {
            indent_level: 0,
            signature_registry: registry,
            in_wasm_bindgen_impl: false,
            needs_wasm_imports: false,
            needs_web_imports: false,
            needs_js_imports: false,
            target,
            is_module: false,
        }
    }
    
    pub fn new_for_module(registry: SignatureRegistry, target: CompilationTarget) -> Self {
        let mut gen = Self::new(registry, target);
        gen.is_module = true;
        gen
    }
    
    fn indent(&self) -> String {
        "    ".repeat(self.indent_level)
    }
    
    /// Map Windjammer decorators to Rust attributes
    /// This abstraction layer allows us to use semantic Windjammer names
    /// while generating appropriate Rust attributes based on compilation target
    fn map_decorator(&mut self, name: &str) -> String {
        match (name, self.target) {
            ("export", CompilationTarget::Wasm) => {
                self.needs_wasm_imports = true;
                "wasm_bindgen".to_string()
            }
            ("export", CompilationTarget::Node) => {
                // Future: Node.js native modules via Neon
                "neon::export".to_string()
            }
            ("export", CompilationTarget::Python) => {
                // Future: Python bindings via PyO3
                "pyfunction".to_string()
            }
            ("export", CompilationTarget::C) => {
                // Future: C FFI
                "no_mangle".to_string()
            }
            ("test", _) => "test".to_string(),
            ("async", _) => "async".to_string(),
            // Pass through other decorators as-is
            (other, _) => other.to_string()
        }
    }
    
    fn generate_block(&mut self, stmts: &[Statement]) -> String {
        let mut output = String::new();
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == len - 1;
            if is_last && matches!(stmt, Statement::Expression(_)) {
                // Last statement is an expression - generate without semicolon (it's the return value)
                if let Statement::Expression(expr) = stmt {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_expression(expr));
                    output.push('\n');
                }
            } else {
                output.push_str(&self.generate_statement(stmt));
            }
        }
        output
    }
    
    pub fn generate_program(&mut self, program: &Program, analyzed: &[AnalyzedFunction]) -> String {
        let mut imports = String::new();
        let mut body = String::new();
        
        // Generate explicit use statements
        for item in &program.items {
            if let Item::Use(path) = item {
                imports.push_str(&self.generate_use(path));
                imports.push('\n');
            }
        }
        
        // Generate const and static declarations
        for item in &program.items {
            match item {
                Item::Const { name, type_, value } => {
                    body.push_str(&format!("const {}: {} = {};\n", 
                        name, 
                        self.type_to_rust(type_), 
                        self.generate_expression_immut(value)));
                }
                Item::Static { name, mutable, type_, value } => {
                    if *mutable {
                        body.push_str(&format!("static mut {}: {} = {};\n", 
                            name, 
                            self.type_to_rust(type_), 
                            self.generate_expression_immut(value)));
                    } else {
                        body.push_str(&format!("static {}: {} = {};\n", 
                            name, 
                            self.type_to_rust(type_), 
                            self.generate_expression_immut(value)));
                    }
                }
                _ => {}
            }
        }
        
        if !body.is_empty() {
            body.push('\n');
        }
        
        // Collect names of functions in impl blocks to avoid generating them twice
        let mut impl_methods = std::collections::HashSet::new();
        for item in &program.items {
            if let Item::Impl(impl_block) = item {
                for func in &impl_block.functions {
                    impl_methods.insert(func.name.clone());
                }
            }
        }
        
        // Generate structs, enums, and traits
        for item in &program.items {
            match item {
                Item::Struct(s) => {
                    body.push_str(&self.generate_struct(s));
                    body.push_str("\n\n");
                }
                Item::Enum(e) => {
                    body.push_str(&self.generate_enum(e));
                    body.push_str("\n\n");
                }
                Item::Trait(t) => {
                    body.push_str(&self.generate_trait(t));
                    body.push_str("\n\n");
                }
                Item::Impl(impl_block) => {
                    body.push_str(&self.generate_impl(impl_block, analyzed));
                    body.push_str("\n\n");
                }
                _ => {}
            }
        }
        
        // Generate top-level functions (skip impl methods)
        for analyzed_func in analyzed {
            if !impl_methods.contains(&analyzed_func.decl.name) {
                body.push_str(&self.generate_function(&analyzed_func));
                body.push_str("\n\n");
            }
        }
        
        // Inject implicit imports if needed
        let mut implicit_imports = String::new();
        if self.needs_wasm_imports {
            implicit_imports.push_str("use wasm_bindgen::prelude::*;\n");
        }
        if self.needs_web_imports {
            implicit_imports.push_str("use web_sys::*;\n");
        }
        if self.needs_js_imports {
            implicit_imports.push_str("use js_sys::*;\n");
        }
        
        // Combine: implicit imports + explicit imports + body
        let mut output = String::new();
        if !implicit_imports.is_empty() {
            output.push_str(&implicit_imports);
            if !imports.is_empty() {
                output.push('\n');
            }
        }
        if !imports.is_empty() {
            output.push_str(&imports);
        }
        if !output.is_empty() && !body.is_empty() {
            output.push('\n');
        }
        output.push_str(&body);
        
        output
    }
    
    fn generate_use(&self, path: &[String]) -> String {
        if path.is_empty() {
            return String::new();
        }
        
        // Skip stdlib imports - they're handled by the compiler
        // std.* modules are built-in and don't need explicit imports
        if path[0] == "std" {
            return String::new();
        }
        
        // Handle relative imports: ./utils or ../utils
        let full_path = path.join(".");
        if full_path.starts_with("./") || full_path.starts_with("../") {
            // Extract module name: ./utils -> utils, ../utils/helpers -> helpers  
            let stripped = full_path.strip_prefix("./")
                .or_else(|| full_path.strip_prefix("../"))
                .unwrap_or(&full_path);
            let module_name = stripped.split('/').last().unwrap_or(stripped);
            return format!("use {}::*;\n", module_name);
        }
        
        // Convert Windjammer's Go-style imports to Rust's glob imports
        // e.g., "use wasm_bindgen.prelude" -> "use wasm_bindgen::prelude::*;"
        format!("use {}::*;\n", path.join("::"))
    }
    
    fn generate_struct(&mut self, s: &StructDecl) -> String {
        let mut output = String::new();
        
        // Convert decorators to Rust attributes
        for decorator in &s.decorators {
            if decorator.name == "auto" {
                // Special handling for @auto decorator
                let traits = if decorator.arguments.is_empty() {
                    // Smart inference: no arguments, so infer traits based on field types
                    self.infer_derivable_traits(s)
                } else {
                    // Explicit: extract trait names from decorator arguments
                    let mut explicit_traits = Vec::new();
                    for (_key, expr) in &decorator.arguments {
                        if let Expression::Identifier(trait_name) = expr {
                            explicit_traits.push(trait_name.clone());
                        }
                    }
                    explicit_traits
                };
                
                if !traits.is_empty() {
                    output.push_str(&format!("#[derive({})]\n", traits.join(", ")));
                }
            } else {
                // Map Windjammer decorator to Rust attribute
                let rust_attr = self.map_decorator(&decorator.name);
                if decorator.arguments.is_empty() {
                    output.push_str(&format!("#[{}]\n", rust_attr));
                } else {
                    output.push_str(&format!("#[{}(", rust_attr));
                    let args: Vec<String> = decorator.arguments.iter()
                        .map(|(key, expr)| {
                            format!("{} = {}", key, self.generate_expression_immut(expr))
                        })
                        .collect();
                    output.push_str(&args.join(", "));
                    output.push_str(")]\n");
                }
            }
        }
        
        // Add struct declaration with type parameters
        output.push_str("struct ");
        output.push_str(&s.name);
        if !s.type_params.is_empty() {
            output.push('<');
            output.push_str(&s.type_params.join(", "));
            output.push('>');
        }
        output.push_str(" {\n");
        
        for field in &s.fields {
            // Generate decorators for the field (convert to Rust attributes)
            for decorator in &field.decorators {
                output.push_str(&format!("    #[{}(", decorator.name));
                let args: Vec<String> = decorator.arguments.iter()
                    .map(|(key, expr)| {
                        format!("{} = {}", key, self.generate_expression_immut(expr))
                    })
                    .collect();
                output.push_str(&args.join(", "));
                output.push_str(")]\n");
            }
            output.push_str(&format!("    {}: {},\n", field.name, self.type_to_rust(&field.field_type)));
        }
        
        output.push_str("}");
        output
    }
    
    fn generate_enum(&self, e: &EnumDecl) -> String {
        let mut output = format!("enum {} {{\n", e.name);
        
        for variant in &e.variants {
            if let Some(data) = &variant.data {
                output.push_str(&format!("    {}({}),\n", variant.name, self.type_to_rust(data)));
            } else {
                output.push_str(&format!("    {},\n", variant.name));
            }
        }
        
        output.push_str("}");
        output
    }
    
    fn generate_trait(&mut self, trait_decl: &crate::parser::TraitDecl) -> String {
        let mut output = String::from("trait ");
        output.push_str(&trait_decl.name);
        
        // Add generic parameters (these become associated types in Rust)
        if !trait_decl.generics.is_empty() {
            output.push_str(" {\n");
            self.indent_level += 1;
            
            // Convert generics to associated types
            for generic in &trait_decl.generics {
                output.push_str(&self.indent());
                output.push_str(&format!("type {};\n", generic));
            }
            output.push('\n');
        } else {
            output.push_str(" {\n");
            self.indent_level += 1;
        }
        
        // Generate trait methods
        for method in &trait_decl.methods {
            output.push_str(&self.indent());
            
            if method.is_async {
                output.push_str("async ");
            }
            
            output.push_str("fn ");
            output.push_str(&method.name);
            output.push('(');
            
            // Generate parameters
            let params: Vec<String> = method.parameters.iter().map(|param| {
                use crate::parser::OwnershipHint;
                let type_str = match &param.ownership {
                    OwnershipHint::Owned => {
                        if param.name == "self" {
                            return "self".to_string();
                        }
                        self.type_to_rust(&param.type_)
                    }
                    OwnershipHint::Ref => {
                        if param.name == "self" {
                            return "&self".to_string();
                        }
                        format!("&{}", self.type_to_rust(&param.type_))
                    }
                    OwnershipHint::Mut => {
                        if param.name == "self" {
                            return "&mut self".to_string();
                        }
                        format!("&mut {}", self.type_to_rust(&param.type_))
                    }
                    OwnershipHint::Inferred => {
                        // Default to &
                        if param.name == "self" {
                            "&self".to_string()
                        } else {
                            format!("&{}", self.type_to_rust(&param.type_))
                        }
                    }
                };
                
                format!("{}: {}", param.name, type_str)
            }).collect();
            
            output.push_str(&params.join(", "));
            output.push(')');
            
            // Return type
            if let Some(ret_type) = &method.return_type {
                output.push_str(" -> ");
                output.push_str(&self.type_to_rust(ret_type));
            }
            
            // Default implementation (if provided)
            if let Some(body) = &method.body {
                output.push_str(" {\n");
                self.indent_level += 1;
                
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                
                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push_str("}\n");
            } else {
                output.push_str(";\n");
            }
        }
        
        self.indent_level -= 1;
        output.push('}');
        output
    }
    
    fn generate_impl(&mut self, impl_block: &ImplBlock, analyzed: &[AnalyzedFunction]) -> String {
        let mut output = String::new();
        
        // Check if this impl block has @export or @wasm_bindgen decorator
        let has_wasm_export = impl_block.decorators.iter()
            .any(|d| d.name == "export" || d.name == "wasm_bindgen");
        
        // Generate decorators (map Windjammer decorators to Rust attributes)
        for decorator in &impl_block.decorators {
            let rust_attr = self.map_decorator(&decorator.name);
            output.push_str(&format!("#[{}]\n", rust_attr));
        }
        
        // Generate impl with type parameters
        output.push_str("impl");
        if !impl_block.type_params.is_empty() {
            output.push('<');
            output.push_str(&impl_block.type_params.join(", "));
            output.push('>');
        }
        output.push(' ');
        
        if let Some(trait_name) = &impl_block.trait_name {
            // Trait implementation: impl<T> Trait for Type<T>
            output.push_str(&format!("{} for {} {{\n", trait_name, impl_block.type_name));
        } else {
            // Inherent implementation: impl<T> Type<T>
            output.push_str(&format!("{} {{\n", impl_block.type_name));
        }
        
        self.indent_level += 1;
        
        // Store the wasm export flag for use in generate_function
        let old_in_wasm_impl = self.in_wasm_bindgen_impl;
        self.in_wasm_bindgen_impl = has_wasm_export;
        
        for func in &impl_block.functions {
            // Find the analyzed version of this function
            if let Some(analyzed_func) = analyzed.iter().find(|af| af.decl.name == func.name) {
                output.push_str(&self.generate_function(analyzed_func));
                output.push('\n');
            }
        }
        
        self.in_wasm_bindgen_impl = old_in_wasm_impl;
        
        self.indent_level -= 1;
        output.push('}');
        output
    }
    
    // Helper method for expressions that need to be evaluated without &mut self
    fn generate_expression_immut(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal(lit) => self.generate_literal(lit),
            Expression::Identifier(name) => name.clone(),
            _ => format!("/* expression */")
        }
    }
    
    fn generate_function(&mut self, analyzed: &AnalyzedFunction) -> String {
        let func = &analyzed.decl;
        let mut output = String::new();
        
        // Add `pub` if we're in a #[wasm_bindgen] impl block OR compiling a module
        if self.in_wasm_bindgen_impl || self.is_module {
            output.push_str("pub ");
        }
        
        output.push_str("fn ");
        output.push_str(&func.name);
        
        // Add type parameters: fn foo<T, U>(...)
        if !func.type_params.is_empty() {
            output.push('<');
            output.push_str(&func.type_params.join(", "));
            output.push('>');
        }
        
        output.push('(');
        
        let params: Vec<String> = func.parameters.iter().map(|param| {
            // Handle explicit ownership hints (self, &self, &mut self)
            let type_str = match &param.ownership {
                OwnershipHint::Owned => {
                    if param.name == "self" {
                        return "self".to_string();
                    }
                    self.type_to_rust(&param.type_)
                }
                OwnershipHint::Ref => {
                    if param.name == "self" {
                        return "&self".to_string();
                    }
                    format!("&{}", self.type_to_rust(&param.type_))
                }
                OwnershipHint::Mut => {
                    if param.name == "self" {
                        return "&mut self".to_string();
                    }
                    format!("&mut {}", self.type_to_rust(&param.type_))
                }
                OwnershipHint::Inferred => {
                    // Use analyzer's inference
                    let ownership_mode = analyzed.inferred_ownership.get(&param.name)
                        .unwrap_or(&OwnershipMode::Borrowed);
                    
                    // Override for Copy types UNLESS they're mutated
                    // Mutated parameters should be &mut even for Copy types
                    if self.is_copy_type(&param.type_) && ownership_mode != &OwnershipMode::MutBorrowed {
                        self.type_to_rust(&param.type_)
                    } else {
                        match ownership_mode {
                            OwnershipMode::Owned => self.type_to_rust(&param.type_),
                            OwnershipMode::Borrowed => {
                                // For Copy types that are only read, pass by value
                                if self.is_copy_type(&param.type_) {
                                    self.type_to_rust(&param.type_)
                                } else {
                                    format!("&{}", self.type_to_rust(&param.type_))
                                }
                            }
                            OwnershipMode::MutBorrowed => format!("&mut {}", self.type_to_rust(&param.type_)),
                        }
                    }
                }
            };
            
            // Check if this is a pattern parameter
            if let Some(pattern) = &param.pattern {
                // Generate pattern: type syntax
                format!("{}: {}", self.generate_pattern(pattern), type_str)
            } else {
                // Simple name: type syntax
                format!("{}: {}", param.name, type_str)
            }
        }).collect();
        
        output.push_str(&params.join(", "));
        output.push(')');
        
        if let Some(return_type) = &func.return_type {
            output.push_str(" -> ");
            output.push_str(&self.type_to_rust(return_type));
        }
        
        output.push_str(" {\n");
        self.indent_level += 1;
        
        output.push_str(&self.generate_block(&func.body));
        
        self.indent_level -= 1;
        output.push('}');
        
        output
    }
    
    fn type_to_rust(&self, type_: &Type) -> String {
        match type_ {
            Type::Int => "i64".to_string(),
            Type::Int32 => "i32".to_string(),
            Type::Uint => "u64".to_string(),
            Type::Float => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Custom(name) => {
                // Convert Windjammer module.Type syntax to Rust module::Type
                name.replace('.', "::")
            }
            Type::Generic(name) => name.clone(),  // Type parameter: T -> T
            Type::Parameterized(base, args) => {
                // Generic type: Vec<T> -> Vec<T>, HashMap<K, V> -> HashMap<K, V>
                format!(
                    "{}<{}>",
                    base,
                    args.iter().map(|t| self.type_to_rust(t)).collect::<Vec<_>>().join(", ")
                )
            }
            Type::Option(inner) => format!("Option<{}>", self.type_to_rust(inner)),
            Type::Result(ok, err) => format!("Result<{}, {}>", self.type_to_rust(ok), self.type_to_rust(err)),
            Type::Vec(inner) => format!("Vec<{}>", self.type_to_rust(inner)),
            Type::Reference(inner) => {
                // Special case: &[T] (slice) vs &Vec<T>
                if let Type::Vec(elem) = &**inner {
                    format!("&[{}]", self.type_to_rust(elem))
                } else {
                    format!("&{}", self.type_to_rust(inner))
                }
            }
            Type::MutableReference(inner) => {
                // Special case: &mut [T] (mutable slice) vs &mut Vec<T>
                if let Type::Vec(elem) = &**inner {
                    format!("&mut [{}]", self.type_to_rust(elem))
                } else {
                    format!("&mut {}", self.type_to_rust(inner))
                }
            }
            Type::Tuple(types) => {
                let rust_types: Vec<String> = types.iter()
                    .map(|t| self.type_to_rust(t))
                    .collect();
                format!("({})", rust_types.join(", "))
            }
        }
    }
    
    fn generate_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::Let { name, mutable, type_, value } => {
                let mut output = self.indent();
                output.push_str("let ");
                if *mutable {
                    output.push_str("mut ");
                }
                output.push_str(name);
                
                if let Some(t) = type_ {
                    output.push_str(": ");
                    output.push_str(&self.type_to_rust(t));
                }
                
                output.push_str(" = ");
                output.push_str(&self.generate_expression(value));
                output.push_str(";\n");
                output
            }
            Statement::Const { name, type_, value } => {
                let mut output = self.indent();
                output.push_str(&format!("const {}: {} = {};\n", 
                    name, 
                    self.type_to_rust(type_), 
                    self.generate_expression(value)));
                output
            }
            Statement::Static { name, mutable, type_, value } => {
                let mut output = self.indent();
                if *mutable {
                    output.push_str(&format!("static mut {}: {} = {};\n", 
                        name, 
                        self.type_to_rust(type_), 
                        self.generate_expression(value)));
                } else {
                    output.push_str(&format!("static {}: {} = {};\n", 
                        name, 
                        self.type_to_rust(type_), 
                        self.generate_expression(value)));
                }
                output
            }
            Statement::Return(expr) => {
                let mut output = self.indent();
                output.push_str("return");
                if let Some(e) = expr {
                    output.push(' ');
                    output.push_str(&self.generate_expression(e));
                }
                output.push_str(";\n");
                output
            }
            Statement::Expression(expr) => {
                let mut output = self.indent();
                output.push_str(&self.generate_expression(expr));
                output.push_str(";\n");
                output
            }
            Statement::If { condition, then_block, else_block } => {
                let mut output = self.indent();
                output.push_str("if ");
                output.push_str(&self.generate_expression(condition));
                output.push_str(" {\n");
                
                self.indent_level += 1;
                output.push_str(&self.generate_block(then_block));
                self.indent_level -= 1;
                
                output.push_str(&self.indent());
                output.push('}');
                
                if let Some(else_b) = else_block {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    output.push_str(&self.generate_block(else_b));
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }
                
                output.push('\n');
                output
            }
            Statement::Match { value, arms } => {
                let mut output = self.indent();
                output.push_str("match ");
                output.push_str(&self.generate_expression(value));
                output.push_str(" {\n");
                
                self.indent_level += 1;
                for arm in arms {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_pattern(&arm.pattern));
                    
                    // Add guard if present
                    if let Some(guard) = &arm.guard {
                        output.push_str(" if ");
                        output.push_str(&self.generate_expression(guard));
                    }
                    
                    output.push_str(" => ");
                    output.push_str(&self.generate_expression(&arm.body));
                    output.push_str(",\n");
                }
                self.indent_level -= 1;
                
                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Loop { body } => {
                let mut output = self.indent();
                output.push_str("loop {\n");
                
                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                
                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::While { condition, body } => {
                let mut output = self.indent();
                output.push_str("while ");
                output.push_str(&self.generate_expression(condition));
                output.push_str(" {\n");
                
                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                
                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::For { variable, iterable, body } => {
                let mut output = self.indent();
                output.push_str("for ");
                output.push_str(variable);
                output.push_str(" in ");
                output.push_str(&self.generate_expression(iterable));
                output.push_str(" {\n");
                
                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                
                output.push_str(&self.indent());
                output.push_str("}\n");
                output
            }
            Statement::Break => {
                let mut output = self.indent();
                output.push_str("break;\n");
                output
            }
            Statement::Continue => {
                let mut output = self.indent();
                output.push_str("continue;\n");
                output
            }
            Statement::Assignment { target, value } => {
                let mut output = self.indent();
                output.push_str(&self.generate_expression(target));
                output.push_str(" = ");
                output.push_str(&self.generate_expression(value));
                output.push_str(";\n");
                output
            }
            Statement::Go { body } => {
                // Transpile to tokio::spawn or std::thread::spawn
                let mut output = self.indent();
                output.push_str("tokio::spawn(async move {\n");
                
                self.indent_level += 1;
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                
                output.push_str(&self.indent());
                output.push_str("});\n");
                output
            }
            Statement::Defer(stmt) => {
                // Defer is not directly supported in Rust
                // We'll generate a comment for now
                let mut output = self.indent();
                output.push_str("// TODO: defer not yet implemented\n");
                output.push_str(&self.generate_statement(stmt));
                output
            }
        }
    }
    
    fn generate_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard => "_".to_string(),
            Pattern::Identifier(name) => name.clone(),
            Pattern::EnumVariant(name, binding) => {
                if let Some(b) = binding {
                    format!("{}({})", name, b)
                } else {
                    name.clone()
                }
            }
            Pattern::Literal(lit) => self.generate_literal(lit),
            Pattern::Tuple(patterns) => {
                let pattern_strs: Vec<String> = patterns.iter()
                    .map(|p| self.generate_pattern(p))
                    .collect();
                format!("({})", pattern_strs.join(", "))
            }
            Pattern::Or(patterns) => {
                let pattern_strs: Vec<String> = patterns.iter()
                    .map(|p| self.generate_pattern(p))
                    .collect();
                pattern_strs.join(" | ")
            }
        }
    }
    
    fn generate_expression_with_precedence(&mut self, expr: &Expression) -> String {
        // Wrap expressions in parentheses if they need them for proper precedence
        // when used as the object of a method call or field access
        match expr {
            Expression::Range { .. } |
            Expression::Binary { .. } |
            Expression::Closure { .. } |
            Expression::Ternary { .. } => {
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr)
        }
    }
    
    fn generate_expression(&mut self, expr: &Expression) -> String {
        match expr {
            Expression::Literal(lit) => self.generate_literal(lit),
            Expression::Identifier(name) => {
                // Convert qualified paths: std.fs.read -> std::fs::read
                // But keep simple identifiers: variable_name -> variable_name
                if name.contains('.') {
                    name.replace('.', "::")
                } else {
                    name.clone()
                }
            }
            Expression::Binary { left, op, right } => {
                // Wrap operands in parens if they have lower precedence
                let left_str = match left.as_ref() {
                    Expression::Binary { op: left_op, .. } => {
                        if self.op_precedence(left_op) < self.op_precedence(op) {
                            format!("({})", self.generate_expression(left))
                        } else {
                            self.generate_expression(left)
                        }
                    }
                    _ => self.generate_expression(left)
                };
                let right_str = match right.as_ref() {
                    Expression::Binary { op: right_op, .. } => {
                        if self.op_precedence(right_op) < self.op_precedence(op) {
                            format!("({})", self.generate_expression(right))
                        } else {
                            self.generate_expression(right)
                        }
                    }
                    _ => self.generate_expression(right)
                };
                let op_str = self.binary_op_to_rust(op);
                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::Ternary { condition, true_expr, false_expr } => {
                let cond_str = self.generate_expression(condition);
                let true_str = self.generate_expression(true_expr);
                let false_str = self.generate_expression(false_expr);
                format!("if {} {{ {} }} else {{ {} }}", cond_str, true_str, false_str)
            }
            Expression::Unary { op, operand } => {
                let operand_str = self.generate_expression(operand);
                let op_str = self.unary_op_to_rust(op);
                format!("{}{}", op_str, operand_str)
            }
            Expression::Call { function, arguments } => {
                // Extract function name for signature lookup
                let func_name = self.extract_function_name(function);
                let func_str = self.generate_expression(function);
                
                // Look up signature and clone it to avoid borrow conflicts
                let signature = self.signature_registry.get_signature(&func_name).cloned();
                
                let args: Vec<String> = arguments.iter().enumerate()
                    .map(|(i, (_label, arg))| {
                        let arg_str = self.generate_expression(arg);
                        
                        // Check if this parameter expects a borrow
                        if let Some(ref sig) = signature {
                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed => {
                                        // Insert & if not already a reference
                                        if !self.is_reference_expression(arg) {
                                            return format!("&{}", arg_str);
                                        }
                                    }
                                    OwnershipMode::MutBorrowed => {
                                        // Insert &mut if not already a reference
                                        if !self.is_reference_expression(arg) {
                                            return format!("&mut {}", arg_str);
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // No change needed
                                    }
                                }
                            }
                        }
                        
                        arg_str
                    })
                    .collect();
                format!("{}({})", func_str, args.join(", "))
            }
            Expression::MethodCall { object, method, arguments } => {
                let obj_str = self.generate_expression_with_precedence(object);
                let args: Vec<String> = arguments.iter()
                    .map(|(_label, arg)| self.generate_expression(arg))
                    .collect();
                
                // Determine separator: :: for static calls, . for instance methods
                // - FunctionCall result: instance, use .
                // - Identifier/FieldAccess in module context: static, use ::
                // - Everything else: instance method, use .
                let separator = match **object {
                    Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
                    Expression::Identifier(_) | Expression::FieldAccess { .. } if self.is_module => "::", // Static call in stdlib
                    Expression::Identifier(_) | Expression::FieldAccess { .. } => "::", // Module/type call
                    _ => "."  // Instance method
                };
                
                format!("{}{}{}({})", obj_str, separator, method, args.join(", "))
            }
            Expression::FieldAccess { object, field } => {
                let obj_str = self.generate_expression_with_precedence(object);
                
                // In module context (stdlib), always use :: for Rust paths
                // Otherwise, use :: for module/type paths and . for field access
                let separator = if self.is_module {
                    "::"
                } else {
                    match **object {
                        Expression::Identifier(ref name) if name.contains('.') || (!name.is_empty() && name.chars().next().unwrap().is_uppercase()) => "::",
                        Expression::FieldAccess { .. } => "::", // Chained path
                        _ => "."  // Actual field access
                    }
                };
                
                format!("{}{}{}", obj_str, separator, field)
            }
            Expression::StructLiteral { name, fields } => {
                let field_str: Vec<String> = fields.iter()
                    .map(|(field_name, expr)| {
                        format!("{}: {}", field_name, self.generate_expression(expr))
                    })
                    .collect();
                format!("{} {{ {} }}", name, field_str.join(", "))
            }
            Expression::TryOp(inner) => {
                format!("{}?", self.generate_expression(inner))
            }
            Expression::Await(inner) => {
                format!("{}.await", self.generate_expression(inner))
            }
            Expression::ChannelSend { channel, value } => {
                let ch_str = self.generate_expression(channel);
                let val_str = self.generate_expression(value);
                format!("{}.send({})", ch_str, val_str)
            }
            Expression::ChannelRecv(channel) => {
                let ch_str = self.generate_expression(channel);
                format!("{}.recv()", ch_str)
            }
            Expression::Range { start, end, inclusive } => {
                let start_str = self.generate_expression(start);
                let end_str = self.generate_expression(end);
                if *inclusive {
                    format!("{}..={}", start_str, end_str)
                } else {
                    format!("{}..{}", start_str, end_str)
                }
            }
            Expression::Closure { parameters, body } => {
                let params = parameters.join(", ");
                let body_str = self.generate_expression(body);
                format!("|{}| {}", params, body_str)
            }
            Expression::Index { object, index } => {
                let obj_str = self.generate_expression(object);
                let idx_str = self.generate_expression(index);
                format!("{}[{}]", obj_str, idx_str)
            }
            Expression::Tuple(exprs) => {
                let expr_strs: Vec<String> = exprs.iter()
                    .map(|e| self.generate_expression(e))
                    .collect();
                format!("({})", expr_strs.join(", "))
            }
            Expression::MacroInvocation { name, args, delimiter } => {
                use crate::parser::MacroDelimiter;
                
                // Special case: if this is println!/eprintln!/print!/eprint! and first arg is format!, flatten it
                let should_flatten = (name == "println" || name == "eprintln" || name == "print" || name == "eprint")
                    && !args.is_empty()
                    && matches!(&args[0], Expression::MacroInvocation { name: macro_name, .. } if macro_name == "format");
                
                let arg_strs: Vec<String> = if should_flatten {
                    // Flatten format! macro arguments into the print macro
                    if let Expression::MacroInvocation { args: format_args, .. } = &args[0] {
                        format_args.iter()
                            .map(|e| self.generate_expression(e))
                            .collect()
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                } else {
                    args.iter().map(|e| self.generate_expression(e)).collect()
                };
                
                let (open, close) = match delimiter {
                    MacroDelimiter::Parens => ("(", ")"),
                    MacroDelimiter::Brackets => ("[", "]"),
                    MacroDelimiter::Braces => ("{", "}"),
                };
                format!("{}!{}{}{}", name, open, arg_strs.join(", "), close)
            }
            Expression::Cast { expr, type_ } => {
                // Add parentheses around binary expressions for correct precedence
                let expr_str = match &**expr {
                    Expression::Binary { .. } => {
                        format!("({})", self.generate_expression(expr))
                    }
                    _ => self.generate_expression(expr)
                };
                let type_str = self.type_to_rust(type_);
                format!("{} as {}", expr_str, type_str)
            }
            Expression::Block(stmts) => {
                // Special case: if the block contains only a match statement, generate it as a match expression
                if stmts.len() == 1 {
                    if let Statement::Match { value, arms } = &stmts[0] {
                        let mut output = String::from("match ");
                        output.push_str(&self.generate_expression(value));
                        output.push_str(" {\n");
                        
                        self.indent_level += 1;
                        for arm in arms {
                            output.push_str(&self.indent());
                            output.push_str(&self.generate_pattern(&arm.pattern));
                            
                            // Add guard if present
                            if let Some(guard) = &arm.guard {
                                output.push_str(" if ");
                                output.push_str(&self.generate_expression(guard));
                            }
                            
                            output.push_str(" => ");
                            output.push_str(&self.generate_expression(&arm.body));
                            output.push_str(",\n");
                        }
                        self.indent_level -= 1;
                        
                        output.push_str(&self.indent());
                        output.push('}');
                        return output;
                    }
                }
                
                // Regular block
                let mut output = String::from("{\n");
                self.indent_level += 1;
                for stmt in stmts {
                    output.push_str(&self.generate_statement(stmt));
                }
                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push('}');
                output
            }
        }
    }
    
    fn generate_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => {
                let s = f.to_string();
                // Ensure float literals always have a decimal point
                if !s.contains('.') && !s.contains('e') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Char(c) => {
                // Escape special characters
                match c {
                    '\n' => "'\\n'".to_string(),
                    '\t' => "'\\t'".to_string(),
                    '\r' => "'\\r'".to_string(),
                    '\\' => "'\\\\'".to_string(),
                    '\'' => "'\\''".to_string(),
                    '\0' => "'\\0'".to_string(),
                    _ => format!("'{}'", c),
                }
            }
            Literal::Bool(b) => b.to_string(),
        }
    }
    
    fn binary_op_to_rust(&self, op: &BinaryOp) -> &str {
        match op {
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
        }
    }
    
    fn op_precedence(&self, op: &BinaryOp) -> i32 {
        match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::Ne => 3,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 4,
            BinaryOp::Add | BinaryOp::Sub => 5,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 6,
        }
    }
    
    fn unary_op_to_rust(&self, op: &UnaryOp) -> &str {
        match op {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "-",
            UnaryOp::Ref => "&",
            UnaryOp::Deref => "*",
        }
    }
    
    fn extract_function_name(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::FieldAccess { field, .. } => field.clone(),
            _ => String::new(), // Can't determine function name
        }
    }
    
    fn is_reference_expression(&self, expr: &Expression) -> bool {
        matches!(
            expr,
            Expression::Unary { op: UnaryOp::Ref, .. }
        )
    }
    
    fn infer_derivable_traits(&self, struct_: &StructDecl) -> Vec<String> {
        let mut traits = vec![
            "Debug".to_string(),
            "Clone".to_string(),
        ]; // Always safe to derive
        
        // Check if all fields are Copy
        if self.all_fields_are_copy(&struct_.fields) {
            traits.push("Copy".to_string());
        }
        
        // Check if all fields are PartialEq/Eq
        if self.all_fields_are_comparable(&struct_.fields) {
            traits.push("PartialEq".to_string());
            traits.push("Eq".to_string());
            
            // If Eq, also check for Hash
            if self.all_fields_are_hashable(&struct_.fields) {
                traits.push("Hash".to_string());
            }
        }
        
        // Check if all fields have Default
        if self.all_fields_have_default(&struct_.fields) {
            traits.push("Default".to_string());
        }
        
        traits
    }
    
    fn all_fields_are_copy(&self, fields: &[crate::parser::StructField]) -> bool {
        fields.iter().all(|field| self.is_copy_type(&field.field_type))
    }
    
    fn all_fields_are_comparable(&self, fields: &[crate::parser::StructField]) -> bool {
        fields.iter().all(|field| self.is_comparable_type(&field.field_type))
    }
    
    fn all_fields_are_hashable(&self, fields: &[crate::parser::StructField]) -> bool {
        fields.iter().all(|field| self.is_hashable_type(&field.field_type))
    }
    
    fn all_fields_have_default(&self, fields: &[crate::parser::StructField]) -> bool {
        fields.iter().all(|field| self.has_default(&field.field_type))
    }
    
    fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::Reference(_) => true,  // References are Copy
            Type::MutableReference(_) => false,  // Mutable references are not Copy
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            Type::Custom(name) => {
                // Recognize common Rust primitive types by name
                matches!(name.as_str(), 
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
                    "f32" | "f64" | "bool" | "char"
                )
            }
            _ => false,  // String, Vec, Option, Result, other Custom types are not Copy
        }
    }
    
    fn is_comparable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool | Type::String => true,
            Type::Reference(inner) | Type::MutableReference(inner) => self.is_comparable_type(inner),
            Type::Tuple(types) => types.iter().all(|t| self.is_comparable_type(t)),
            Type::Option(inner) => self.is_comparable_type(inner),
            Type::Result(ok, err) => self.is_comparable_type(ok) && self.is_comparable_type(err),
            _ => false,  // Vec is not Eq (only PartialEq), Custom types unknown
        }
    }
    
    fn is_hashable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Bool | Type::String => true,
            Type::Float => false,  // Floats are not Hash
            Type::Reference(inner) => self.is_hashable_type(inner),
            Type::MutableReference(_) => false,
            Type::Tuple(types) => types.iter().all(|t| self.is_hashable_type(t)),
            Type::Vec(_) => false,  // Vec is not Hash
            Type::Option(inner) => self.is_hashable_type(inner),
            _ => false,  // Result, Custom types - assume not Hash
        }
    }
    
    fn has_default(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::String => true,  // String has Default ("")
            Type::Vec(_) => true,  // Vec has Default (empty vec)
            Type::Option(_) => true,  // Option has Default (None)
            Type::Tuple(types) => types.iter().all(|t| self.has_default(t)),
            _ => false,  // Refs don't have Default, Result/Custom types unknown
        }
    }
}

