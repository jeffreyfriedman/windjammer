//! `thread { ... }` and `async { ... }` statements: the compiled path emits `std::thread::spawn(move || { ... })` /
//! `tokio::spawn(async move { ... })` and drops the handle. Here the body becomes a `move` closure started as a
//! scheduler task, exactly like `thread::spawn(move || { ... })` (the handle is dropped; nothing joins it).

use super::engine::Interpreter;
use super::error::R;
use super::value::*;
use crate::parser::{Expression, Statement};
use std::rc::Rc;

impl Interpreter {
    pub(crate) fn spawn_block(&mut self, stmt: &'static Statement<'static>, body: &'static [&'static Statement<'static>]) -> R {
        let key = stmt as *const Statement<'static> as usize;
        let block: &'static Expression<'static> = match self.task_blocks.get(&key) {
            Some(b) => b,
            None => {
                // one leaked block per source statement, like the program AST itself
                let b: &'static Expression<'static> = Box::leak(Box::new(Expression::Block {
                    statements: body.to_vec(),
                    is_unsafe: false,
                    location: stmt_location(stmt),
                }));
                self.task_blocks.insert(key, b);
                b
            }
        };
        let f = Value::Func(Rc::new(Callable::Closure {
            params: &[],
            body: block,
            env: self.scope.vars.clone(),
            module: self.scope.module,
            self_ty: self.scope.self_ty.clone(),
            tparams: self.scope.tparams.clone(),
        }));
        let f = super::builtins::thread::move_captures(f);
        self.spawn_task(f, None)?;
        Ok(Value::Unit)
    }
}

fn stmt_location(s: &Statement<'static>) -> Option<crate::source_map::Location> {
    match s {
        Statement::Thread { location, .. } | Statement::Async { location, .. } => location.clone(),
        _ => unreachable!("spawn_block is only called for thread/async statements"),
    }
}
