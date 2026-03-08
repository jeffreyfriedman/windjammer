//! WGSL code generation
//!
//! Generates WGSL shader code from Windjammer AST.

use crate::codegen::backend::{CodegenBackend, CodegenConfig, CodegenOutput, Target};
use crate::parser::{Program, FunctionDecl, Statement, Expression, BinaryOp, Pattern, Literal, Item};
use crate::parser_impl::StructDecl;
use anyhow::Result;

use super::types::map_type_to_wgsl;
use super::validation::validate_for_gpu;
use super::structs::StructLayout;

pub struct WgslBackend;

impl WgslBackend {
    pub fn new() -> Self {
        WgslBackend
    }
    
    fn generate_struct(&self, struct_decl: &StructDecl) -> Result<String> {
        // Calculate layout with automatic padding
        let layout = StructLayout::from_struct_decl(struct_decl)?;
        
        // Generate WGSL with padding fields included
        let mut output = layout.to_wgsl_string();
        output.push('\n');
        
        Ok(output)
    }
    
    fn generate_extern_let(
        &self,
        name: &str,
        type_: &crate::parser::ast::types::Type,
        decorators: &[crate::parser::Decorator],
        _is_pub: bool,
    ) -> Result<String> {
        let mut output = String::new();
        
        // Generate decorators (@group, @binding)
        for decorator in decorators {
            // Skip storage-class decorators (@uniform, @storage) - handled below
            if decorator.name == "uniform" || decorator.name == "storage" {
                continue;
            }
            
            output.push_str("@");
            output.push_str(&decorator.name);
            
            if !decorator.arguments.is_empty() {
                output.push('(');
                for (i, (key, value)) in decorator.arguments.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    
                    if !key.is_empty() {
                        output.push_str(key);
                        output.push_str(" = ");
                    }
                    
                    output.push_str(&self.generate_expression(value)?);
                }
                output.push(')');
            }
            
            output.push('\n');
        }
        
        // Determine storage class from decorators
        let is_uniform = decorators.iter().any(|d| d.name == "uniform");
        
        let storage_class = if is_uniform {
            "uniform"
        } else if let Some(storage_dec) = decorators.iter().find(|d| d.name == "storage") {
            // @storage(read_write) or @storage(read)
            if let Some((_, value)) = storage_dec.arguments.first() {
                // Extract the access mode from expression
                match self.generate_expression(value)?.as_str() {
                    "read_write" => "storage, read_write",
                    "read" => "storage, read",
                    _ => "storage",
                }
            } else {
                "storage"
            }
        } else {
            // Default to uniform if no storage decorator
            "uniform"
        };
        
        // Generate: var<uniform> name: Type;
        output.push_str("var<");
        output.push_str(storage_class);
        output.push_str("> ");
        output.push_str(name);
        output.push_str(": ");
        
        let mut wgsl_type = map_type_to_wgsl(type_)?;
        
        // CRITICAL FIX: Convert u32 → f32 in uniform buffers!
        // This prevents the black screen bug where host sends f32 but shader expects u32
        if is_uniform {
            let original_type = wgsl_type.clone();
            wgsl_type = wgsl_type.to_uniform_safe_type();
            
            // Add comment if type was converted
            if original_type != wgsl_type {
                output.push_str("/* auto-converted from ");
                output.push_str(&original_type.to_wgsl_string());
                output.push_str(" */ ");
            }
        }
        
        output.push_str(&wgsl_type.to_wgsl_string());
        
        output.push_str(";\n");
        
