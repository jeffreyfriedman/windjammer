// MODULE: pipe operator (`|>`) parsing and desugaring.
//
// Responsibility: parse the stages of `lhs |> stage |> stage ...` and rewrite each
// one into an ordinary `Expression::Call` / `Expression::MethodCall`. This is the
// ONLY place the pipe operator is desugared; the analyzer, IR and all backends see
// plain calls. Spec: docs/design/pipe-operator.md.
//
// Entry points (both `impl Parser`):
// - `parse_pipe_expression`: lowest-precedence expression level (wraps the binary
//   operator parser), used by `parse_expression`.
// - `parse_pipe_stages`: consume zero or more `|> stage` suffixes after an already
//   parsed left-hand side (also used for `match` arm values).
//
// Placeholder `_`: `parse_primary_expression` only yields an `Identifier("_")` while
// `in_pipe_stage` is set; each one bumps `pipe_placeholders`, which lets this module
// reject placeholders that are not direct call arguments.
//
// Dependencies: `binary_expression_parser` (operands), `call_expression_parser`
// (`parse_postfix_expression` for the `.method()` stage form).

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

type Args = Vec<(Option<String>, &'static Expression<'static>)>;

impl Parser {
    pub(in crate::parser) fn parse_pipe_expression(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        if self.current_token() == &Token::PipeOp {
            return Err(self.pipe_error(
                self.position,
                "`|>` needs a left-hand value (a pipe cannot start an expression)",
            ));
        }
        let left = self.parse_binary_expression(0)?;
        self.parse_pipe_stages(left)
    }

    pub(in crate::parser) fn parse_pipe_stages(
        &mut self,
        mut left: &'static Expression<'static>,
    ) -> Result<&'static Expression<'static>, String> {
        while self.current_token() == &Token::PipeOp {
            let pipe_pos = self.position;
            self.advance();
            if matches!(
                self.current_token(),
                Token::Eof
                    | Token::RBrace
                    | Token::RParen
                    | Token::RBracket
                    | Token::Comma
                    | Token::Semicolon
                    | Token::PipeOp
            ) {
                return Err(self.pipe_error(pipe_pos, "expected a stage after `|>`"));
            }
            left = self.parse_pipe_stage(left)?;
        }
        Ok(left)
    }

    fn parse_pipe_stage(
        &mut self,
        piped: &'static Expression<'static>,
    ) -> Result<&'static Expression<'static>, String> {
        let stage_pos = self.position;
        let saved_flag = self.in_pipe_stage;
        let saved_count = self.pipe_placeholders;
        self.in_pipe_stage = true;
        self.pipe_placeholders = 0;

        let result = if self.current_token() == &Token::Dot {
            // Method form: `x |> .method(a)` is `x.method(a)` (and may continue
            // with further postfix operators such as `?` or `.other()`).
            self.parse_postfix_expression(piped).map(|e| (e, 0usize))
        } else {
            self.parse_primary_expression()
                .and_then(|stage| self.insert_piped(piped, stage, stage_pos))
        };

        let total = self.pipe_placeholders;
        self.in_pipe_stage = saved_flag;
        self.pipe_placeholders = saved_count;

        let (expr, direct) = result?;
        if total > direct {
            return Err(self.pipe_error(
                stage_pos,
                "placeholder `_` must be a direct argument of the stage call (e.g. `f(a, _)`)",
            ));
        }
        Ok(expr)
    }

    /// Rewrites `piped |> stage` into a plain call. Returns the expression and the
    /// number of direct-argument placeholders that were consumed.
    fn insert_piped(
        &mut self,
        piped: &'static Expression<'static>,
        stage: &'static Expression<'static>,
        stage_pos: usize,
    ) -> Result<(&'static Expression<'static>, usize), String> {
        match stage {
            Expression::TryOp { expr, location } => {
                let (inner, n) = self.insert_piped(piped, expr, stage_pos)?;
                Ok((
                    self.alloc_expr(Expression::TryOp {
                        expr: inner,
                        location: location.clone(),
                    }),
                    n,
                ))
            }
            Expression::Await { expr, location } => {
                let (inner, n) = self.insert_piped(piped, expr, stage_pos)?;
                Ok((
                    self.alloc_expr(Expression::Await {
                        expr: inner,
                        location: location.clone(),
                    }),
                    n,
                ))
            }
            Expression::Call {
                function,
                arguments,
                location,
            } => {
                let (args, n) = self.place_piped(piped, arguments, stage_pos)?;
                Ok((
                    self.alloc_expr(Expression::Call {
                        function,
                        arguments: args,
                        location: location.clone(),
                    }),
                    n,
                ))
            }
            // `x |> module.f(a)`: the piped value becomes the first argument.
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                location,
            } => {
                let (args, n) = self.place_piped(piped, arguments, stage_pos)?;
                Ok((
                    self.alloc_expr(Expression::MethodCall {
                        object,
                        method: method.clone(),
                        type_args: type_args.clone(),
                        arguments: args,
                        location: location.clone(),
                    }),
                    n,
                ))
            }
            Expression::Identifier { name, .. } if name == "_" => Err(self.pipe_error(
                stage_pos,
                "placeholder `_` cannot be the whole stage; use a call such as `f(a, _)`",
            )),
            Expression::Identifier { .. }
            | Expression::FieldAccess { .. }
            | Expression::Closure { .. } => Ok((
                self.alloc_expr(Expression::Call {
                    function: stage,
                    arguments: vec![(None, piped)],
                    location: stage.location(),
                }),
                0,
            )),
            _ => Err(self.pipe_error(
                stage_pos,
                "right-hand side of `|>` must be a function, call, method (`.name(..)`) or closure",
            )),
        }
    }

    /// Put `piped` at the single `_` placeholder, or first when there is none.
    fn place_piped(
        &self,
        piped: &'static Expression<'static>,
        arguments: &Args,
        stage_pos: usize,
    ) -> Result<(Args, usize), String> {
        let is_hole =
            |e: &Expression| matches!(e, Expression::Identifier { name, .. } if name == "_");
        let holes = arguments.iter().filter(|(_, e)| is_hole(e)).count();
        match holes {
            0 => {
                let mut args = Vec::with_capacity(arguments.len() + 1);
                args.push((None, piped));
                args.extend(arguments.iter().cloned());
                Ok((args, 0))
            }
            1 => Ok((
                arguments
                    .iter()
                    .map(|(l, e)| if is_hole(e) { (l.clone(), piped) } else { (l.clone(), *e) })
                    .collect(),
                1,
            )),
            _ => Err(self.pipe_error(
                stage_pos,
                "a pipe stage may contain at most one `_` placeholder (bind the value with `let` to use it twice)",
            )),
        }
    }

    fn pipe_error(&self, token_index: usize, msg: &str) -> String {
        match self.tokens.get(token_index) {
            Some(t) => format!("{}:{}:{}: error: {}", self.filename, t.line, t.column, msg),
            None => format!("{}: error: {}", self.filename, msg),
        }
    }
}
