use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};
use windjammer::parser::{FunctionDecl, Item, Program, Type};

/// Hover provider for Windjammer code
pub struct HoverProvider {
    program: Option<Program>,
}

impl HoverProvider {
    pub fn new() -> Self {
        Self { program: None }
    }

    /// Update the parsed program
    pub fn update_program(&mut self, program: Program) {
        self.program = Some(program);
    }

    /// Get hover information at a position
    pub fn get_hover(&self, position: Position) -> Option<Hover> {
        let program = self.program.as_ref()?;

        // For now, we'll do a simple line-based search
        // TODO: Build a proper symbol table with position tracking

        let line = position.line as usize;

        // Search for functions at this line
        for item in &program.items {
            if let Some(hover) = self.check_item(item, line) {
                return Some(hover);
            }
        }

        None
    }

    fn check_item(&self, item: &Item, line: usize) -> Option<Hover> {
        match item {
            Item::Function(func) => self.check_function(func, line),
            Item::Struct(_) => {
                // TODO: Add struct hover info
                None
            }
            Item::Enum(_) => {
                // TODO: Add enum hover info
                None
            }
            Item::Trait(_) => {
                // TODO: Add trait hover info
                None
            }
            Item::Impl(_) => {
                // TODO: Add impl hover info
                None
            }
            _ => None,
        }
    }

    fn check_function(&self, func: &FunctionDecl, _line: usize) -> Option<Hover> {
        // For demonstration, show function signature
        let signature = self.format_function_signature(func);

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: signature,
            }),
            range: None,
        })
    }

    fn format_function_signature(&self, func: &FunctionDecl) -> String {
        let mut sig = String::from("```windjammer\n");

        // Decorators
        for decorator in &func.decorators {
            sig.push_str(&format!("@{}\n", decorator.name));
        }

        // Function keyword
        sig.push_str("fn ");

        // Name
        sig.push_str(&func.name);

        // Type parameters
        if !func.type_params.is_empty() {
            sig.push('<');
            for (i, tp) in func.type_params.iter().enumerate() {
                if i > 0 {
                    sig.push_str(", ");
                }
                sig.push_str(&tp.name);
                if !tp.bounds.is_empty() {
                    sig.push_str(": ");
                    sig.push_str(&tp.bounds.join(" + "));
                }
            }
            sig.push('>');
        }

        // Parameters
        sig.push('(');
        for (i, param) in func.parameters.iter().enumerate() {
            if i > 0 {
                sig.push_str(", ");
            }
            sig.push_str(&param.name);
            sig.push_str(": ");
            sig.push_str(&self.format_type(&param.type_));

            // Show inferred ownership if available
            // TODO: Get from analyzer
            // sig.push_str(" /* inferred: & */");
        }
        sig.push(')');

        // Return type
        if let Some(ret_type) = &func.return_type {
            sig.push_str(" -> ");
            sig.push_str(&self.format_type(ret_type));
        }

        sig.push_str("\n```\n");

        // Add ownership info
        sig.push_str("\n---\n\n");
        sig.push_str("**Ownership Analysis:**\n");
        for param in &func.parameters {
            sig.push_str(&format!("- `{}`: ", param.name));

            // TODO: Get actual inferred ownership from analyzer
            // For now, show placeholder
            sig.push_str("*ownership inference pending*\n");
        }

        sig
    }

    #[allow(clippy::only_used_in_recursion)]
    fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "int".to_string(),
            Type::Int32 => "int32".to_string(),
            Type::Uint => "uint".to_string(),
            Type::Float => "float".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Custom(name) => name.clone(),
            Type::Generic(name) => name.clone(),
            Type::Parameterized(name, params) => {
                let mut s = name.clone();
                s.push('<');
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&self.format_type(param));
                }
                s.push('>');
                s
            }
            Type::Associated(base, assoc) => {
                format!("{}::{}", base, assoc)
            }
            Type::TraitObject(trait_name) => {
                format!("dyn {}", trait_name)
            }
            Type::Option(inner) => {
                format!("Option<{}>", self.format_type(inner))
            }
            Type::Result(ok, err) => {
                format!(
                    "Result<{}, {}>",
                    self.format_type(ok),
                    self.format_type(err)
                )
            }
            Type::Vec(inner) => {
                format!("Vec<{}>", self.format_type(inner))
            }
            Type::Array(inner, size) => {
                format!("[{}; {}]", self.format_type(inner), size)
            }
            Type::Reference(inner) => {
                format!("&{}", self.format_type(inner))
            }
            Type::MutableReference(inner) => {
                format!("&mut {}", self.format_type(inner))
            }
            Type::Tuple(types) => {
                let mut s = "(".to_string();
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&self.format_type(ty));
                }
                s.push(')');
                s
            }
            Type::Infer => "_".to_string(),
            Type::FunctionPointer {
                params,
                return_type,
            } => {
                let param_strs: Vec<String> = params.iter().map(|t| self.format_type(t)).collect();
                if let Some(ret) = return_type {
                    format!("fn({}) -> {}", param_strs.join(", "), self.format_type(ret))
                } else {
                    format!("fn({})", param_strs.join(", "))
                }
            }
        }
    }
}

impl Default for HoverProvider {
    fn default() -> Self {
        Self::new()
    }
}
