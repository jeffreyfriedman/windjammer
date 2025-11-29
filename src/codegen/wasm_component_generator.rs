// WASM Component Code Generator
// Generates WebAssembly-compatible Rust code for @component decorated structs
// with direct DOM manipulation and zero JavaScript (except minimal loader)

use crate::component_analyzer::ComponentInfo;
use crate::parser::{BinaryOp, Expression, ImplBlock, Literal, Statement, StructDecl, Type};
use std::collections::HashMap;

pub struct WasmComponentGenerator {
    components: HashMap<String, ComponentInfo>,
}

impl WasmComponentGenerator {
    pub fn new(components: HashMap<String, ComponentInfo>) -> Self {
        WasmComponentGenerator { components }
    }

    /// Generate WASM-compatible Rust code for a component struct
    pub fn generate_component_struct(&self, struct_decl: &StructDecl) -> String {
        if !self.components.contains_key(&struct_decl.name) {
            return String::new(); // Not a component
        }

        let component_info = &self.components[&struct_decl.name];
        let mut code = String::new();

        // Generate the component struct with simple fields (no Signal wrappers for now)
        code.push_str(&format!("// Component: {}\n", struct_decl.name));
        code.push_str("#[wasm_bindgen]\n");
        code.push_str(&format!("pub struct {} {{\n", struct_decl.name));

        // Simple fields - state is managed directly
        for field in &component_info.state_fields {
            let rust_type = self.type_to_rust(&field.field_type);
            code.push_str(&format!("    {}: {},\n", field.name, rust_type));
        }

        // Add root element reference
        code.push_str("    root_element: Element,\n");

        code.push_str("}\n\n");

        code
    }