        Ok(output)
    }
    
    fn generate_function(&self, func: &FunctionDecl) -> Result<String> {
        let mut output = String::new();
        
        // Process GPU decorators
        for decorator in &func.decorators {
            match decorator.name.as_str() {
                "compute" => {
                    output.push_str("@compute");
                    
                    // Extract workgroup_size argument
                    if let Some((_key, workgroup_expr)) = decorator.arguments.iter()
                        .find(|(k, _)| k == "workgroup_size")
                    {
                        // Parse array expression: [x, y, z]
                        if let Expression::Array { elements, .. } = workgroup_expr {
                            if elements.len() == 3 {
                                output.push_str(" @workgroup_size(");
                                for (i, elem) in elements.iter().enumerate() {
                                    if i > 0 {
                                        output.push_str(", ");
                                    }
                                    output.push_str(&self.generate_expression(elem)?);
                                }
                                output.push_str(")");
                            }
                        }
                    }
                    output.push('\n');
                }
                "vertex" => {
                    output.push_str("@vertex\n");
                }
                "fragment" => {
                    output.push_str("@fragment\n");
                }
                _ => {} // Ignore other decorators
            }
        }
        
        // Function signature
        output.push_str("fn ");
        output.push_str(&func.name);
        output.push('(');
        
        // Parameters
        for (i, param) in func.parameters.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            
            // Generate parameter decorators (@builtin, @location, etc.)
            for decorator in &param.decorators {
                output.push_str("@");
                output.push_str(&decorator.name);
                
                // Generate decorator arguments
                if !decorator.arguments.is_empty() {
                    output.push('(');
                    for (j, (key, value)) in decorator.arguments.iter().enumerate() {
                        if j > 0 {
                            output.push_str(", ");
                        }
                        
                        if !key.is_empty() {
                            // Named argument: key = value
                            output.push_str(key);
                            output.push_str(" = ");
                        }
                        
                        // Generate value expression
                        output.push_str(&self.generate_expression(value)?);
                    }
                    output.push(')');
                }
                
                output.push(' ');
            }
            
            output.push_str(&param.name);
            output.push_str(": ");
            
            let wgsl_type = map_type_to_wgsl(&param.type_)?;
            output.push_str(&wgsl_type.to_wgsl_string());
        }
        
        output.push(')');
        
        // Return type
        if let Some(ref return_type) = func.return_type {
            output.push_str(" -> ");
            let wgsl_type = map_type_to_wgsl(return_type)?;
            output.push_str(&wgsl_type.to_wgsl_string());
        }
        
        output.push_str(" {\n");
        
        // Function body
        let body_len = func.body.len();
        for (i, stmt) in func.body.iter().enumerate() {
            let is_last = i == body_len - 1;
            
            // WGSL requires explicit returns, so convert last expression to return
            if is_last && matches!(stmt, Statement::Expression { .. }) {
                if let Statement::Expression { expr, .. } = stmt {
                    output.push_str("    return ");
                    output.push_str(&self.generate_expression(expr)?);
                    output.push_str(";\n");
                    continue;
                }
            }
            
            output.push_str(&self.generate_statement(stmt, 1)?);
        }
        
        output.push_str("}\n");
        
        Ok(output)
    }
    
    fn generate_statement(&self, stmt: &Statement, indent: usize) -> Result<String> {
        let indent_str = "    ".repeat(indent);
        let mut output = String::new();
        
        match stmt {
            Statement::Let { pattern, value, mutable, type_, .. } => {
                output.push_str(&indent_str);
                
                // WGSL uses 'var' for mutable locals, 'let' for immutable
                if *mutable {
                    output.push_str("var ");
                } else {
                    output.push_str("let ");
                }
                
                // Extract variable name from pattern
                let var_name = match pattern {
                    Pattern::Identifier(name) => name.clone(),
                    _ => "tmp".to_string(), // TODO: Handle complex patterns
                };
                
                output.push_str(&var_name);
                
                // Add type annotation if present (required for var without initializer)
                if let Some(type_annotation) = type_ {
                    output.push_str(": ");
                    let wgsl_type = map_type_to_wgsl(type_annotation)?;
                    output.push_str(&wgsl_type.to_wgsl_string());
                }
                
                output.push_str(" = ");
                output.push_str(&self.generate_expression(value)?);
                output.push_str(";\n");
            }
            
            Statement::Return { value, .. } => {
                output.push_str(&indent_str);
                output.push_str("return");
                if let Some(expr) = value {
                    output.push(' ');
                    output.push_str(&self.generate_expression(expr)?);
                }
                output.push_str(";\n");
            }
            
            Statement::Expression { expr, .. } => {
                output.push_str(&indent_str);
                output.push_str(&self.generate_expression(expr)?);
                output.push_str(";\n");
            }
            
            Statement::Assignment { target, value, compound_op, .. } => {
                output.push_str(&indent_str);
                output.push_str(&self.generate_expression(target)?);
                
                if let Some(op) = compound_op {
                    // Compound assignment: +=, -=, *=, /=, |=, &=, etc.
                    use crate::parser::CompoundOp;
                    let op_str = match op {
                        CompoundOp::Add => " += ",
                        CompoundOp::Sub => " -= ",
                        CompoundOp::Mul => " *= ",
                        CompoundOp::Div => " /= ",
                        CompoundOp::Mod => " %= ",
                        CompoundOp::BitAnd => " &= ",
                        CompoundOp::BitOr => " |= ",
                        CompoundOp::BitXor => " ^= ",
                        CompoundOp::Shl => " <<= ",
                        CompoundOp::Shr => " >>= ",
                    };
                    output.push_str(op_str);
                } else {
                    output.push_str(" = ");
                }
                
                output.push_str(&self.generate_expression(value)?);
                output.push_str(";\n");
            }
            
            Statement::If { condition, then_block, else_block, .. } => {
                output.push_str(&indent_str);
                output.push_str("if (");
                output.push_str(&self.generate_expression(condition)?);
                output.push_str(") {\n");
                
                for stmt in then_block {
                    output.push_str(&self.generate_statement(stmt, indent + 1)?);
                }
                
                output.push_str(&indent_str);
                output.push('}');
                
                if let Some(else_stmts) = else_block {
                    output.push_str(" else {\n");
                    for stmt in else_stmts {
                        output.push_str(&self.generate_statement(stmt, indent + 1)?);
                    }
                    output.push_str(&indent_str);
                    output.push('}');
                }
                
                output.push('\n');
            }
            
            Statement::While { condition, body, .. } => {
                output.push_str(&indent_str);
                output.push_str("while (");
                output.push_str(&self.generate_expression(condition)?);
                output.push_str(") {\n");
                
                for stmt in body {
                    output.push_str(&self.generate_statement(stmt, indent + 1)?);
                }
                
                output.push_str(&indent_str);
                output.push_str("}\n");
            }
            
            _ => {
                // TODO: Implement other statement types
            }
        }
        
        Ok(output)
    }
    
    fn generate_expression(&self, expr: &Expression) -> Result<String> {
        match expr {
            Expression::Identifier { name, .. } => Ok(name.clone()),
            
            Expression::Literal { value, .. } => {
                match value {
                    Literal::Int(n) => Ok(n.to_string()),
                    Literal::Float(f) => Ok(f.to_string()),
                    Literal::Bool(b) => Ok(b.to_string()),
                    Literal::String(s) => Ok(format!("\"{}\"", s)),
                    Literal::Char(c) => Ok(format!("'{}'", c)),
                }
            }
            
            Expression::Binary { left, op, right, .. } => {
                let left_str = self.generate_expression(left)?;
                let right_str = self.generate_expression(right)?;
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
                Ok(format!("({} {} {})", left_str, op_str, right_str))
            }
            
            Expression::Call { function, arguments, .. } => {
                let mut output = String::new();
                
                // Get function name
                if let Expression::Identifier { name, .. } = &**function {
                    output.push_str(name);
                } else {
                    output.push_str(&self.generate_expression(function)?);
                }
                
                output.push('(');
                
                for (i, (_, arg)) in arguments.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&self.generate_expression(arg)?);
                }
                
                output.push(')');
                Ok(output)
            }
            
            Expression::FieldAccess { object, field, .. } => {
                let object_str = self.generate_expression(object)?;
                Ok(format!("{}.{}", object_str, field))
            }
            
            Expression::Index { object, index, .. } => {
                let object_str = self.generate_expression(object)?;
                let index_str = self.generate_expression(index)?;
                Ok(format!("{}[{}]", object_str, index_str))
            }
            
            Expression::Cast { expr, type_, .. } => {
                // Type cast: float(x) or uint(x)
                let wgsl_type = map_type_to_wgsl(type_)?;
                let type_str = wgsl_type.to_wgsl_string();
                let expr_str = self.generate_expression(expr)?;
                Ok(format!("{}({})", type_str, expr_str))
            }
            
            Expression::Unary { op, operand, .. } => {
                use crate::parser::UnaryOp;
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                    UnaryOp::Ref | UnaryOp::MutRef | UnaryOp::Deref => {
                        // Not supported in WGSL
                        return Ok("/* Unsupported unary op */".to_string());
                    }
                };
                let operand_str = self.generate_expression(operand)?;
                Ok(format!("{}{}", op_str, operand_str))
            }
            
            Expression::StructLiteral { name, fields, .. } => {
                // Struct literal: Output { position: vec4(...), color: vec4(...) }
                let mut output = String::new();
                output.push_str(name);
                output.push('(');
                
                for (i, (_field_name, field_expr)) in fields.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&self.generate_expression(field_expr)?);
                }
                
                output.push(')');
                Ok(output)
            }
            
            _ => {
                // TODO: Implement other expression types
                Ok("/* TODO */".to_string())
            }
        }
    }
}

