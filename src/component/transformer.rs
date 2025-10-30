//! Transform reactive variables to signals

use super::analyzer::DependencyInfo;
use super::ast::*;
use crate::parser::{Expression, Pattern, Statement, Type};
use anyhow::Result;
use std::collections::HashMap;

/// Transforms reactive variables to Signal<T>
pub struct SignalTransformer {
    _deps: DependencyInfo,
    signal_vars: HashMap<String, String>, // var_name -> Signal<Type>
}

#[allow(dead_code)]
#[allow(clippy::useless_format)]
#[allow(clippy::single_char_add_str)]
#[allow(clippy::only_used_in_recursion)]
impl SignalTransformer {
    /// Transform a component to use signals
    pub fn transform(
        component: &ComponentFile,
        deps: &DependencyInfo,
    ) -> Result<TransformedComponent> {
        let mut transformer = Self {
            _deps: deps.clone(),
            signal_vars: HashMap::new(),
        };

        match &component.style {
            ComponentStyle::Minimal(minimal) => {
                transformer.transform_minimal(minimal, component.name.as_deref())
            }
            ComponentStyle::Advanced(advanced) => transformer.transform_advanced(advanced),
        }
    }

    fn transform_minimal(
        &mut self,
        component: &MinimalComponent,
        name: Option<&str>,
    ) -> Result<TransformedComponent> {
        let component_name = name.unwrap_or("Component").to_string();
        let mut transformed = TransformedComponent::new();
        transformed.name = component_name.clone();

        // Transform state to signal fields
        for state in &component.state {
            let rust_type = self.type_to_rust(&state.type_);
            let signal_type = format!("Signal<{}>", rust_type);
            let initial_value_rust = self.expression_to_rust(&state.initial_value);
            let initial_value = format!("Signal::new({})", initial_value_rust);

            self.signal_vars
                .insert(state.name.clone(), rust_type.clone());

            transformed.fields.push(SignalField {
                name: state.name.clone(),
                type_: signal_type,
                initial_value,
            });
        }

        // Transform computed to computed fields
        for computed in &component.computed {
            let rust_type = if let Some(ty) = &computed.type_ {
                self.type_to_rust(ty)
            } else {
                "i32".to_string() // Default type
            };
            let signal_type = format!("Computed<{}>", rust_type);
            let expr_rust = self.expression_to_rust(&computed.expression);
            let initial_value = format!("Computed::new(move || {})", expr_rust);

            transformed.fields.push(SignalField {
                name: computed.name.clone(),
                type_: signal_type,
                initial_value,
            });
        }

        // Add constructor
        let constructor_body = self.generate_constructor(&transformed.fields);
        transformed.methods.push(TransformedMethod {
            name: "new".to_string(),
            params: vec![],
            return_type: Some(component_name.clone()),
            body: constructor_body,
        });

        // Transform functions to methods
        for func in &component.functions {
            let body = self.transform_statements(&func.body);
            transformed.methods.push(TransformedMethod {
                name: func.name.clone(),
                params: vec![], // TODO: Transform parameters
                return_type: func.return_type.as_ref().map(|t| self.type_to_rust(t)),
                body,
            });
        }

        // Generate mount method from view (direct DOM manipulation)
        let mount_body = self.generate_mount(&component.view, &component_name);
        transformed.methods.push(TransformedMethod {
            name: "mount".to_string(),
            params: vec!["parent: &Element".to_string()],
            return_type: Some("Result<(), JsValue>".to_string()),
            body: mount_body,
        });

        // Generate mount method for lifecycle hooks
        if !component.lifecycle.is_empty() {
            let mount_body = self.generate_lifecycle_methods(&component.lifecycle);
            transformed.methods.push(TransformedMethod {
                name: "mount".to_string(),
                params: vec![],
                return_type: None,
                body: mount_body,
            });
        }

        Ok(transformed)
    }

