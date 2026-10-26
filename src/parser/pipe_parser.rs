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
// `|>?` (short-circuiting pipe): `x |>? stage` is `x? |> stage`; the left-hand side is
// wrapped in a `TryOp` before the stage is applied.
//
// Placeholder `_`: `parse_primary_expression` only yields an `Identifier("_")` while
// `in_pipe_stage` is set; each one records its token index in `pipe_placeholders`, so
// misuse (second `_`, nested `_`) is reported at the offending token.
//
// Dependencies: `binary_expression_parser` (operands), `call_expression_parser`
// (`parse_postfix_expression` for the `.method()` stage form).

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;

type Args = Vec<(Option<String>, &'static Expression<'static>)>;
/// Source locations of the `_` placeholders that were direct call arguments.
type Holes = Vec<SourceLocation>;

impl Parser {
    pub(in crate::parser) fn parse_pipe_expression(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        if let Some(op) = self.pipe_op_text() {
            return Err(self.pipe_error(
                self.position,
                &format!("`{op}` needs a left-hand value (a pipe cannot start an expression)"),
            ));
        }
        let left = self.parse_range_expression()?;
        self.parse_pipe_stages(left)
    }

    pub(in crate::parser) fn parse_pipe_stages(
        &mut self,
        mut left: &'static Expression<'static>,
    ) -> Result<&'static Expression<'static>, String> {
        while let Some(op) = self.pipe_op_text() {
            let pipe_pos = self.position;
            let try_lhs = self.current_token() == &Token::PipeTry;
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
                    | Token::PipeTry
            ) {
                return Err(self.pipe_error(pipe_pos, &format!("expected a stage after `{op}`")));
            }
            if try_lhs {
                left = self.alloc_expr(Expression::TryOp {
                    expr: left,
                    location: self.token_location(pipe_pos),
                });
            }
            left = self.parse_pipe_stage(left, op)?;
        }
        Ok(left)
    }

    fn pipe_op_text(&self) -> Option<&'static str> {
        match self.current_token() {
            Token::PipeOp => Some("|>"),
            Token::PipeTry => Some("|>?"),
            _ => None,
        }
    }

    fn parse_pipe_stage(
        &mut self,
        piped: &'static Expression<'static>,
        op: &str,
    ) -> Result<&'static Expression<'static>, String> {
        let stage_pos = self.position;
        let saved_flag = self.in_pipe_stage;
        let saved_holes = std::mem::take(&mut self.pipe_placeholders);
        self.in_pipe_stage = true;

        let result = if self.current_token() == &Token::Dot {
            // Method form: `x |> .method(a)` is `x.method(a)` (and may continue
            // with further postfix operators such as `?` or `.other()`).
            self.parse_postfix_expression(piped).map(|e| (e, Vec::new()))
        } else {
            self.parse_primary_expression()
                .and_then(|stage| self.insert_piped(piped, stage, stage_pos, op))
        };

        let holes = std::mem::replace(&mut self.pipe_placeholders, saved_holes);
        self.in_pipe_stage = saved_flag;

        let (expr, direct) = result?;
        // A placeholder token that is not one of the direct arguments is nested.
        if let Some(&stray) = holes.iter().find(|&&h| !direct.contains(&self.token_location(h))) {
            return Err(self.pipe_error(
                stray,
                "placeholder `_` must be a direct argument of the stage call (e.g. `f(a, _)`)",
            ));
        }
        if self.get_binary_op().is_some() && self.on_same_line_as_previous() {
            return Err(self.pipe_error(
                self.position,
                &format!("a `{op}` stage cannot be followed by an operator; parenthesise the stage or the whole pipe, e.g. `(x {op} f) + 1`"),
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
        op: &str,
    ) -> Result<(&'static Expression<'static>, Holes), String> {
        match stage {
            Expression::TryOp { expr, location } => {
                let (inner, n) = self.insert_piped(piped, expr, stage_pos, op)?;
                Ok((
                    self.alloc_expr(Expression::TryOp {
                        expr: inner,
                        location: location.clone(),
                    }),
                    n,
                ))
            }
            Expression::Await { expr, location } => {
                let (inner, n) = self.insert_piped(piped, expr, stage_pos, op)?;
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
                Vec::new(),
            )),
            _ => Err(self.pipe_error(
                stage_pos,
                &format!("right-hand side of `{op}` must be a function, call, method (`.name(..)`) or closure"),
            )),
        }
    }

    /// Put `piped` at the single `_` placeholder, or first when there is none.
    fn place_piped(
        &self,
        piped: &'static Expression<'static>,
        arguments: &Args,
        stage_pos: usize,
    ) -> Result<(Args, Holes), String> {
        let is_hole =
            |e: &Expression| matches!(e, Expression::Identifier { name, .. } if name == "_");
        let holes: Holes = arguments.iter().filter(|(_, e)| is_hole(e)).map(|(_, e)| e.location()).collect();
        match holes.len() {
            0 => {
                let mut args = Vec::with_capacity(arguments.len() + 1);
                args.push((None, piped));
                args.extend(arguments.iter().cloned());
                Ok((args, holes))
            }
            1 => Ok((
                arguments
                    .iter()
                    .map(|(l, e)| if is_hole(e) { (l.clone(), piped) } else { (l.clone(), *e) })
                    .collect(),
                holes,
            )),
            _ => {
                // Point at the second `_` (the one that makes the stage ambiguous).
                let second = self.pipe_placeholders.get(1).copied().unwrap_or(stage_pos);
                Err(self.pipe_error(
                    second,
                    "a pipe stage may contain at most one `_` placeholder (bind the value with `let` to use it twice)",
                ))
            }
        }
    }

    pub(in crate::parser) fn token_location(&self, token_index: usize) -> SourceLocation {
        self.tokens.get(token_index).map(|t| crate::source_map::Location {
            file: std::path::PathBuf::from(&self.filename),
            line: t.line,
            column: t.column,
        })
    }

    fn on_same_line_as_previous(&self) -> bool {
        match (self.tokens.get(self.position.wrapping_sub(1)), self.tokens.get(self.position)) {
            (Some(a), Some(b)) => a.line == b.line,
            _ => false,
        }
    }

    pub(in crate::parser) fn pipe_error(&self, token_index: usize, msg: &str) -> String {
        match self.tokens.get(token_index) {
            Some(t) => format!("{}:{}:{}: error: {}", self.filename, t.line, t.column, msg),
            None => format!("{}: error: {}", self.filename, msg),
        }
    }
}
