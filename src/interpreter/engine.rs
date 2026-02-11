//! The Windjammerscript tree-walking interpreter engine.
//!
//! Walks the AST directly, evaluating expressions and executing statements.
//! Uses the same parser as the compiler backends — same `.wj` source runs
//! everywhere.

use super::environment::Environment;
use super::value::{EnumData, FunctionValue, Value};
use crate::parser::{
    BinaryOp, CompoundOp, Expression, FunctionDecl, Item, Literal, MatchArm, Pattern, Program,
    Statement, UnaryOp,
};
use std::collections::HashMap;
use std::io::Write;

/// Control flow signal returned by statement execution
enum ControlFlow {
    /// Normal execution continues
    Continue,
    /// A `return` was hit with an optional value
    Return(Value),
    /// A `break` was hit
    Break,
    /// A `continue` was hit (loop continue)
    LoopContinue,
}

/// Stored function definition
struct FunctionDef<'a> {
    decl: &'a FunctionDecl<'a>,
    /// For methods: the type this is impl'd on
    #[allow(dead_code)]
    receiver_type: Option<String>,
}

/// Information about an enum variant for runtime construction
#[derive(Debug, Clone)]
struct EnumVariantInfo {
    enum_name: String,
    variant_name: String,
    data_kind: EnumVariantKind,
}

#[derive(Debug, Clone)]
enum EnumVariantKind {
    Unit,
    Tuple(usize), // number of fields
    Struct(Vec<String>), // field names
}

/// The Windjammerscript interpreter
pub struct Interpreter<'a> {
    env: Environment,
    /// All function/method definitions indexed by name
    functions: HashMap<String, Vec<FunctionDef<'a>>>,
    /// Struct definitions: name → field names
    struct_defs: HashMap<String, Vec<String>>,
    /// Enum variant info: "EnumName::Variant" → EnumVariantInfo
    enum_variants: HashMap<String, EnumVariantInfo>,
    /// Captured stdout (for testing)
    output: Vec<String>,
    /// Whether to capture output instead of printing
    capture_output: bool,
}

