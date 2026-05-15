//! Expression evaluation and built-in calls for the interpreter.

use super::engine::{ControlFlow, EnumVariantKind, Interpreter};
use super::value::{EnumData, FunctionValue, Value};
use super::value_operations::{binary_op, literal_to_value};
use crate::parser::{
    BinaryOp, Expression, UnaryOp,
};
use std::collections::HashMap;
use std::io::Write;

impl<'a> Interpreter<'a> {
    pub(crate) fn eval_expression(&mut self, expr: &'a Expression<'a>) -> Value {
        match expr {
            Expression::Literal { value, .. } => literal_to_value(value),

            Expression::Identifier { name, .. } => {
                if let Some(val) = self.env.get(name) {
                    return val.clone();
                }
                if let Some(info) = self.enum_variants.get(name) {
                    if matches!(info.data_kind, EnumVariantKind::Unit) {
                        return Value::Enum {
                            type_name: info.enum_name.clone(),
                            variant: info.variant_name.clone(),
                            data: EnumData::Unit,
                        };
                    }
                }
                Value::String(name.clone())
            }

            Expression::Binary {
                left, op, right, ..
            } => {
                let left_val = self.eval_expression(left);
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
                binary_op(&left_val, &right_val, op)
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

                    if let Some(info) = self.enum_variants.get(name.as_str()).cloned() {
                        if let EnumVariantKind::Tuple { .. } = &info.data_kind {
                            return Value::Enum {
                                type_name: info.enum_name.clone(),
                                variant: info.variant_name.clone(),
                                data: EnumData::Tuple(args),
                            };
                        }
                    }

                    return self.call_function(name, &args).unwrap_or(Value::Nil);
                }

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

                if method == "push" {
                    self.handle_push_method(object, &args);
                    return Value::Unit;
                }

                self.call_method_with_self_mutation(object, &obj_val, &type_name, method, &args)
                    .unwrap_or(Value::Nil)
            }

            Expression::FieldAccess { object, field, .. } => {
                let obj_val = self.eval_expression(object);
                match obj_val {
                    Value::Struct { fields, .. } => fields.get(field).cloned().unwrap_or(Value::Nil),
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
                let items: Vec<Value> = elements.iter().map(|e| self.eval_expression(e)).collect();
                Value::Vec(items)
            }

            Expression::Tuple { elements, .. } => {
                let items: Vec<Value> = elements.iter().map(|e| self.eval_expression(e)).collect();
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
                parameters,
                body: _,
                ..
            } => {
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

            Expression::Cast { expr, .. } => self.eval_expression(expr),

            Expression::MacroInvocation { name, args, .. } => {
                let evaluated_args: Vec<Value> =
                    args.iter().map(|a| self.eval_expression(a)).collect();

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

    pub(crate) fn call_builtin_method(
        &mut self,
        receiver: &Value,
        method: &str,
        args: &[Value],
    ) -> Option<Value> {
        match (receiver, method) {
            (Value::Vec(items), "len") => Some(Value::Int(items.len() as i64)),
            (Value::Vec(items), "is_empty") => Some(Value::Bool(items.is_empty())),
            (Value::Vec(_), "push") => None,
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

    pub(crate) fn builtin_println(&mut self, args: &[Value]) {
        let text = self.format_string(args);
        if self.capture_output {
            self.output.push(format!("{}\n", text));
        } else {
            println!("{}", text);
        }
    }

    pub(crate) fn builtin_print(&mut self, args: &[Value]) {
        let text = self.format_string(args);
        if self.capture_output {
            self.output.push(text);
        } else {
            print!("{}", text);
            let _ = std::io::stdout().flush();
        }
    }

    pub(crate) fn format_string(&self, args: &[Value]) -> String {
        if args.is_empty() {
            return String::new();
        }

        if let Value::String(fmt_str) = &args[0] {
            if fmt_str.contains("{}") {
                let mut result = fmt_str.clone();
                for arg in &args[1..] {
                    if let Some(pos) = result.find("{}") {
                        let replacement = arg.to_display_string();
                        result = format!("{}{}{}", &result[..pos], replacement, &result[pos + 2..]);
                    }
                }
                return result;
            }
        }

        if args.len() == 1 {
            args[0].to_display_string()
        } else {
            args.iter()
                .map(|a| a.to_display_string())
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    pub(crate) fn handle_push_method(&mut self, object: &Expression, args: &[Value]) {
        if let Expression::Identifier { name, .. } = object {
            if let Some(Value::Vec(items)) = self.env.get_mut(name) {
                if let Some(arg) = args.first() {
                    items.push(arg.clone());
                }
            }
        }
    }

    pub(crate) fn call_method_with_self_mutation(
        &mut self,
        receiver_expr: &'a Expression<'a>,
        receiver_val: &Value,
        type_name: &str,
        method_name: &str,
        args: &[Value],
    ) -> Result<Value, String> {
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

        self.env.define("self", receiver_val.clone());

        let param_iter = decl.parameters.iter().filter(|p| p.name != "self");

        for (param, arg) in param_iter.zip(args.iter()) {
            self.env.define(&param.name, arg.clone());
        }

        let result = self.exec_body(&decl.body);

        let mutated_self = self.env.get("self").cloned();

        self.env.pop_scope();

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