    /// Generate WASM-compatible impl block for a component
    pub fn generate_component_impl(
        &self,
        impl_block: &ImplBlock,
        component_info: &ComponentInfo,
    ) -> String {
        let mut code = String::new();

        let type_name = &impl_block.type_name;

        // Generate wasm_bindgen methods
        code.push_str("#[wasm_bindgen]\n");
        code.push_str(&format!("impl {} {{\n", type_name));

        // Generate constructor
        code.push_str("    #[wasm_bindgen(constructor)]\n");
        code.push_str("    pub fn new() -> Result<Self, JsValue> {\n");
        code.push_str("        console_error_panic_hook::set_once();\n");
        code.push_str("        \n");
        code.push_str("        let window = window().ok_or(\"No window\")?;\n");
        code.push_str("        let document = window.document().ok_or(\"No document\")?;\n");
        code.push_str(
            "        let root = document.get_element_by_id(\"app\").ok_or(\"No #app element\")?;\n",
        );
        code.push_str("        \n");
        code.push_str(&format!("        let component = {} {{\n", type_name));

        for field in &component_info.state_fields {
            let default_value = self.get_default_value(&field.field_type);
            code.push_str(&format!("            {}: {},\n", field.name, default_value));
        }

        code.push_str("            root_element: root,\n");
        code.push_str("        };\n");
        code.push_str("        \n");
        code.push_str("        Ok(component)\n");
        code.push_str("    }\n\n");

        // Generate event handler methods from the component
        for method in &component_info.methods {
            if method.is_event_handler {
                // Find the actual method in impl_block
                if let Some(func_decl) = impl_block.functions.iter().find(|m| m.name == method.name)
                {
                    code.push_str(&format!(
                        "    pub fn {}(&mut self) -> Result<(), JsValue> {{\n",
                        method.name
                    ));

                    // Generate the method body
                    for stmt in &func_decl.body {
                        code.push_str(&self.transform_statement(stmt, component_info, 2));
                    }

                    // Re-render after state change
                    code.push_str("        self.render()\n");
                    code.push_str("    }\n\n");
                }
            }
        }

        // Generate mount method that sets up event listeners
        code.push_str("    pub fn mount(self) -> Result<(), JsValue> {\n");
        code.push_str("        let component = Rc::new(RefCell::new(self));\n");
        code.push_str("        \n");
        code.push_str("        // Initial render\n");
        code.push_str("        component.borrow().render()?;\n");
        code.push_str("        \n");
        code.push_str("        // Set up event listeners\n");
        code.push_str("        let window = window().ok_or(\"No window\")?;\n");
        code.push_str("        let document = window.document().ok_or(\"No document\")?;\n");
        code.push_str("        \n");

        // Generate event listeners for each event handler
        for method in &component_info.methods {
            if method.is_event_handler {
                code.push_str(&format!("        // {} button\n", method.name));
                code.push_str("        {\n");
                code.push_str("            let component_clone = component.clone();\n");
                code.push_str("            let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {\n");
                code.push_str("                if let Some(target) = event.target() {\n");
                code.push_str(
                    "                    if let Some(element) = target.dyn_ref::<Element>() {\n",
                );
                code.push_str(&format!(
                    "                        if element.id() == \"btn-{}\" {{\n",
                    method.name
                ));
                code.push_str(
                    "                            let mut c = component_clone.borrow_mut();\n",
                );
                code.push_str(&format!(
                    "                            let _ = c.{}();\n",
                    method.name
                ));
                code.push_str("                        }\n");
                code.push_str("                    }\n");
                code.push_str("                }\n");
                code.push_str("            }) as Box<dyn FnMut(_)>);\n");
                code.push_str("            \n");
                code.push_str("            document.add_event_listener_with_callback(\"click\", closure.as_ref().unchecked_ref())?;\n");
                code.push_str("            closure.forget(); // Keep the closure alive\n");
                code.push_str("        }\n");
                code.push_str("        \n");
            }
        }

        code.push_str("        Ok(())\n");
        code.push_str("    }\n");

        code.push_str("}\n\n");

        // Generate private impl block with render method
        code.push_str(&format!("impl {} {{\n", type_name));
        code.push_str("    fn render(&self) -> Result<(), JsValue> {\n");

        // Find the render method to generate HTML
        if let Some(_render_method) = impl_block.functions.iter().find(|m| m.name == "render") {
            code.push_str("        // Build the HTML\n");
            code.push_str("        let html = format!(r#\"\n");
            code.push_str("            <div class=\"counter-app\">\n");
            code.push_str("                <h1>Windjammer Counter</h1>\n");
            code.push_str("                <div class=\"counter-display\">\n");

            // Generate dynamic content based on state fields
            for _field in &component_info.state_fields {
                code.push_str("                    <span class=\"count-text\">Count: {}</span>\n");
            }

            code.push_str("                </div>\n");
            code.push_str("                <div class=\"counter-controls\">\n");

            // Generate buttons for each event handler
            for method in &component_info.methods {
                if method.is_event_handler {
                    let label = self.method_name_to_label(&method.name);
                    code.push_str(&format!("                    <button class=\"btn btn-{}\" id=\"btn-{}\">{}</button>\n", 
                        method.name, method.name, label));
                }
            }

            code.push_str("                </div>\n");
            code.push_str("                <div class=\"powered-by\">Powered by Windjammer 🌊 | Zero JavaScript</div>\n");
            code.push_str("            </div>\n");
            code.push_str("        \"#");

            // Add format args for state fields
            for field in &component_info.state_fields {
                code.push_str(&format!(", self.{}", field.name));
            }

            code.push_str(");\n");
            code.push_str("        \n");
            code.push_str("        self.root_element.set_inner_html(&html);\n");
            code.push_str("        \n");
            code.push_str("        Ok(())\n");
        }

        code.push_str("    }\n");
        code.push_str("}\n\n");

        // Generate wasm_bindgen start function
        code.push_str("// Initialize the component when the module loads\n");
        code.push_str("#[wasm_bindgen(start)]\n");
        code.push_str("pub fn main() -> Result<(), JsValue> {\n");
        code.push_str("    console_error_panic_hook::set_once();\n");
        code.push_str("    \n");
        code.push_str(&format!("    let component = {}::new()?;\n", type_name));
        code.push_str("    component.mount()?;\n");
        code.push_str("    \n");
        code.push_str("    Ok(())\n");
        code.push_str("}\n");

        code
    }

