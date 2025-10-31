//! Dependency analyzer for reactive variables

use super::ast::*;
use crate::parser::{Expression, Pattern, Statement};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

/// Analyzes reactive dependencies in a component
pub struct DependencyAnalyzer {
    /// Map of variable name to its dependencies
    _dependencies: HashMap<String, HashSet<String>>,
}

impl DependencyAnalyzer {
    pub fn new() -> Self {
        Self {
            _dependencies: HashMap::new(),
        }
    }

    /// Analyze a component and return dependency information
    pub fn analyze(component: &ComponentFile) -> Result<DependencyInfo> {
        let mut analyzer = Self::new();

        match &component.style {
            ComponentStyle::Minimal(minimal) => analyzer.analyze_minimal(minimal),
            ComponentStyle::Advanced(advanced) => analyzer.analyze_advanced(advanced),
        }
    }

    fn analyze_minimal(&mut self, component: &MinimalComponent) -> Result<DependencyInfo> {
        let mut info = DependencyInfo::new();

        // All state variables are reactive
        for state in &component.state {
            info.reactive_vars.insert(state.name.clone());
        }

        // Analyze computed dependencies
        for computed in &component.computed {
            let deps = self.extract_dependencies(&computed.expression);
            info.computed_deps.insert(computed.name.clone(), deps);
        }

        // Analyze function dependencies
        for func in &component.functions {
            let reads = self.extract_reads_from_statements(&func.body);
            let writes = self.extract_writes_from_statements(&func.body);
            info.function_reads.insert(func.name.clone(), reads);
            info.function_writes.insert(func.name.clone(), writes);
        }

        // Analyze view dependencies
        self.analyze_view_dependencies(&component.view, &mut info);

        Ok(info)
    }

    fn analyze_advanced(&mut self, component: &AdvancedComponent) -> Result<DependencyInfo> {
        let mut info = DependencyInfo::new();

        // All struct fields are potentially reactive
        for field in &component.struct_decl.fields {
            info.reactive_vars.insert(field.name.clone());
        }

        // Analyze method dependencies
        for method in &component.impl_block.methods {
            let reads = self.extract_reads_from_statements(&method.function.body);
            let writes = self.extract_writes_from_statements(&method.function.body);
            info.function_reads
                .insert(method.function.name.clone(), reads);
            info.function_writes
                .insert(method.function.name.clone(), writes);

            if method.kind == MethodKind::Computed {
                let deps = self.extract_reads_from_statements(&method.function.body);
                info.computed_deps
                    .insert(method.function.name.clone(), deps);
            }
        }

        Ok(info)
    }

    fn analyze_view_dependencies(&mut self, view: &ViewBlock, info: &mut DependencyInfo) {
        self.analyze_view_node_dependencies(&view.root, info);
    }

    fn analyze_view_node_dependencies(&mut self, node: &ViewNode, info: &mut DependencyInfo) {
        match node {
            ViewNode::Element(elem) => {
                // Analyze attributes
                for attr in &elem.attributes {
                    match attr {
                        Attribute::Dynamic { value, .. }
                        | Attribute::Event { handler: value, .. } => {
                            let deps = self.extract_dependencies(value);
                            for dep in deps {
                                info.reactive_vars.insert(dep);
                            }
                        }
                        _ => {}
                    }
                }
                // Analyze children
                for child in &elem.children {
                    self.analyze_view_node_dependencies(child, info);
                }
            }
            ViewNode::Text(text) => {
                for part in &text.parts {
                    if let TextPart::Dynamic(expr) = part {
                        let deps = self.extract_dependencies(expr);
                        for dep in deps {
                            info.reactive_vars.insert(dep);
                        }
                    }
                }
            }
            ViewNode::If(if_node) => {
                let deps = self.extract_dependencies(&if_node.condition);
                for dep in deps {
                    info.reactive_vars.insert(dep);
                }
                for child in &if_node.then_branch {
                    self.analyze_view_node_dependencies(child, info);
                }
                if let Some(else_branch) = &if_node.else_branch {
                    for child in else_branch {
                        self.analyze_view_node_dependencies(child, info);
                    }
                }
            }
            ViewNode::For(for_node) => {
                let deps = self.extract_dependencies(&for_node.iterable);
                for dep in deps {
                    info.reactive_vars.insert(dep);
                }
                for child in &for_node.body {
                    self.analyze_view_node_dependencies(child, info);
                }
            }
            ViewNode::Component(comp) => {
                for (_, value) in &comp.props {
                    let deps = self.extract_dependencies(value);
                    for dep in deps {
                        info.reactive_vars.insert(dep);
                    }
                }
                for child in &comp.children {
                    self.analyze_view_node_dependencies(child, info);
                }
            }
        }
    }

