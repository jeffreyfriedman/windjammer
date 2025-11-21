// Component Analyzer - Analyzes @component decorated structs for reactive UI
// This module handles:
// 1. Validating @component structs
// 2. Tracking reactive state fields
// 3. Generating component metadata for codegen

use crate::parser::{Decorator, FunctionDecl, ImplBlock, Item, StructDecl, Type};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub name: String,
    pub state_fields: Vec<StateField>,
    pub methods: Vec<ComponentMethod>,
    pub render_method: Option<String>, // Name of the render method
}

#[derive(Debug, Clone)]
pub struct StateField {
    pub name: String,
    pub field_type: Type,
    pub is_reactive: bool, // If true, changes trigger re-render
}

#[derive(Debug, Clone)]
pub struct ComponentMethod {
    pub name: String,
    pub mutates_state: bool,
    pub is_event_handler: bool,
}

pub struct ComponentAnalyzer {
    components: HashMap<String, ComponentInfo>,
}

impl Default for ComponentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentAnalyzer {
    pub fn new() -> Self {
        ComponentAnalyzer {
            components: HashMap::new(),
        }
    }

    /// Analyze a program and extract component information
    pub fn analyze(&mut self, items: &[Item]) -> Result<(), String> {
        // First pass: Find all @component structs
        for item in items {
            if let Item::Struct {
                decl: struct_decl, ..
            } = item
            {
                if self.has_component_decorator(&struct_decl.decorators) {
                    self.analyze_component_struct(struct_decl)?;
                }
            }
        }

        // Second pass: Find impl blocks for components
        for item in items {
            if let Item::Impl {
                block: impl_block, ..
            } = item
            {
                let type_name = &impl_block.type_name;
                if self.components.contains_key(type_name) {
                    self.analyze_component_impl(type_name, impl_block)?;
                }
            }
        }

        Ok(())
    }

    fn has_component_decorator(&self, decorators: &[Decorator]) -> bool {
        decorators.iter().any(|d| d.name == "component")
    }

    fn analyze_component_struct(&mut self, struct_decl: &StructDecl) -> Result<(), String> {
        let mut state_fields = Vec::new();

        for field in &struct_decl.fields {
            state_fields.push(StateField {
                name: field.name.clone(),
                field_type: field.field_type.clone(),
                is_reactive: true, // All fields in @component are reactive by default
            });
        }

        let component_info = ComponentInfo {
            name: struct_decl.name.clone(),
            state_fields,
            methods: Vec::new(),
            render_method: None,
        };

        self.components
            .insert(struct_decl.name.clone(), component_info);
        Ok(())
    }

    fn analyze_component_impl(
        &mut self,
        type_name: &str,
        impl_block: &ImplBlock,
    ) -> Result<(), String> {
        // First, collect state fields to avoid borrow issues
        let state_fields = self
            .components
            .get(type_name)
            .ok_or_else(|| format!("Component {} not found", type_name))?
            .state_fields
            .clone();

        // Now we can mutate the component
        let component = self
            .components
            .get_mut(type_name)
            .ok_or_else(|| format!("Component {} not found", type_name))?;

        for method in &impl_block.functions {
            let mutates_state = Self::method_mutates_state(method, &state_fields);

            // Check if this is the render method
            if method.name == "render" {
                component.render_method = Some(method.name.clone());
            }

            component.methods.push(ComponentMethod {
                name: method.name.clone(),
                mutates_state,
                is_event_handler: mutates_state && method.name != "render",
            });
        }

        Ok(())
    }

    fn method_mutates_state(method: &FunctionDecl, state_fields: &[StateField]) -> bool {
        // Simple heuristic: if method body contains assignments to state fields
        // In a full implementation, we'd do proper data flow analysis
        let state_field_names: HashSet<_> = state_fields.iter().map(|f| f.name.as_str()).collect();

        // Check if method has mutable self parameter or assigns to fields
        for param in &method.parameters {
            if param.name == "self"
                && matches!(param.ownership, crate::parser::OwnershipHint::Owned)
            {
                // Mutable self
                return true;
            }
        }

        // Check for assignments in body (simplified check)
        for stmt in &method.body {
            if Self::statement_mutates_fields(stmt, &state_field_names) {
                return true;
            }
        }

        false
    }

    fn statement_mutates_fields(
        stmt: &crate::parser::Statement,
        field_names: &HashSet<&str>,
    ) -> bool {
        use crate::parser::Statement;

        match stmt {
            Statement::Assignment { target, .. } => {
                // Check if target is a field name
                if let crate::parser::Expression::Identifier { name, .. } = target {
                    return field_names.contains(name.as_str());
                }
                false
            }
            // Note: Windjammer doesn't have CompoundAssignment in AST
            // Compound assignments like += are parsed as regular assignments
            _ => false,
        }
    }

    pub fn get_component(&self, name: &str) -> Option<&ComponentInfo> {
        self.components.get(name)
    }

    pub fn is_component(&self, name: &str) -> bool {
        self.components.contains_key(name)
    }

    pub fn all_components(&self) -> impl Iterator<Item = (&String, &ComponentInfo)> {
        self.components.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_analyzer_creation() {
        let analyzer = ComponentAnalyzer::new();
        assert_eq!(analyzer.components.len(), 0);
    }
}