impl<'a> Interpreter<'a> {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            functions: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_variants: HashMap::new(),
            output: Vec::new(),
            capture_output: false,
        }
    }

    /// Create an interpreter that captures output (for testing)
    pub fn new_capturing() -> Self {
        Self {
            env: Environment::new(),
            functions: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_variants: HashMap::new(),
            output: Vec::new(),
            capture_output: true,
        }
    }

    /// Get captured output
    pub fn get_output(&self) -> String {
        self.output.join("")
    }

    /// Run a complete program
    pub fn run(&mut self, program: &'a Program<'a>) -> Result<Value, String> {
        // First pass: register all functions, structs, impls
        self.register_definitions(program);

        // Find and call main()
        if self.functions.contains_key("main") {
            self.call_function("main", &[])
        } else {
            Err("No main() function found".to_string())
        }
    }

    /// Register all top-level definitions
    fn register_definitions(&mut self, program: &'a Program<'a>) {
        for item in &program.items {
            match item {
                Item::Function { decl, .. } => {
                    let name = decl.name.clone();
                    self.functions
                        .entry(name)
                        .or_default()
                        .push(FunctionDef {
                            decl,
                            receiver_type: None,
                        });
                }
                Item::Struct { decl, .. } => {
                    let field_names: Vec<String> =
                        decl.fields.iter().map(|f| f.name.clone()).collect();
                    self.struct_defs.insert(decl.name.clone(), field_names);
                }
                Item::Impl { block, .. } => {
                    let type_name = block.type_name.clone();
                    for method in &block.functions {
                        let method_key = format!("{}::{}", type_name, method.name);
                        self.functions
                            .entry(method_key)
                            .or_default()
                            .push(FunctionDef {
                                decl: method,
                                receiver_type: Some(type_name.clone()),
                            });
                    }
                }
                Item::Enum { decl, .. } => {
                    for variant in &decl.variants {
                        let key = format!("{}::{}", decl.name, variant.name);
                        let data_kind = match &variant.data {
                            crate::parser::EnumVariantData::Unit => EnumVariantKind::Unit,
                            crate::parser::EnumVariantData::Tuple(types) => {
                                EnumVariantKind::Tuple(types.len())
                            }
                            crate::parser::EnumVariantData::Struct(fields) => {
                                EnumVariantKind::Struct(
                                    fields.iter().map(|(name, _)| name.clone()).collect(),
                                )
                            }
                        };
                        self.enum_variants.insert(
                            key,
                            EnumVariantInfo {
                                enum_name: decl.name.clone(),
                                variant_name: variant.name.clone(),
                                data_kind,
                            },
                        );
                    }
                }
                Item::Const { name, value, .. } => {
                    let val = self.eval_expression(value);
                    self.env.define(name, val);
                }
                _ => {} // Traits, Use, etc. — not needed for execution
            }
        }
    }

    // ================================================================
    // Function Calling
    // ================================================================

    /// Call a named function with arguments
    fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        // Get function declaration (must extract before mutating env)
        let decl = {
            let func_defs = self
                .functions
                .get(name)
                .ok_or_else(|| format!("Undefined function: {}", name))?;
            let func_def = func_defs
                .first()
                .ok_or_else(|| format!("No definition for function: {}", name))?;
            func_def.decl
        };

        self.env.push_scope();

        // Bind parameters
        let param_iter = decl
            .parameters
            .iter()
            .filter(|p| p.name != "self");

        for (param, arg) in param_iter.zip(args.iter()) {
            self.env.define(&param.name, arg.clone());
        }

        // Execute body
        let result = self.exec_body(&decl.body);

        self.env.pop_scope();

        match result {
            ControlFlow::Return(val) => Ok(val),
            ControlFlow::Continue => Ok(Value::Unit),
            _ => Ok(Value::Unit),
        }
    }

    // call_method removed — replaced by call_method_with_self_mutation
    // which correctly propagates self mutations back to the receiver

    /// Built-in methods on standard types
    fn call_builtin_method(
        &mut self,
        receiver: &Value,
        method: &str,
        args: &[Value],
    ) -> Option<Value> {
        match (receiver, method) {
            // Vec methods
            (Value::Vec(items), "len") => Some(Value::Int(items.len() as i64)),
            (Value::Vec(items), "is_empty") => Some(Value::Bool(items.is_empty())),
            (Value::Vec(_), "push") => {
                // push mutates — handled specially by the caller
                None
            }
            // String methods
            (Value::String(s), "len") => Some(Value::Int(s.len() as i64)),
            (Value::String(s), "is_empty") => Some(Value::Bool(s.is_empty())),
            (Value::String(s), "contains") => {
                if let Some(Value::String(substr)) = args.first() {
                    Some(Value::Bool(s.contains(substr.as_str())))
                } else {
                    Some(Value::Bool(false))
                }
            }
            (Value::String(s), "to_uppercase") => Some(Value::String(s.to_uppercase())),
            (Value::String(s), "to_lowercase") => Some(Value::String(s.to_lowercase())),
            (Value::String(s), "trim") => Some(Value::String(s.trim().to_string())),
            _ => None,
        }
    }

    // ================================================================
    // Statement Execution
    // ================================================================

    fn exec_body(&mut self, stmts: &[&'a Statement<'a>]) -> ControlFlow {
        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == len - 1;
            let flow = self.exec_statement(stmt, is_last);
            match flow {
                ControlFlow::Continue => {}
                other => return other,
            }
        }
        ControlFlow::Continue
    }

    /// Execute a block body without implicit return on the last expression.
    /// Used for if/while/for/loop blocks where the last expression is NOT
    /// an implicit function return.
    fn exec_body_no_implicit_return(&mut self, stmts: &[&'a Statement<'a>]) -> ControlFlow {
        for stmt in stmts {
            let flow = self.exec_statement(stmt, false);
            match flow {
                ControlFlow::Continue => {}
                other => return other,
            }
        }
        ControlFlow::Continue
    }

    fn exec_statement(&mut self, stmt: &'a Statement<'a>, is_last: bool) -> ControlFlow {
        match stmt {
            Statement::Let {
                pattern,
                value,
                ..
            } => {
                let val = self.eval_expression(value);
                self.bind_pattern(pattern, val);
                ControlFlow::Continue
            }

            Statement::Assignment {
                target,
                value,
                compound_op,
                ..
            } => {
                let new_val = self.eval_expression(value);
                self.exec_assignment(target, new_val, compound_op.as_ref());
                ControlFlow::Continue
            }

            Statement::Expression { expr, .. } => {
                let val = self.eval_expression(expr);
                if is_last {
                    // Last expression is implicit return
                    ControlFlow::Return(val)
                } else {
                    ControlFlow::Continue
                }
            }

            Statement::Return { value, .. } => {
                let val = match value {
                    Some(expr) => self.eval_expression(expr),
                    None => Value::Unit,
                };
                ControlFlow::Return(val)
            }

            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let cond_val = self.eval_expression(condition);
                // When is_last is true, the if/else is the last expression
                // in a function body — it should produce an implicit return.
                // Otherwise, it's a plain statement and shouldn't.
                if cond_val.is_truthy() {
                    self.env.push_scope();
                    let flow = if is_last {
                        self.exec_body(then_block)
                    } else {
                        self.exec_body_no_implicit_return(then_block)
                    };
                    self.env.pop_scope();
                    flow
                } else if let Some(else_stmts) = else_block {
                    self.env.push_scope();
                    let flow = if is_last {
                        self.exec_body(else_stmts)
                    } else {
                        self.exec_body_no_implicit_return(else_stmts)
                    };
                    self.env.pop_scope();
                    flow
                } else {
                    ControlFlow::Continue
                }
            }

            Statement::While {
                condition, body, ..
            } => {
                loop {
                    let cond_val = self.eval_expression(condition);
                    if !cond_val.is_truthy() {
                        break;
                    }
                    self.env.push_scope();
                    let flow = self.exec_body_no_implicit_return(body);
                    self.env.pop_scope();
                    match flow {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return ControlFlow::Return(v),
                        ControlFlow::LoopContinue | ControlFlow::Continue => {}
                    }
                }
                ControlFlow::Continue
            }

            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                let iter_val = self.eval_expression(iterable);
                let items = self.value_to_iterable(iter_val);

                for item in items {
                    self.env.push_scope();
                    self.bind_pattern(pattern, item);
                    let flow = self.exec_body_no_implicit_return(body);
                    self.env.pop_scope();
                    match flow {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return ControlFlow::Return(v),
                        ControlFlow::LoopContinue | ControlFlow::Continue => {}
                    }
                }
                ControlFlow::Continue
            }

            Statement::Loop { body, .. } => {
                loop {
                    self.env.push_scope();
                    let flow = self.exec_body_no_implicit_return(body);
                    self.env.pop_scope();
                    match flow {
                        ControlFlow::Break => break,
                        ControlFlow::Return(v) => return ControlFlow::Return(v),
                        ControlFlow::LoopContinue | ControlFlow::Continue => {}
                    }
                }
                ControlFlow::Continue
            }

            Statement::Break { .. } => ControlFlow::Break,
            Statement::Continue { .. } => ControlFlow::LoopContinue,

            Statement::Match { value, arms, .. } => {
                let val = self.eval_expression(value);
                self.exec_match(&val, arms, is_last)
            }

            _ => ControlFlow::Continue,
        }
    }

    // ================================================================
    // Assignment
    // ================================================================

    fn exec_assignment(
        &mut self,
        target: &'a Expression<'a>,
        new_val: Value,
        compound_op: Option<&CompoundOp>,
    ) {
        match target {
            Expression::Identifier { name, .. } => {
                let final_val = if let Some(op) = compound_op {
                    let old = self.env.get(name).cloned().unwrap_or(Value::Int(0));
                    apply_compound_op_static(&old, &new_val, op)
                } else {
                    new_val
                };
                self.env.set(name, final_val);
            }
            Expression::FieldAccess { object, field, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    let var_name = name.clone();
                    let field_name = field.clone();
                    // Compute the final value before borrowing env mutably
                    let final_val = if let Some(op) = compound_op {
                        let old = self
                            .env
                            .get(&var_name)
                            .and_then(|v| {
                                if let Value::Struct { fields, .. } = v {
                                    fields.get(&field_name).cloned()
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(Value::Int(0));
                        apply_compound_op_static(&old, &new_val, op)
                    } else {
                        new_val
                    };
                    if let Some(obj) = self.env.get_mut(&var_name) {
                        if let Value::Struct { fields, .. } = obj {
                            fields.insert(field_name, final_val);
                        }
                    }
                }
            }
            Expression::Index { object, index, .. } => {
                let idx = self.eval_expression(index);
                if let Expression::Identifier { name, .. } = &**object {
                    let var_name = name.clone();
                    if let Some(idx_val) = idx.as_int() {
                        let i = idx_val as usize;
                        let final_val = if let Some(op) = compound_op {
                            let old = self
                                .env
                                .get(&var_name)
                                .and_then(|v| {
                                    if let Value::Vec(items) = v {
                                        items.get(i).cloned()
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(Value::Int(0));
                            apply_compound_op_static(&old, &new_val, op)
                        } else {
                            new_val
                        };
                        if let Some(obj) = self.env.get_mut(&var_name) {
                            if let Value::Vec(items) = obj {
                                if i < items.len() {
                                    items[i] = final_val;
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // apply_compound_op moved to free function to avoid borrow conflicts

    // ================================================================
    // Pattern Matching
    // ================================================================

    fn bind_pattern(&mut self, pattern: &Pattern, value: Value) {
        match pattern {
            Pattern::Identifier(name) => {
                self.env.define(name, value);
            }
            Pattern::Wildcard => {} // Discard
            Pattern::Tuple(patterns) => {
                if let Value::Tuple(items) = value {
                    for (pat, val) in patterns.iter().zip(items.into_iter()) {
                        self.bind_pattern(pat, val);
                    }
                }
            }
            Pattern::EnumVariant(_, _) => {
                // Delegate to bind_match_pattern for nested enum destructuring
                self.bind_match_pattern(pattern, &value);
            }
            _ => {}
        }
    }

    fn exec_match(&mut self, value: &Value, arms: &[MatchArm<'a>], is_last: bool) -> ControlFlow {
        for arm in arms {
            if self.pattern_matches(&arm.pattern, value) {
                // Bind pattern variables BEFORE checking guard
                // (guards like `n if n > 0` need `n` to be bound)
                self.env.push_scope();
                self.bind_match_pattern(&arm.pattern, value);

                // Check guard (with pattern variables in scope)
                if let Some(guard) = &arm.guard {
                    let guard_val = self.eval_expression(guard);
                    if !guard_val.is_truthy() {
                        self.env.pop_scope();
                        continue;
                    }
                }

                let result = self.eval_expression(arm.body);
                self.env.pop_scope();
                if is_last {
                    return ControlFlow::Return(result);
                } else {
                    return ControlFlow::Continue;
                }
            }
        }
        ControlFlow::Continue
    }

    fn pattern_matches(&self, pattern: &Pattern, value: &Value) -> bool {
        match pattern {
            Pattern::Wildcard => true,
            Pattern::Identifier(_) => true, // Always matches, binds the value
            Pattern::Literal(lit) => {
                let lit_val = self.literal_to_value(lit);
                lit_val == *value
            }
            Pattern::EnumVariant(full_path, binding) => {
                if let Value::Enum {
                    type_name,
                    variant,
                    data,
                } = value
                {
                    // Match the variant name
                    let expected_full = format!("{}::{}", type_name, variant);
                    let variant_matches = full_path == &expected_full
                        || full_path
                            .rsplit("::")
                            .next()
                            .map_or(false, |pat_variant| pat_variant == variant);

                    if !variant_matches {
                        return false;
                    }

                    // Also check nested patterns inside the binding
                    match (binding, data) {
                        (
                            crate::parser::EnumPatternBinding::Tuple(pats),
                            EnumData::Tuple(vals),
                        ) => {
                            // All inner patterns must also match
                            pats.iter()
                                .zip(vals.iter())
                                .all(|(p, v)| self.pattern_matches(p, v))
                        }
                        (crate::parser::EnumPatternBinding::None, EnumData::Unit) => true,
                        (crate::parser::EnumPatternBinding::None, _) => true,
                        (crate::parser::EnumPatternBinding::Wildcard, _) => true,
                        (crate::parser::EnumPatternBinding::Single(_), _) => true,
                        _ => true,
                    }
                } else {
                    false
                }
            }
            Pattern::Tuple(patterns) => {
                if let Value::Tuple(items) = value {
                    patterns.len() == items.len()
                        && patterns
                            .iter()
                            .zip(items.iter())
                            .all(|(p, v)| self.pattern_matches(p, v))
                } else {
                    false
                }
            }
            Pattern::Or(patterns) => patterns.iter().any(|p| self.pattern_matches(p, value)),
            Pattern::Reference(inner) => self.pattern_matches(inner, value),
        }
    }

    fn bind_match_pattern(&mut self, pattern: &Pattern, value: &Value) {
        match pattern {
            Pattern::Identifier(name) => {
                self.env.define(name, value.clone());
            }
            Pattern::EnumVariant(_, binding) => {
                // Bind enum variant data if needed
                if let Value::Enum { data, .. } = value {
                    match (binding, data) {
                        (
                            crate::parser::EnumPatternBinding::Tuple(pats),
                            EnumData::Tuple(vals),
                        ) => {
                            for (name_pat, val) in pats.iter().zip(vals.iter()) {
                                self.bind_pattern(name_pat, val.clone());
                            }
                        }
                        (
                            crate::parser::EnumPatternBinding::Single(name),
                            EnumData::Tuple(vals),
                        ) => {
                            if let Some(val) = vals.first() {
                                self.env.define(name, val.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // ================================================================
    // Expression Evaluation
    // ================================================================

    fn eval_expression(&mut self, expr: &'a Expression<'a>) -> Value {
        match expr {
            Expression::Literal { value, .. } => self.literal_to_value(value),

            Expression::Identifier { name, .. } => {
                // Check environment first
                if let Some(val) = self.env.get(name) {
                    return val.clone();
                }
                // Check if this is a unit enum variant (e.g., "Color::Red")
                if let Some(info) = self.enum_variants.get(name) {
                    if matches!(info.data_kind, EnumVariantKind::Unit) {
                        return Value::Enum {
                            type_name: info.enum_name.clone(),
                            variant: info.variant_name.clone(),
                            data: EnumData::Unit,
                        };
                    }
                }
                // Fallback: string (for unknown identifiers)
                Value::String(name.clone())
            }

            Expression::Binary {
                left, op, right, ..
            } => {
                let left_val = self.eval_expression(left);
                // Short-circuit for && and ||
                match op {
                    BinaryOp::And => {
                        if !left_val.is_truthy() {
                            return Value::Bool(false);
                        }
                        let right_val = self.eval_expression(right);
                        return Value::Bool(right_val.is_truthy());
                    }
                    BinaryOp::Or => {
                        if left_val.is_truthy() {
                            return Value::Bool(true);
                        }
                        let right_val = self.eval_expression(right);
                        return Value::Bool(right_val.is_truthy());
                    }
                    _ => {}
                }
                let right_val = self.eval_expression(right);
                self.binary_op(&left_val, &right_val, op)
            }

            Expression::Unary { op, operand, .. } => {
                let val = self.eval_expression(operand);
                match op {
                    UnaryOp::Neg => match val {
                        Value::Int(n) => Value::Int(-n),
                        Value::Float(f) => Value::Float(-f),
                        _ => Value::Nil,
                    },
                    UnaryOp::Not => Value::Bool(!val.is_truthy()),
                    _ => val,
                }
            }

            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Expression::Identifier { name, .. } = &**function {
                    let args: Vec<Value> = arguments
                        .iter()
                        .map(|(_, arg)| self.eval_expression(arg))
                        .collect();

                    // Check built-in functions
                    match name.as_str() {
                        "println" => {
                            self.builtin_println(&args);
                            return Value::Unit;
                        }
                        "print" => {
                            self.builtin_print(&args);
                            return Value::Unit;
                        }
                        _ => {}
                    }

                    // Check if this is an enum tuple variant constructor (e.g., "Shape::Circle")
                    if let Some(info) = self.enum_variants.get(name.as_str()).cloned() {
                        if let EnumVariantKind::Tuple(_) = &info.data_kind {
                            return Value::Enum {
                                type_name: info.enum_name.clone(),
                                variant: info.variant_name.clone(),
                                data: EnumData::Tuple(args),
                            };
                        }
                    }

                    // Regular function call (includes static methods like Player::new)
                    return self
                        .call_function(name, &args)
                        .unwrap_or(Value::Nil);
                }

                // General call (function value)
                let func_val = self.eval_expression(function);
                let args: Vec<Value> = arguments
                    .iter()
                    .map(|(_, arg)| self.eval_expression(arg))
                    .collect();
                if let Value::Function(fv) = func_val {
                    self.call_function(&fv.name, &args).unwrap_or(Value::Nil)
                } else {
                    Value::Nil
                }
            }

            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let obj_val = self.eval_expression(object);
                let args: Vec<Value> = arguments
                    .iter()
                    .map(|(_, arg)| self.eval_expression(arg))
                    .collect();

                let type_name = match &obj_val {
                    Value::Struct { type_name, .. } => type_name.clone(),
                    Value::Vec(_) => "Vec".to_string(),
                    Value::String(_) => "String".to_string(),
                    _ => obj_val.type_name().to_string(),
                };

                // Handle push specially (mutates the receiver)
                if method == "push" {
                    self.handle_push_method(object, &args);
                    return Value::Unit;
                }

                // Call the method, capturing any mutations to self
                let result = self
                    .call_method_with_self_mutation(object, &obj_val, &type_name, method, &args)
                    .unwrap_or(Value::Nil);

                result
            }

            Expression::FieldAccess { object, field, .. } => {
                let obj_val = self.eval_expression(object);
                match obj_val {
                    Value::Struct { fields, .. } => {
                        fields.get(field).cloned().unwrap_or(Value::Nil)
                    }
                    _ => Value::Nil,
                }
            }

            Expression::Index { object, index, .. } => {
                let obj_val = self.eval_expression(object);
                let idx_val = self.eval_expression(index);
                match (obj_val, idx_val) {
                    (Value::Vec(items), Value::Int(i)) => {
                        items.get(i as usize).cloned().unwrap_or(Value::Nil)
                    }
                    (Value::String(s), Value::Int(i)) => s
                        .chars()
                        .nth(i as usize)
                        .map(Value::Char)
                        .unwrap_or(Value::Nil),
                    _ => Value::Nil,
                }
            }

            Expression::StructLiteral { name, fields, .. } => {
                let mut field_map = HashMap::new();
                for (fname, fexpr) in fields {
                    field_map.insert(fname.clone(), self.eval_expression(fexpr));
                }
                Value::Struct {
                    type_name: name.clone(),
                    fields: field_map,
                }
            }

            Expression::Array { elements, .. } => {
                let items: Vec<Value> = elements
                    .iter()
                    .map(|e| self.eval_expression(e))
                    .collect();
                Value::Vec(items)
            }

            Expression::Tuple { elements, .. } => {
                let items: Vec<Value> = elements
                    .iter()
                    .map(|e| self.eval_expression(e))
                    .collect();
                if items.is_empty() {
                    Value::Unit
                } else {
                    Value::Tuple(items)
                }
            }

            Expression::Range { start, end, .. } => {
                let start_val = self.eval_expression(start);
                let end_val = self.eval_expression(end);
                if let (Some(s), Some(e)) = (start_val.as_int(), end_val.as_int()) {
                    Value::Vec((s..e).map(Value::Int).collect())
                } else {
                    Value::Vec(vec![])
                }
            }

            Expression::Closure {
                parameters, body: _, ..
            } => {
                // For now, store as a named function reference
                // This is a simplification — real closures need captured state
                let closure_name = format!("__closure_{}", self.functions.len());
                Value::Function(FunctionValue {
                    name: closure_name,
                    params: parameters.clone(),
                    body_id: 0,
                })
            }

            Expression::Block { statements, .. } => {
                self.env.push_scope();
                let mut result = Value::Unit;
                let len = statements.len();
                for (i, stmt) in statements.iter().enumerate() {
                    let is_last = i == len - 1;
                    let flow = self.exec_statement(stmt, is_last);
                    match flow {
                        ControlFlow::Return(val) => {
                            result = val;
                            break;
                        }
                        ControlFlow::Continue => {}
                        other => {
                            self.env.pop_scope();
                            match other {
                                ControlFlow::Break => return Value::Unit,
                                _ => return Value::Unit,
                            }
                        }
                    }
                }
                self.env.pop_scope();
                result
            }

            Expression::Cast { expr, .. } => {
                // For the interpreter, casts are mostly no-ops
                self.eval_expression(expr)
            }

            Expression::MacroInvocation { name, args, .. } => {
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|a| self.eval_expression(a))
                    .collect();

                match name.as_str() {
                    "println" => {
                        self.builtin_println(&evaluated_args);
                        Value::Unit
                    }
                    "print" => {
                        self.builtin_print(&evaluated_args);
                        Value::Unit
                    }
                    "vec" => Value::Vec(evaluated_args),
                    "format" => {
                        let s = self.format_string(&evaluated_args);
                        Value::String(s)
                    }
                    _ => Value::Nil,
                }
            }

            _ => Value::Nil,
        }
    }

    // ================================================================
    // Binary Operations
    // ================================================================

    fn binary_op(&self, left: &Value, right: &Value, op: &BinaryOp) -> Value {
        // String concatenation
        if let BinaryOp::Add = op {
            if let (Value::String(a), Value::String(b)) = (left, right) {
                return Value::String(format!("{}{}", a, b));
            }
        }

        // Integer operations
        if let (Some(a), Some(b)) = (left.as_int(), right.as_int()) {
            // Check if either side is explicitly float
            let either_float =
                matches!(left, Value::Float(_)) || matches!(right, Value::Float(_));
            if !either_float {
                return match op {
                    BinaryOp::Add => Value::Int(a + b),
                    BinaryOp::Sub => Value::Int(a - b),
                    BinaryOp::Mul => Value::Int(a * b),
                    BinaryOp::Div => {
                        if b == 0 {
                            Value::Int(0)
                        } else {
                            Value::Int(a / b)
                        }
                    }
                    BinaryOp::Mod => {
                        if b == 0 {
                            Value::Int(0)
                        } else {
                            Value::Int(a % b)
                        }
                    }
                    BinaryOp::Eq => Value::Bool(a == b),
                    BinaryOp::Ne => Value::Bool(a != b),
                    BinaryOp::Lt => Value::Bool(a < b),
                    BinaryOp::Le => Value::Bool(a <= b),
                    BinaryOp::Gt => Value::Bool(a > b),
                    BinaryOp::Ge => Value::Bool(a >= b),
                    BinaryOp::BitAnd => Value::Int(a & b),
                    BinaryOp::BitOr => Value::Int(a | b),
                    BinaryOp::BitXor => Value::Int(a ^ b),
                    BinaryOp::Shl => Value::Int(a << b),
                    BinaryOp::Shr => Value::Int(a >> b),
                    _ => Value::Nil,
                };
            }
        }

        // Float operations
        if let (Some(a), Some(b)) = (left.as_float(), right.as_float()) {
            return match op {
                BinaryOp::Add => Value::Float(a + b),
                BinaryOp::Sub => Value::Float(a - b),
                BinaryOp::Mul => Value::Float(a * b),
                BinaryOp::Div => Value::Float(a / b),
                BinaryOp::Mod => Value::Float(a % b),
                BinaryOp::Eq => Value::Bool(a == b),
                BinaryOp::Ne => Value::Bool(a != b),
                BinaryOp::Lt => Value::Bool(a < b),
                BinaryOp::Le => Value::Bool(a <= b),
                BinaryOp::Gt => Value::Bool(a > b),
                BinaryOp::Ge => Value::Bool(a >= b),
                _ => Value::Nil,
            };
        }

        // Boolean operations
        if let (Value::Bool(a), Value::Bool(b)) = (left, right) {
            return match op {
                BinaryOp::Eq => Value::Bool(a == b),
                BinaryOp::Ne => Value::Bool(a != b),
                _ => Value::Nil,
            };
        }

        // String comparisons
        if let (Value::String(a), Value::String(b)) = (left, right) {
            return match op {
                BinaryOp::Eq => Value::Bool(a == b),
                BinaryOp::Ne => Value::Bool(a != b),
                BinaryOp::Lt => Value::Bool(a < b),
                BinaryOp::Le => Value::Bool(a <= b),
                BinaryOp::Gt => Value::Bool(a > b),
                BinaryOp::Ge => Value::Bool(a >= b),
                _ => Value::Nil,
            };
        }

        Value::Nil
    }

    // ================================================================
    // Built-in Functions
    // ================================================================

    fn builtin_println(&mut self, args: &[Value]) {
        let text = self.format_string(args);
        if self.capture_output {
            self.output.push(format!("{}\n", text));
        } else {
            println!("{}", text);
        }
    }

    fn builtin_print(&mut self, args: &[Value]) {
        let text = self.format_string(args);
        if self.capture_output {
            self.output.push(text);
        } else {
            print!("{}", text);
            let _ = std::io::stdout().flush();
        }
    }

    /// Format a string with {} placeholders, matching Windjammer/Rust behavior
    fn format_string(&self, args: &[Value]) -> String {
        if args.is_empty() {
            return String::new();
        }

        // If first argument is a string with {} placeholders
        if let Value::String(fmt_str) = &args[0] {
            if fmt_str.contains("{}") {
                let mut result = fmt_str.clone();
                for arg in &args[1..] {
                    if let Some(pos) = result.find("{}") {
                        let replacement = arg.to_display_string();
                        result = format!(
                            "{}{}{}",
                            &result[..pos],
                            replacement,
                            &result[pos + 2..]
                        );
                    }
                }
                return result;
            }
        }

        // No format string — just concatenate all args with spaces
        if args.len() == 1 {
            args[0].to_display_string()
        } else {
            args.iter()
                .map(|a| a.to_display_string())
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    // ================================================================
    // Helpers
    // ================================================================

    fn literal_to_value(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Int(n) => Value::Int(*n),
            Literal::Float(f) => Value::Float(*f),
            Literal::Bool(b) => Value::Bool(*b),
            Literal::String(s) => Value::String(s.clone()),
            Literal::Char(c) => Value::Char(*c),
        }
    }

    fn value_to_iterable(&self, value: Value) -> Vec<Value> {
        match value {
            Value::Vec(items) => items,
            Value::String(s) => s.chars().map(Value::Char).collect(),
            _ => vec![],
        }
    }

    fn handle_push_method(&mut self, object: &Expression, args: &[Value]) {
        if let Expression::Identifier { name, .. } = object {
            if let Some(val) = self.env.get_mut(name) {
                if let Value::Vec(items) = val {
                    if let Some(arg) = args.first() {
                        items.push(arg.clone());
                    }
                }
            }
        }
    }

    /// Call a method and propagate any mutations to `self` back to the receiver
    fn call_method_with_self_mutation(
        &mut self,
        receiver_expr: &'a Expression<'a>,
        receiver_val: &Value,
        type_name: &str,
        method_name: &str,
        args: &[Value],
    ) -> Result<Value, String> {
        // Check built-in methods first
        if let Some(result) = self.call_builtin_method(receiver_val, method_name, args) {
            return Ok(result);
        }

        let method_key = format!("{}::{}", type_name, method_name);
        let decl = {
            let func_defs = self
                .functions
                .get(&method_key)
                .ok_or_else(|| format!("Undefined method: {}.{}", type_name, method_name))?;
            let func_def = func_defs
                .first()
                .ok_or_else(|| format!("No definition for method: {}", method_key))?;
            func_def.decl
        };

        self.env.push_scope();

        // Bind self
        self.env.define("self", receiver_val.clone());

        // Bind other parameters
        let param_iter = decl
            .parameters
            .iter()
            .filter(|p| p.name != "self");

        for (param, arg) in param_iter.zip(args.iter()) {
            self.env.define(&param.name, arg.clone());
        }

        let result = self.exec_body(&decl.body);

        // Read back the possibly-mutated self
        let mutated_self = self.env.get("self").cloned();

        self.env.pop_scope();

        // Propagate self mutations back to the original variable
        if let Some(new_self) = mutated_self {
            if let Expression::Identifier { name, .. } = receiver_expr {
                self.env.set(name, new_self);
            }
        }

        match result {
            ControlFlow::Return(val) => Ok(val),
            _ => Ok(Value::Unit),
        }
    }
}

/// Apply a compound operation (+=, -=, etc.) without borrowing the interpreter
fn apply_compound_op_static(left: &Value, right: &Value, op: &CompoundOp) -> Value {
    let bin_op = match op {
        CompoundOp::Add => BinaryOp::Add,
        CompoundOp::Sub => BinaryOp::Sub,
        CompoundOp::Mul => BinaryOp::Mul,
        CompoundOp::Div => BinaryOp::Div,
        CompoundOp::Mod => BinaryOp::Mod,
        CompoundOp::BitAnd => BinaryOp::BitAnd,
        CompoundOp::BitOr => BinaryOp::BitOr,
        CompoundOp::BitXor => BinaryOp::BitXor,
        CompoundOp::Shl => BinaryOp::Shl,
        CompoundOp::Shr => BinaryOp::Shr,
    };
    // Inline the binary_op logic for the common cases
    if let (Some(a), Some(b)) = (left.as_int(), right.as_int()) {
        return match bin_op {
            BinaryOp::Add => Value::Int(a + b),
            BinaryOp::Sub => Value::Int(a - b),
            BinaryOp::Mul => Value::Int(a * b),
            BinaryOp::Div => Value::Int(if b != 0 { a / b } else { 0 }),
            BinaryOp::Mod => Value::Int(if b != 0 { a % b } else { 0 }),
            BinaryOp::BitAnd => Value::Int(a & b),
            BinaryOp::BitOr => Value::Int(a | b),
            BinaryOp::BitXor => Value::Int(a ^ b),
            BinaryOp::Shl => Value::Int(a << b),
            BinaryOp::Shr => Value::Int(a >> b),
            _ => Value::Nil,
        };
    }
    if let (Some(a), Some(b)) = (left.as_float(), right.as_float()) {
        return match bin_op {
            BinaryOp::Add => Value::Float(a + b),
            BinaryOp::Sub => Value::Float(a - b),
            BinaryOp::Mul => Value::Float(a * b),
            BinaryOp::Div => Value::Float(a / b),
            BinaryOp::Mod => Value::Float(a % b),
            _ => Value::Nil,
        };
    }
    Value::Nil
}