    fn transform_advanced(
        &mut self,
        component: &AdvancedComponent,
    ) -> Result<TransformedComponent> {
        let mut transformed = TransformedComponent::new();
        transformed.name = component.struct_decl.name.clone();

        // Transform struct fields to signals
        for field in &component.struct_decl.fields {
            let rust_type = self.type_to_rust(&field.type_);
            let signal_type = format!("Signal<{}>", rust_type);
            let initial_value = if let Some(default) = &field.default {
                format!("Signal::new({})", self.expression_to_rust(default))
            } else {
                format!("Signal::new(Default::default())")
            };

            self.signal_vars.insert(field.name.clone(), rust_type);

            transformed.fields.push(SignalField {
                name: field.name.clone(),
                type_: signal_type,
                initial_value,
            });
        }

        // Transform methods
        for method in &component.impl_block.methods {
            let body = self.transform_statements(&method.function.body);
            transformed.methods.push(TransformedMethod {
                name: method.function.name.clone(),
                params: vec![], // TODO: Transform parameters
                return_type: method
                    .function
                    .return_type
                    .as_ref()
                    .map(|t| self.type_to_rust(t)),
                body,
            });
        }

        Ok(transformed)
    }

    fn generate_constructor(&self, fields: &[SignalField]) -> String {
        let mut body = String::new();
        body.push_str("Self {\n");
        for field in fields {
            body.push_str(&format!(
                "            {}: {},\n",
                field.name, field.initial_value
            ));
        }
        body.push_str("        }");
        body
    }

    fn generate_render(&mut self, view: &ViewBlock) -> String {
        self.view_node_to_rust(&view.root)
    }

    fn generate_mount(&mut self, view: &ViewBlock, _component_name: &str) -> String {
        let mut code = String::new();
        code.push_str("let document = component_runtime::document()?;\n");
        code.push_str(&self.generate_dom_node(&view.root, "parent", 0));
        code.push_str("Ok(())");
        code
    }

    fn generate_dom_node(&mut self, node: &ViewNode, parent_var: &str, indent: usize) -> String {
        match node {
            ViewNode::Element(elem) => self.generate_dom_element(elem, parent_var, indent),
            ViewNode::Text(text) => self.generate_dom_text(text, parent_var, indent),
            ViewNode::If(if_node) => self.generate_dom_if(if_node, parent_var, indent),
            ViewNode::For(for_node) => self.generate_dom_for(for_node, parent_var, indent),
            ViewNode::Component(_comp) => {
                // TODO: Component composition
                String::new()
            }
        }
    }

