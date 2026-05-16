//! Statement execution, pattern matching, and assignments for the interpreter.

use super::engine::ControlFlow;
use super::engine::Interpreter;
use super::value::{EnumData, Value};
use super::value_operations::{apply_compound_op_static, literal_to_value, value_to_iterable};
use crate::parser::{CompoundOp, Expression, MatchArm, Pattern, Statement};

impl<'a> Interpreter<'a> {
    pub(crate) fn exec_body(&mut self, stmts: &[&'a Statement<'a>]) -> ControlFlow {
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
    pub(crate) fn exec_body_no_implicit_return(
        &mut self,
        stmts: &[&'a Statement<'a>],
    ) -> ControlFlow {
        for stmt in stmts {
            let flow = self.exec_statement(stmt, false);
            match flow {
                ControlFlow::Continue => {}
                other => return other,
            }
        }
        ControlFlow::Continue
    }

    pub(crate) fn exec_statement(&mut self, stmt: &'a Statement<'a>, is_last: bool) -> ControlFlow {
        match stmt {
            Statement::Let { pattern, value, .. } => {
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
                let items = value_to_iterable(iter_val);

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

    #[allow(clippy::collapsible_match)]
    pub(crate) fn exec_assignment(
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
                    if let Some(Value::Struct { fields, .. }) = self.env.get_mut(&var_name) {
                        fields.insert(field_name, final_val);
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
                        if let Some(Value::Vec(items)) = self.env.get_mut(&var_name) {
                            if i < items.len() {
                                items[i] = final_val;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn bind_pattern(&mut self, pattern: &Pattern, value: Value) {
        match pattern {
            Pattern::Identifier(name) => {
                self.env.define(name, value);
            }
            Pattern::Wildcard => {}
            Pattern::Tuple(patterns) => {
                if let Value::Tuple(items) = value {
                    for (pat, val) in patterns.iter().zip(items) {
                        self.bind_pattern(pat, val);
                    }
                }
            }
            Pattern::EnumVariant(_, _) => {
                self.bind_match_pattern(pattern, &value);
            }
            _ => {}
        }
    }

    pub(crate) fn exec_match(
        &mut self,
        value: &Value,
        arms: &[MatchArm<'a>],
        is_last: bool,
    ) -> ControlFlow {
        for arm in arms {
            if self.pattern_matches(&arm.pattern, value) {
                self.env.push_scope();
                self.bind_match_pattern(&arm.pattern, value);

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

    pub(crate) fn pattern_matches(&self, pattern: &Pattern, value: &Value) -> bool {
        match pattern {
            Pattern::Wildcard => true,
            Pattern::Identifier(_) | Pattern::MutBinding(_) => true,
            Pattern::Literal(lit) => {
                let lit_val = literal_to_value(lit);
                lit_val == *value
            }
            Pattern::EnumVariant(full_path, binding) => {
                if let Value::Enum {
                    type_name,
                    variant,
                    data,
                } = value
                {
                    let expected_full = format!("{}::{}", type_name, variant);
                    let variant_matches = full_path == &expected_full
                        || full_path
                            .rsplit("::")
                            .next()
                            .is_some_and(|pat_variant| pat_variant == variant);

                    if !variant_matches {
                        return false;
                    }

                    match (binding, data) {
                        (crate::parser::EnumPatternBinding::Tuple(pats), EnumData::Tuple(vals)) => {
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
            Pattern::Ref(_) | Pattern::RefMut(_) => true,
        }
    }

    pub(crate) fn bind_match_pattern(&mut self, pattern: &Pattern, value: &Value) {
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => {
                self.env.define(name, value.clone());
            }
            Pattern::EnumVariant(_, binding) => {
                if let Value::Enum { data, .. } = value {
                    match (binding, data) {
                        (crate::parser::EnumPatternBinding::Tuple(pats), EnumData::Tuple(vals)) => {
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
}
