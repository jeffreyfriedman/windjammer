// Ownership and borrow checking analyzer
use crate::parser::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AnalyzedFunction {
    pub decl: FunctionDecl,
    pub inferred_ownership: HashMap<String, OwnershipMode>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OwnershipMode {
    Owned,
    Borrowed,
    MutBorrowed,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub param_ownership: Vec<OwnershipMode>,
    pub return_ownership: OwnershipMode,
}

#[derive(Debug, Clone)]
pub struct SignatureRegistry {
    signatures: HashMap<String, FunctionSignature>,
}

impl SignatureRegistry {
    pub fn new() -> Self {
        SignatureRegistry {
            signatures: HashMap::new(),
        }
    }
    
    pub fn add_function(&mut self, name: String, sig: FunctionSignature) {
        self.signatures.insert(name, sig);
    }
    
    pub fn get_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(name)
    }
}

pub struct Analyzer {
    // Track variable ownership modes
    variables: HashMap<String, OwnershipMode>,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            variables: HashMap::new(),
        }
    }
    
    pub fn analyze_program(&mut self, program: &Program) -> Result<(Vec<AnalyzedFunction>, SignatureRegistry), String> {
        let mut analyzed = Vec::new();
        let mut registry = SignatureRegistry::new();
        
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    let analyzed_func = self.analyze_function(func)?;
                    let signature = self.build_signature(&analyzed_func);
                    registry.add_function(func.name.clone(), signature);
                    analyzed.push(analyzed_func);
                }
                Item::Impl(impl_block) => {
                    // Analyze methods in impl blocks
                    for func in &impl_block.functions {
                        let analyzed_func = self.analyze_function(func)?;
                        let signature = self.build_signature(&analyzed_func);
                        registry.add_function(func.name.clone(), signature);
                        analyzed.push(analyzed_func);
                    }
                }
                _ => {}
            }
        }
        
        Ok((analyzed, registry))
    }
    
    fn analyze_function(&mut self, func: &FunctionDecl) -> Result<AnalyzedFunction, String> {
        let mut inferred_ownership = HashMap::new();
        
        // Analyze each parameter to infer ownership mode
        for param in &func.parameters {
            let mode = match param.ownership {
                OwnershipHint::Owned => OwnershipMode::Owned,
                OwnershipHint::Mut => OwnershipMode::MutBorrowed,
                OwnershipHint::Ref => OwnershipMode::Borrowed,
                OwnershipHint::Inferred => {
                    // Perform inference based on usage in function body
                    self.infer_parameter_ownership(&param.name, &func.body, &func.return_type)?
                }
            };
            
            inferred_ownership.insert(param.name.clone(), mode);
        }
        
        Ok(AnalyzedFunction {
            decl: func.clone(),
            inferred_ownership,
        })
    }
    
    fn infer_parameter_ownership(
        &self,
        param_name: &str,
        body: &[Statement],
        _return_type: &Option<Type>,
    ) -> Result<OwnershipMode, String> {
        // Simple heuristic-based inference
        
        // 1. Check if parameter is mutated
        if self.is_mutated(param_name, body) {
            return Ok(OwnershipMode::MutBorrowed);
        }
        
        // 2. Check if parameter is returned (escapes function)
        if self.is_returned(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }
        
        // 3. Check if parameter is stored in a struct or collection
        if self.is_stored(param_name, body) {
            return Ok(OwnershipMode::Owned);
        }
        
        // 4. Default to borrowed for read-only access
        Ok(OwnershipMode::Borrowed)
    }
    
    fn is_mutated(&self, name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Assignment { target, .. } => {
                    if let Expression::Identifier(id) = target {
                        if id == name {
                            return true;
                        }
                    }
                }
                Statement::Expression(expr) => {
                    // Check for method calls that might mutate
                    if self.has_mutable_method_call(name, expr) {
                        return true;
                    }
                }
                Statement::If { then_block, else_block, .. } => {
                    if self.is_mutated(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_mutated(name, else_b) {
                            return true;
                        }
                    }
                }
                Statement::Loop { body } | Statement::While { body, .. } | Statement::For { body, .. } => {
                    if self.is_mutated(name, body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
    
    fn has_mutable_method_call(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                if let Expression::Identifier(id) = &**object {
                    if id == name {
                        // Heuristic: methods like push, insert, etc. are mutating
                        return method.starts_with("push")
                            || method.starts_with("insert")
                            || method.starts_with("remove")
                            || method.starts_with("clear")
                            || method.ends_with("_mut");
                    }
                }
                false
            }
            _ => false,
        }
    }
    
    fn is_returned(&self, name: &str, statements: &[Statement]) -> bool {
        for stmt in statements {
            match stmt {
                Statement::Return(Some(expr)) => {
                    if self.expression_uses_identifier(name, expr) {
                        return true;
                    }
                }
                Statement::If { then_block, else_block, .. } => {
                    if self.is_returned(name, then_block) {
                        return true;
                    }
                    if let Some(else_b) = else_block {
                        if self.is_returned(name, else_b) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
    
    fn is_stored(&self, name: &str, statements: &[Statement]) -> bool {
        // Check if the parameter is stored in a struct field or collection
        for stmt in statements {
            match stmt {
                Statement::Let { value, .. } => {
                    if let Expression::StructLiteral { fields, .. } = value {
                        for (_, field_expr) in fields {
                            if self.expression_uses_identifier(name, field_expr) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
    
    fn expression_uses_identifier(&self, name: &str, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(id) => id == name,
            Expression::Binary { left, right, .. } => {
                self.expression_uses_identifier(name, left)
                    || self.expression_uses_identifier(name, right)
            }
            Expression::Unary { operand, .. } => {
                self.expression_uses_identifier(name, operand)
            }
            Expression::Call { arguments, .. } => {
                arguments.iter().any(|(_label, arg)| self.expression_uses_identifier(name, arg))
            }
            Expression::MethodCall { object, arguments, .. } => {
                self.expression_uses_identifier(name, object)
                    || arguments.iter().any(|(_label, arg)| self.expression_uses_identifier(name, arg))
            }
            Expression::FieldAccess { object, .. } => {
                self.expression_uses_identifier(name, object)
            }
            Expression::TryOp(inner) => {
                self.expression_uses_identifier(name, inner)
            }
            _ => false,
        }
    }
    
    fn build_signature(&self, func: &AnalyzedFunction) -> FunctionSignature {
        let param_ownership: Vec<OwnershipMode> = func.decl.parameters
            .iter()
            .map(|param| {
                func.inferred_ownership
                    .get(&param.name)
                    .cloned()
                    .unwrap_or(OwnershipMode::Owned)
            })
            .collect();
        
        FunctionSignature {
            name: func.decl.name.clone(),
            param_ownership,
            return_ownership: OwnershipMode::Owned, // For now, always owned
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_infer_borrowed() {
        let analyzer = Analyzer::new();
        
        // fn print(s: string) { println(s) }
        // Should infer borrowed
        let body = vec![
            Statement::Expression(Expression::Call {
                function: Box::new(Expression::Identifier("println".to_string())),
                arguments: vec![Expression::Identifier("s".to_string())],
            })
        ];
        
        let mode = analyzer.infer_parameter_ownership("s", &body, &None).unwrap();
        assert_eq!(mode, OwnershipMode::Borrowed);
    }
}