    fn extract_dependencies(&self, expr: &Expression) -> HashSet<String> {
        let mut deps = HashSet::new();
        self.extract_dependencies_recursive(expr, &mut deps);
        deps
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_dependencies_recursive(&self, expr: &Expression, deps: &mut HashSet<String>) {
        match expr {
            Expression::Identifier(name) => {
                deps.insert(name.clone());
            }
            Expression::Binary { left, right, .. } => {
                self.extract_dependencies_recursive(left, deps);
                self.extract_dependencies_recursive(right, deps);
            }
            Expression::Unary { operand, .. } => {
                self.extract_dependencies_recursive(operand, deps);
            }
            Expression::Call {
                function,
                arguments,
            } => {
                self.extract_dependencies_recursive(function, deps);
                for (_, arg) in arguments {
                    self.extract_dependencies_recursive(arg, deps);
                }
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.extract_dependencies_recursive(object, deps);
                for (_, arg) in arguments {
                    self.extract_dependencies_recursive(arg, deps);
                }
            }
            Expression::FieldAccess { object, .. } => {
                self.extract_dependencies_recursive(object, deps);
            }
            Expression::Index { object, index } => {
                self.extract_dependencies_recursive(object, deps);
                self.extract_dependencies_recursive(index, deps);
            }
            Expression::Array(elements) => {
                for elem in elements {
                    self.extract_dependencies_recursive(elem, deps);
                }
            }
            Expression::Tuple(elements) => {
                for elem in elements {
                    self.extract_dependencies_recursive(elem, deps);
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.extract_dependencies_recursive(value, deps);
                }
            }
            Expression::MacroInvocation { args, .. } => {
                for arg in args {
                    self.extract_dependencies_recursive(arg, deps);
                }
            }
            Expression::Closure { body, .. } => {
                self.extract_dependencies_recursive(body, deps);
            }
            _ => {}
        }
    }

    fn extract_reads_from_statements(&self, statements: &[Statement]) -> HashSet<String> {
        let mut reads = HashSet::new();
        for stmt in statements {
            self.extract_reads_from_statement(stmt, &mut reads);
        }
        reads
    }

    fn extract_reads_from_statement(&self, stmt: &Statement, reads: &mut HashSet<String>) {
        match stmt {
            Statement::Let { value, .. } => {
                self.extract_dependencies_recursive(value, reads);
            }
            Statement::Assignment { target, value } => {
                self.extract_dependencies_recursive(value, reads);
                // Also read the target if it's a field access or index
                if !matches!(target, Expression::Identifier(_)) {
                    self.extract_dependencies_recursive(target, reads);
                }
            }
            Statement::Return(Some(expr)) => {
                self.extract_dependencies_recursive(expr, reads);
            }
            Statement::Expression(expr) => {
                self.extract_dependencies_recursive(expr, reads);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.extract_dependencies_recursive(condition, reads);
                for stmt in then_block {
                    self.extract_reads_from_statement(stmt, reads);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.extract_reads_from_statement(stmt, reads);
                    }
                }
            }
            Statement::For { iterable, body, .. } => {
                self.extract_dependencies_recursive(iterable, reads);
                for stmt in body {
                    self.extract_reads_from_statement(stmt, reads);
                }
            }
            Statement::While { condition, body } => {
                self.extract_dependencies_recursive(condition, reads);
                for stmt in body {
                    self.extract_reads_from_statement(stmt, reads);
                }
            }
            Statement::Loop { body } => {
                for stmt in body {
                    self.extract_reads_from_statement(stmt, reads);
                }
            }
            Statement::Match { value, arms } => {
                self.extract_dependencies_recursive(value, reads);
                for arm in arms {
                    self.extract_dependencies_recursive(&arm.body, reads);
                    if let Some(guard) = &arm.guard {
                        self.extract_dependencies_recursive(guard, reads);
                    }
                }
            }
            _ => {}
        }
    }

    fn extract_writes_from_statements(&self, statements: &[Statement]) -> HashSet<String> {
        let mut writes = HashSet::new();
        for stmt in statements {
            self.extract_writes_from_statement(stmt, &mut writes);
        }
        writes
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_writes_from_statement(&self, stmt: &Statement, writes: &mut HashSet<String>) {
        match stmt {
            Statement::Let { pattern, .. } => {
                // Only track simple identifier patterns
                if let Pattern::Identifier(name) = pattern {
                    writes.insert(name.clone());
                }
            }
            Statement::Assignment {
                target: Expression::Identifier(name),
                ..
            } => {
                writes.insert(name.clone());
            }
            Statement::Assignment { .. } => {
                // Non-identifier assignments (e.g., field access) don't count as writes to local variables
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                for stmt in then_block {
                    self.extract_writes_from_statement(stmt, writes);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.extract_writes_from_statement(stmt, writes);
                    }
                }
            }
            Statement::For { body, .. }
            | Statement::While { body, .. }
            | Statement::Loop { body } => {
                for stmt in body {
                    self.extract_writes_from_statement(stmt, writes);
                }
            }
            Statement::Match { .. } => {
                // Match arms have expression bodies, not statements
                // Writes would be in nested statements within those expressions
            }
            _ => {}
        }
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about reactive dependencies
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Variables that are reactive (need to be signals)
    pub reactive_vars: HashSet<String>,
    /// Map of computed variable to its dependencies
    pub computed_deps: HashMap<String, HashSet<String>>,
    /// Map of function to variables it reads
    pub function_reads: HashMap<String, HashSet<String>>,
    /// Map of function to variables it writes
    pub function_writes: HashMap<String, HashSet<String>>,
}

impl DependencyInfo {
    pub fn new() -> Self {
        Self {
            reactive_vars: HashSet::new(),
            computed_deps: HashMap::new(),
            function_reads: HashMap::new(),
            function_writes: HashMap::new(),
        }
    }
}

impl Default for DependencyInfo {
    fn default() -> Self {
        Self::new()
    }
}