impl CodegenBackend for WgslBackend {
    fn name(&self) -> &str {
        "WGSL"
    }
    
    fn target(&self) -> Target {
        Target::Wgsl
    }
    
    fn generate(&self, program: &Program, _config: &CodegenConfig) -> Result<CodegenOutput> {
        // Validate program can be compiled to GPU
        validate_for_gpu(program)?;
        
        let mut output = String::new();
        
        // Generate all items (structs first, then globals, then functions)
        for item in &program.items {
            if let Item::Struct { decl, .. } = item {
                output.push_str(&self.generate_struct(decl)?);
                output.push('\n');
            }
        }
        
        for item in &program.items {
            if let Item::ExternLet { name, type_, decorators, is_pub, .. } = item {
                output.push_str(&self.generate_extern_let(name, type_, decorators, *is_pub)?);
                output.push('\n');
            }
        }
        
        for item in &program.items {
            if let Item::Function { decl, .. } = item {
                output.push_str(&self.generate_function(decl)?);
                output.push('\n');
            }
        }
        
        Ok(CodegenOutput::new(output, "wgsl".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgsl_backend_exists() {
        let backend = WgslBackend::new();
        assert_eq!(backend.name(), "WGSL");
        assert_eq!(backend.target(), Target::Wgsl);
    }
}
