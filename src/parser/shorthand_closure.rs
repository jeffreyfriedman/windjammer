// MODULE: `_` shorthand closures (ADR-0018 feature 2, spec area CLOS).
//
// `users.filter(_.active)`, `xs.map(_ * 2)`, `xs.fold(0, _ + _)`: inside a call argument, an expression that
// contains `_` (and is not exactly `_`) is a closure with one parameter per `_`, left to right. The closure is
// the SMALLEST enclosing call argument: `f(g(_ + 1))` is `f(g(|x| x + 1))`.
//
// A bare `_` argument is never a closure: in a `|>` stage it is the pipe placeholder (pipe_parser.rs), anywhere
// else an error. The parser emits a plain `Expression::Closure` with parameters `__it0`, `__it1`, ..., so the
// analyzer, the IR solver and every backend only ever see ordinary closures.
//
// State: one `ShorthandFrame` per call argument being parsed (a stack, innermost last). `parse_primary_expression`
// calls `shorthand_hole` for a `_` token; `parse_arguments` calls `parse_argument_with_shorthand`.

use crate::parser::ast::*;
use crate::parser_impl::Parser;

/// Parameter name of the `i`-th `_` of a shorthand closure.
pub(crate) fn hole_name(i: usize) -> String {
    format!("__it{i}")
}

#[derive(Debug, Default)]
pub(crate) struct ShorthandFrame {
    /// Token index of each `_` of this argument, in order.
    holes: Vec<usize>,
    /// Explicit-closure nesting depth when the frame was opened.
    closure_depth: usize,
}

impl Parser {
    /// A `_` expression token at `self.position` (not yet consumed). Returns the expression it stands for.
    pub(in crate::parser) fn shorthand_hole(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        let pos = self.position;
        let location = self.current_location();
        let depth = self.closure_depth;
        let Some(frame) = self.shorthand_frames.last_mut() else {
            if self.in_pipe_stage {
                // `x |> _`: reported by the pipe parser with its own message.
                self.pipe_placeholders.push(pos);
                self.advance();
                return Ok(self.alloc_expr(Expression::Identifier {
                    name: "_".to_string(),
                    location,
                }));
            }
            return Err(self.pipe_error(
                pos,
                "`_` is only valid inside a call argument (shorthand closure such as `xs.map(_ * 2)`) or as a `|>` placeholder; write a closure such as `|x| x + 1`",
            ));
        };
        if depth > frame.closure_depth {
            return Err(self.pipe_error(
                pos,
                "`_` shorthand cannot be used inside an explicit closure; name the parameter (`|x| ..`) instead",
            ));
        }
        frame.holes.push(pos);
        let name = hole_name(frame.holes.len() - 1);
        self.advance();
        Ok(self.alloc_expr(Expression::Identifier { name, location }))
    }

    /// Parse one call argument, turning it into a closure when it contains `_` holes.
    pub(in crate::parser) fn parse_argument_with_shorthand(
        &mut self,
    ) -> Result<&'static Expression<'static>, String> {
        self.shorthand_frames.push(ShorthandFrame {
            holes: Vec::new(),
            closure_depth: self.closure_depth,
        });
        let parsed = self.parse_expression();
        let frame = self.shorthand_frames.pop().unwrap_or_default();
        let expr = parsed?;
        if frame.holes.is_empty() {
            return Ok(expr);
        }
        let bare = matches!(expr, Expression::Identifier { name, .. } if *name == hole_name(0));
        if bare {
            let pos = frame.holes[0];
            if self.in_pipe_stage {
                self.pipe_placeholders.push(pos);
                return Ok(self.alloc_expr(Expression::Identifier {
                    name: "_".to_string(),
                    location: expr.location(),
                }));
            }
            return Err(self.pipe_error(
                pos,
                "`_` alone is not a function: write the function name or a closure such as `|x| x` (shorthand needs an expression around `_`, e.g. `_ * 2`)",
            ));
        }
        let parameters = (0..frame.holes.len()).map(hole_name).collect();
        Ok(self.alloc_expr(Expression::Closure {
            parameters,
            body: expr,
            location: expr.location(),
        }))
    }

    /// Run an explicit-closure parser with the nesting depth raised (so `_` inside it is rejected).
    pub(in crate::parser) fn with_explicit_closure<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        self.closure_depth += 1;
        let r = f(self);
        self.closure_depth -= 1;
        r
    }
}