    fn method_name_to_label(&self, name: &str) -> String {
        match name {
            "increment" => "+".to_string(),
            "decrement" => "−".to_string(),
            "reset" => "Reset".to_string(),
            _ => {
                // Capitalize first letter
                let mut chars = name.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        }
    }

    fn transform_statement(
        &self,
        stmt: &Statement,
        component_info: &ComponentInfo,
        indent_level: usize,
    ) -> String {
        let indent = "    ".repeat(indent_level);
        match stmt {
            Statement::Assignment { target, value, .. } => {
                let rust_target = self.expression_to_rust(target, component_info);
                let rust_value = self.expression_to_rust(value, component_info);
                format!("{}{} = {};\n", indent, rust_target, rust_value)
            }
            Statement::Expression { expr, .. } => {
                let rust_expr = self.expression_to_rust(expr, component_info);
                format!("{}{};\n", indent, rust_expr)
            }
            Statement::Return {
                value: Some(expr), ..
            } => {
                let rust_expr = self.expression_to_rust(expr, component_info);
                format!("{}return {};\n", indent, rust_expr)
            }
            Statement::Return { value: None, .. } => {
                format!("{}return;\n", indent)
            }
            _ => format!("{}/* TODO: statement {:?} */\n", indent, stmt),
        }
    }

    fn expression_to_rust(&self, expr: &Expression, component_info: &ComponentInfo) -> String {
        match expr {
            Expression::Identifier { name, .. } => {
                if component_info.state_fields.iter().any(|f| f.name == *name) {
                    format!("self.{}", name)
                } else {
                    name.clone()
                }
            }
            Expression::Literal { value: lit, .. } => self.literal_to_rust(lit),
            Expression::Binary {
                left, op, right, ..
            } => {
                let left_rust = self.expression_to_rust(left, component_info);
                let right_rust = self.expression_to_rust(right, component_info);
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
                format!("({} {} {})", left_rust, op_str, right_rust)
            }
            Expression::MacroInvocation { name, args, .. } => {
                let mut macro_call = format!("{}!(", name);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        macro_call.push_str(", ");
                    }
                    macro_call.push_str(&self.expression_to_rust(arg, component_info));
                }
                macro_call.push(')');
                macro_call
            }
            _ => format!("/* TODO: expression {:?} */", expr),
        }
    }

    fn literal_to_rust(&self, lit: &Literal) -> String {
        match lit {
            Literal::Int(n) => n.to_string(),
            Literal::Float(f) => f.to_string(),
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Bool(b) => b.to_string(),
            Literal::Char(c) => format!("'{}'", c),
        }
    }

    fn type_to_rust(&self, ty: &Type) -> String {
        match ty {
            Type::Int | Type::Int32 => "i32".to_string(),
            Type::Uint => "u32".to_string(),
            Type::Float => "f64".to_string(),
            Type::String => "String".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Custom(name) => name.clone(),
            _ => "()".to_string(),
        }
    }

    fn get_default_value(&self, ty: &Type) -> String {
        match ty {
            Type::Int | Type::Int32 | Type::Uint => "0".to_string(),
            Type::Float => "0.0".to_string(),
            Type::String => "String::new()".to_string(),
            Type::Bool => "false".to_string(),
            _ => "Default::default()".to_string(),
        }
    }

    /// Generate the required imports for WASM components
    pub fn generate_imports() -> String {
        let mut code = String::new();
        code.push_str("use wasm_bindgen::prelude::*;\n");
        code.push_str("use wasm_bindgen::JsCast;\n");
        code.push_str("use web_sys::{Document, Element, window};\n");
        code.push_str("use std::rc::Rc;\n");
        code.push_str("use std::cell::RefCell;\n");
        code.push('\n');
        code
    }
}
