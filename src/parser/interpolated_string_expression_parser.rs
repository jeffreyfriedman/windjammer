// Interpolated string → `format!(...)` expression lowering.

use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn finish_interpolated_string(
        &mut self,
        parts: Vec<crate::lexer::StringPart>,
        location: Option<crate::source_map::Location>,
        expr_starts: Vec<(usize, usize, usize)>,
    ) -> Result<&'static Expression<'static>, String> {
        let mut format_string = String::new();
        let mut args = Vec::new();
        let mut expr_index = 0usize;

        for part in parts {
            match part {
                crate::lexer::StringPart::Literal(lit) => {
                    let escaped = lit.replace('{', "{{").replace('}', "}}");
                    format_string.push_str(&escaped);
                }
                crate::lexer::StringPart::Expression(expr_str) => {
                    // `${x:.2}` / `${x:?}`: split a trailing format specifier (ADR-0014).
                    let (body, spec) =
                        crate::intrinsics::split_interp_spec(expr_str.trim());
                    match spec {
                        Some(spec) => format_string.push_str(&format!("{{:{spec}}}")),
                        None => format_string.push_str("{}"),
                    }
                    let trimmed = body.trim();
                    // The body is verbatim source text: lex it at its real position so every
                    // sub-expression location is a location in the file, not in the substring.
                    let origin = expr_starts.get(expr_index).copied();
                    expr_index += 1;
                    let mut expr_lexer = match origin {
                        Some((line, col, byte)) => {
                            let lead = &expr_str[..expr_str.len() - expr_str.trim_start().len()];
                            let (mut l, mut c) = (line, col);
                            for ch in lead.chars() {
                                if ch == '\n' {
                                    l += 1;
                                    c = 1;
                                } else {
                                    c += 1;
                                }
                            }
                            crate::lexer::Lexer::with_origin(trimmed, l, c, byte + lead.len())
                        }
                        None => crate::lexer::Lexer::new(trimmed),
                    };
                    let mut expr_tokens = Vec::new();
                    loop {
                        let tok_with_loc = expr_lexer.next_token_with_location();
                        let is_eof = tok_with_loc.token == crate::lexer::Token::Eof;
                        expr_tokens.push(tok_with_loc);
                        if is_eof {
                            break;
                        }
                    }

                    let expr_parser = Box::leak(Box::new(Parser::new_with_source(
                        expr_tokens,
                        self.filename.clone(),
                        String::new(),
                    )));
                    if self.span_log.borrow().is_some() {
                        expr_parser.enable_span_log();
                    }
                    let expr = expr_parser.parse_expression().map_err(|e| {
                        // Locate at the failing token inside the interpolation (real file coordinates).
                        expr_parser.locate_error(format!(
                            "invalid expression in string interpolation `{trimmed}`: {e}"
                        ))
                    })?;
                    self.adopt_spans(expr_parser);
                    args.push(expr);
                }
            }
        }

        let format_lit = self.alloc_expr(Expression::Literal {
            value: Literal::String(format_string),
            location: location.clone(),
        });
        let mut macro_args = vec![format_lit];
        macro_args.extend(args);

        Ok(self.alloc_expr(Expression::MacroInvocation {
            name: "format".to_string(),
            args: macro_args,
            delimiter: MacroDelimiter::Parens,
            is_repeat: false,
            location,
        }))
    }
}