    fn generate_dom_element(
        &mut self,
        elem: &ElementNode,
        parent_var: &str,
        indent: usize,
    ) -> String {
        let indent_str = "    ".repeat(indent);
        let mut code = String::new();

        // Create element
        let elem_var = format!("elem_{}", self.next_var_id());
        code.push_str(&format!(
            "{}let {} = document.create_element(\"{}\")?;\n",
            indent_str, elem_var, elem.tag
        ));

        // Set attributes
        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value } => {
                    if name == "class" {
                        code.push_str(&format!(
                            "{}{}.set_attribute(\"class\", \"{}\")?;\n",
                            indent_str, elem_var, value
                        ));
                    } else {
                        code.push_str(&format!(
                            "{}{}.set_attribute(\"{}\", \"{}\")?;\n",
                            indent_str, elem_var, name, value
                        ));
                    }
                }
                Attribute::Dynamic { name, value } => {
                    let value_rust = self.expression_to_rust(value);
                    code.push_str(&format!(
                        "{}{}.set_attribute(\"{}\", &{})?;\n",
                        indent_str, elem_var, name, value_rust
                    ));
                }
                Attribute::Event { name, handler } => {
                    if let Some(event_name) = name.strip_prefix("on_") {
                        let handler_name = match handler {
                            crate::parser::Expression::Identifier(name) => name.clone(),
                            _ => self.expression_to_rust(handler),
                        };

                        // Generate event listener with closure
                        code.push_str(&format!(
                            "{}// Event handler for {}\n",
                            indent_str, event_name
                        ));
                        let closure_id = self.next_var_id();
                        code.push_str(&format!("{}let closure_{} = {{\n", indent_str, closure_id));
                        code.push_str(&format!(
                            "{}    let mut self_clone = self.clone();\n",
                            indent_str
                        ));
                        code.push_str(&format!(
                            "{}    Closure::wrap(Box::new(move |_event: web_sys::Event| {{\n",
                            indent_str
                        ));
                        code.push_str(&format!(
                            "{}        self_clone.{}();\n",
                            indent_str, handler_name
                        ));
                        code.push_str(&format!(
                            "{}    }}) as Box<dyn FnMut(web_sys::Event)>)\n",
                            indent_str
                        ));
                        code.push_str(&format!("{}}};\n", indent_str));
                        code.push_str(&format!("{}{}.add_event_listener_with_callback(\"{}\", closure_{}.as_ref().unchecked_ref())?;\n",
                            indent_str, elem_var, event_name, closure_id));
                        code.push_str(&format!("{}closure_{}.forget();\n", indent_str, closure_id));
                    }
                }
            }
        }

        // Add children
        for child in &elem.children {
            code.push_str(&self.generate_dom_node(child, &elem_var, indent));
        }

        // Append to parent
        code.push_str(&format!(
            "{}{}.append_child(&{})?;\n",
            indent_str, parent_var, elem_var
        ));

        code
    }

    fn generate_dom_text(&mut self, text: &TextNode, parent_var: &str, indent: usize) -> String {
        let indent_str = "    ".repeat(indent);
        let mut code = String::new();

        // Check if this is purely static text
        let is_static = text.parts.iter().all(|p| matches!(p, TextPart::Static(_)));

        if is_static && text.parts.len() == 1 {
            if let TextPart::Static(s) = &text.parts[0] {
                let text_var = format!("text_{}", self.next_var_id());
                code.push_str(&format!(
                    "{}let {} = document.create_text_node(\"{}\");\n",
                    indent_str, text_var, s
                ));
                code.push_str(&format!(
                    "{}{}.append_child(&{})?;\n",
                    indent_str, parent_var, text_var
                ));
                return code;
            }
        }

        // Dynamic text - needs reactivity
        let text_var = format!("text_{}", self.next_var_id());

        // Build the format string and args
        let mut format_str = String::new();
        let mut args = Vec::new();
        let mut has_dynamic = false;

        for part in &text.parts {
            match part {
                TextPart::Static(s) => format_str.push_str(s),
                TextPart::Dynamic(expr) => {
                    format_str.push_str("{}");
                    args.push(self.expression_to_rust(expr));
                    has_dynamic = true;
                }
            }
        }

        if !has_dynamic {
            // No dynamic parts, just static
            code.push_str(&format!(
                "{}let {} = document.create_text_node(\"{}\");\n",
                indent_str, text_var, format_str
            ));
            code.push_str(&format!(
                "{}{}.append_child(&{})?;\n",
                indent_str, parent_var, text_var
            ));
            return code;
        }

        // Create initial text node
        let initial_value = if args.is_empty() {
            format!("\"{}\"", format_str)
        } else {
            let mut fmt = format!("format!(\"{}\"", format_str);
            for arg in &args {
                fmt.push_str(", ");
                fmt.push_str(arg);
            }
            fmt.push(')');
            fmt
        };

        code.push_str(&format!(
            "{}let {} = document.create_text_node(&{});\n",
            indent_str, text_var, initial_value
        ));
        code.push_str(&format!(
            "{}{}.append_child(&{})?;\n",
            indent_str, parent_var, text_var
        ));

        // Create effect to update text node when signals change
        code.push_str(&format!(
            "{}// Create effect for reactive text update\n",
            indent_str
        ));
        code.push_str(&format!("{}{{\n", indent_str));
        code.push_str(&format!(
            "{}    let text_node = {}.clone();\n",
            indent_str, text_var
        ));

        // Clone signals for the effect closure
        for (i, arg) in args.iter().enumerate() {
            // Only clone if it looks like a signal access (contains .get())
            if arg.contains(".get()") {
                // Extract signal name from "self.field.get()" -> "field"
                let signal_name = if arg.starts_with("self.") {
                    arg.strip_prefix("self.")
                        .and_then(|s| s.split('.').next())
                        .unwrap_or(arg)
                } else {
                    arg.split('.').next().unwrap_or(arg)
                };
                code.push_str(&format!(
                    "{}    let signal_{} = self.{}.clone();\n",
                    indent_str, i, signal_name
                ));
            }
        }

        code.push_str(&format!("{}    Effect::new(move || {{\n", indent_str));

        // Build the format expression inside the effect
        let mut effect_fmt = format!("format!(\"{}\"", format_str);
        for (i, arg) in args.iter().enumerate() {
            effect_fmt.push_str(", ");
            if arg.contains(".get()") {
                // Replace self.field.get() with signal_i.get()
                effect_fmt.push_str(&format!("signal_{}.get()", i));
            } else {
                effect_fmt.push_str(arg);
            }
        }
        effect_fmt.push(')');

        code.push_str(&format!(
            "{}        let new_text = {};\n",
            indent_str, effect_fmt
        ));
        code.push_str(&format!(
            "{}        text_node.set_node_value(Some(&new_text));\n",
            indent_str
        ));
        code.push_str(&format!("{}    }});\n", indent_str));
        code.push_str(&format!("{}}}\n", indent_str));

        code
    }

    fn generate_dom_if(&mut self, if_node: &IfNode, parent_var: &str, indent: usize) -> String {
        let indent_str = "    ".repeat(indent);
        let mut code = String::new();

        let condition_rust = self.expression_to_rust(&if_node.condition);
        code.push_str(&format!("{}if {} {{\n", indent_str, condition_rust));

        for child in &if_node.then_branch {
            code.push_str(&self.generate_dom_node(child, parent_var, indent + 1));
        }

        if let Some(else_branch) = &if_node.else_branch {
            code.push_str(&format!("{}}} else {{\n", indent_str));
            for child in else_branch {
                code.push_str(&self.generate_dom_node(child, parent_var, indent + 1));
            }
        }

        code.push_str(&format!("{}}}\n", indent_str));
        code
    }

    fn generate_dom_for(&mut self, for_node: &ForNode, parent_var: &str, indent: usize) -> String {
        let indent_str = "    ".repeat(indent);
        let mut code = String::new();

        let iterable_rust = self.expression_to_rust(&for_node.iterable);
        // TODO: Handle pattern properly, for now just use a simple identifier
        let pattern_str = "item"; // Simplified for now
        code.push_str(&format!(
            "{}for {} in {} {{\n",
            indent_str, pattern_str, iterable_rust
        ));

        for child in &for_node.body {
            code.push_str(&self.generate_dom_node(child, parent_var, indent + 1));
        }

        code.push_str(&format!("{}}}\n", indent_str));
        code
    }

    fn next_var_id(&mut self) -> usize {
        static mut COUNTER: usize = 0;
        unsafe {
            COUNTER += 1;
            COUNTER
        }
    }

    fn view_node_to_rust(&mut self, node: &ViewNode) -> String {
        match node {
            ViewNode::Element(elem) => self.element_to_rust(elem),
            ViewNode::Text(text) => self.text_to_rust(text),
            ViewNode::If(if_node) => self.if_node_to_rust(if_node),
            ViewNode::For(for_node) => self.for_node_to_rust(for_node),
            ViewNode::Component(comp) => self.component_node_to_rust(comp),
        }
    }

    fn element_to_rust(&mut self, elem: &ElementNode) -> String {
        let mut code = format!("VNode::element(\"{}\", vec![", elem.tag);

        // Attributes
        for (i, attr) in elem.attributes.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            match attr {
                Attribute::Static { name, value } => {
                    code.push_str(&format!("(\"{}\", VAttr::Static(\"{}\"))", name, value));
                }
                Attribute::Dynamic { name, value } => {
                    let value_rust = self.expression_to_rust(value);
                    code.push_str(&format!("(\"{}\", VAttr::Dynamic({}))", name, value_rust));
                }
                Attribute::Event { name, handler } => {
                    // Generate a closure that captures self and calls the handler method
                    // For now, we'll generate a placeholder that needs to be fixed
                    // The proper solution requires capturing self in the closure
                    let handler_name = match handler {
                        crate::parser::Expression::Identifier(name) => name.clone(),
                        _ => self.expression_to_rust(handler),
                    };
                    // Generate: Rc::new(RefCell::new(move || self.method_name()))
                    // But we can't capture self in the render method, so we need a different approach
                    // For now, generate a placeholder
                    code.push_str(&format!(
                        "(\"{}\", VAttr::Event(Rc::new(RefCell::new(move || {{ /* {} */ }}))))",
                        name, handler_name
                    ));
                }
            }
        }

        code.push_str("], vec![");

        // Children
        for (i, child) in elem.children.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&self.view_node_to_rust(child));
        }

        code.push_str("])");
        code
    }

    fn text_to_rust(&mut self, text: &TextNode) -> String {
        if text.parts.len() == 1 {
            if let TextPart::Static(s) = &text.parts[0] {
                return format!("VNode::text(\"{}\")", s);
            }
        }

        // Multiple parts or dynamic parts
        let mut code = String::from("VNode::text(&format!(\"");
        let mut args = Vec::new();

        for part in &text.parts {
            match part {
                TextPart::Static(s) => code.push_str(s),
                TextPart::Dynamic(expr) => {
                    code.push_str("{}");
                    args.push(self.expression_to_rust(expr));
                }
            }
        }

        code.push_str("\"");
        for arg in args {
            code.push_str(", ");
            code.push_str(&arg);
        }
        code.push_str("))");
        code
    }

    fn if_node_to_rust(&mut self, if_node: &IfNode) -> String {
        let condition = self.expression_to_rust(&if_node.condition);
        let then_nodes: Vec<String> = if_node
            .then_branch
            .iter()
            .map(|n| self.view_node_to_rust(n))
            .collect();
        let else_nodes: Vec<String> = if_node
            .else_branch
            .as_ref()
            .map(|nodes| nodes.iter().map(|n| self.view_node_to_rust(n)).collect())
            .unwrap_or_default();

        format!(
            "if {} {{ vec![{}] }} else {{ vec![{}] }}",
            condition,
            then_nodes.join(", "),
            else_nodes.join(", ")
        )
    }

    fn for_node_to_rust(&mut self, for_node: &ForNode) -> String {
        let iterable = self.expression_to_rust(&for_node.iterable);
        let body_nodes: Vec<String> = for_node
            .body
            .iter()
            .map(|n| self.view_node_to_rust(n))
            .collect();

        format!(
            "{}.iter().map(|{}| vec![{}]).flatten().collect::<Vec<_>>()",
            iterable,
            for_node.pattern,
            body_nodes.join(", ")
        )
    }

    fn component_node_to_rust(&mut self, comp: &ComponentNode) -> String {
        let mut code = format!("VNode::component(\"{}\", vec![", comp.name);

        // Props
        for (i, (name, value)) in comp.props.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            let value_rust = self.expression_to_rust(value);
            code.push_str(&format!("(\"{}\", {})", name, value_rust));
        }

        code.push_str("], vec![");

        // Children
        for (i, child) in comp.children.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&self.view_node_to_rust(child));
        }

        code.push_str("])");
        code
    }

    fn generate_lifecycle_methods(&self, hooks: &[LifecycleHook]) -> String {
        let mut code = String::new();
        for hook in hooks {
            match hook.kind {
                LifecycleKind::OnMount => {
                    code.push_str("// On mount\n");
                    code.push_str(&self.transform_statements(&hook.body));
                }
                LifecycleKind::OnDestroy => {
                    code.push_str("// On destroy\n");
                    code.push_str(&self.transform_statements(&hook.body));
                }
                LifecycleKind::OnUpdate => {
                    code.push_str("// On update\n");
                    code.push_str(&self.transform_statements(&hook.body));
                }
            }
        }
        code
    }

    fn transform_statements(&self, statements: &[Statement]) -> String {
        statements
            .iter()
            .map(|stmt| self.transform_statement(stmt))
            .collect::<Vec<_>>()
            .join("\n        ")
    }

    fn transform_statement(&self, stmt: &Statement) -> String {
        match stmt {
            Statement::Let {
                pattern,
                type_,
                value,
                ..
            } => {
                let pattern_str = match pattern {
                    Pattern::Identifier(name) => name.clone(),
                    Pattern::Tuple(patterns) => {
                        let names: Vec<String> = patterns
                            .iter()
                            .map(|p| match p {
                                Pattern::Identifier(n) => n.clone(),
                                _ => "_".to_string(),
                            })
                            .collect();
                        format!("({})", names.join(", "))
                    }
                    _ => "_".to_string(),
                };
                let type_str = type_
                    .as_ref()
                    .map(|t| format!(": {}", self.type_to_rust(t)))
                    .unwrap_or_default();
                let value_rust = self.expression_to_rust(value);
                format!("let {}{} = {};", pattern_str, type_str, value_rust)
            }
            Statement::Assignment { target, value } => {
                // Check if target is a signal variable
                if let Expression::Identifier(name) = target {
                    if self.signal_vars.contains_key(name) {
                        let value_rust = self.expression_to_rust(value);
                        return format!("self.{}.set({});", name, value_rust);
                    }
                }
                let target_rust = self.expression_to_rust(target);
                let value_rust = self.expression_to_rust(value);
                format!("{} = {};", target_rust, value_rust)
            }
            Statement::Return(Some(expr)) => {
                format!("return {};", self.expression_to_rust(expr))
            }
            Statement::Return(None) => "return;".to_string(),
            Statement::Expression(expr) => {
                format!("{};", self.expression_to_rust(expr))
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition_rust = self.expression_to_rust(condition);
                let then_rust = self.transform_statements(then_block);
                let else_rust = else_block
                    .as_ref()
                    .map(|stmts| self.transform_statements(stmts));

                if let Some(else_code) = else_rust {
                    format!(
                        "if {} {{\n            {}\n        }} else {{\n            {}\n        }}",
                        condition_rust, then_rust, else_code
                    )
                } else {
                    format!(
                        "if {} {{\n            {}\n        }}",
                        condition_rust, then_rust
                    )
                }
            }
            _ => format!("// TODO: transform {:?}", stmt),
        }
    }

    fn expression_to_rust(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => {
                // Check if it's a signal variable
                if self.signal_vars.contains_key(name) {
                    format!("self.{}.get()", name)
                } else {
                    name.clone()
                }
            }
            Expression::Literal(lit) => self.literal_to_rust(lit),
            Expression::Binary { left, right, op } => {
                let left_rust = self.expression_to_rust(left);
                let right_rust = self.expression_to_rust(right);
                let op_str = self.binary_op_to_rust(op);
                format!("({} {} {})", left_rust, op_str, right_rust)
            }
            Expression::Unary { operand, op } => {
                let operand_rust = self.expression_to_rust(operand);
                let op_str = self.unary_op_to_rust(op);
                format!("({}{})", op_str, operand_rust)
            }
            Expression::Call {
                function,
                arguments,
            } => {
                let func_rust = self.expression_to_rust(function);
                let args_rust: Vec<String> = arguments
                    .iter()
                    .map(|(_, arg)| self.expression_to_rust(arg))
                    .collect();
                format!("{}({})", func_rust, args_rust.join(", "))
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let obj_rust = self.expression_to_rust(object);
                let args_rust: Vec<String> = arguments
                    .iter()
                    .map(|(_, arg)| self.expression_to_rust(arg))
                    .collect();
                format!("{}.{}({})", obj_rust, method, args_rust.join(", "))
            }
            Expression::FieldAccess { object, field } => {
                let obj_rust = self.expression_to_rust(object);
                format!("{}.{}", obj_rust, field)
            }
            _ => format!("/* TODO: expression {:?} */", expr),
        }
    }

    fn literal_to_rust(&self, lit: &crate::parser::Literal) -> String {
        match lit {
            crate::parser::Literal::Int(n) => n.to_string(),
            crate::parser::Literal::Float(f) => f.to_string(),
            crate::parser::Literal::String(s) => format!("\"{}\"", s),
            crate::parser::Literal::Bool(b) => b.to_string(),
            crate::parser::Literal::Char(c) => format!("'{}'", c),
        }
    }

    fn binary_op_to_rust(&self, op: &crate::parser::BinaryOp) -> &'static str {
        match op {
            crate::parser::BinaryOp::Add => "+",
            crate::parser::BinaryOp::Sub => "-",
            crate::parser::BinaryOp::Mul => "*",
            crate::parser::BinaryOp::Div => "/",
            crate::parser::BinaryOp::Mod => "%",
            crate::parser::BinaryOp::Eq => "==",
            crate::parser::BinaryOp::Ne => "!=",
            crate::parser::BinaryOp::Lt => "<",
            crate::parser::BinaryOp::Le => "<=",
            crate::parser::BinaryOp::Gt => ">",
            crate::parser::BinaryOp::Ge => ">=",
            crate::parser::BinaryOp::And => "&&",
            crate::parser::BinaryOp::Or => "||",
        }
    }

    fn unary_op_to_rust(&self, op: &crate::parser::UnaryOp) -> &'static str {
        match op {
            crate::parser::UnaryOp::Neg => "-",
            crate::parser::UnaryOp::Not => "!",
            crate::parser::UnaryOp::Ref => "&",
            crate::parser::UnaryOp::MutRef => "&mut ",
            crate::parser::UnaryOp::Deref => "*",
        }
    }

    fn type_to_rust(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "i32".to_string(),
            Type::Int32 => "i32".to_string(),
            Type::Uint => "u32".to_string(),
            Type::Float => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Custom(name) => name.clone(),
            Type::Vec(inner) => format!("Vec<{}>", self.type_to_rust(inner)),
            Type::Option(inner) => format!("Option<{}>", self.type_to_rust(inner)),
            Type::Result(ok, err) => format!(
                "Result<{}, {}>",
                self.type_to_rust(ok),
                self.type_to_rust(err)
            ),
            Type::Reference(inner) => format!("&{}", self.type_to_rust(inner)),
            Type::MutableReference(inner) => format!("&mut {}", self.type_to_rust(inner)),
            Type::Tuple(types) => {
                let types_rust: Vec<String> = types.iter().map(|t| self.type_to_rust(t)).collect();
                format!("({})", types_rust.join(", "))
            }
            _ => "()".to_string(),
        }
    }
}

/// A component transformed to use signals
#[derive(Debug, Clone)]
pub struct TransformedComponent {
    pub name: String,
    pub fields: Vec<SignalField>,
    pub methods: Vec<TransformedMethod>,
}

#[derive(Debug, Clone)]
pub struct SignalField {
    pub name: String,
    pub type_: String,         // e.g., "Signal<i32>"
    pub initial_value: String, // e.g., "Signal::new(0)"
}

#[derive(Debug, Clone)]
pub struct TransformedMethod {
    pub name: String,
    pub params: Vec<String>, // TODO: Proper parameter representation
    pub return_type: Option<String>,
    pub body: String, // Rust code
}

impl TransformedComponent {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            fields: Vec::new(),
            methods: Vec::new(),
        }
    }
}

impl Default for TransformedComponent {
    fn default() -> Self {
        Self::new()
    }
}
