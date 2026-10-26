// Interpolated string → `format!(...)` expression lowering.

use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn finish_interpolated_string(
        &mut self,
        parts: Vec<crate::lexer::StringPart>,
        expr_starts: Vec<(usize, usize, usize)>,
    ) -> Result<&'static Expression<'static>, String> {
        let mut format_string = String::new();
        let mut args = Vec::new();

        // Sub-expressions are lexed on their own, so their (line, column) restart at (1, 1) and would
        // collide with each other and with real code in every position-keyed table (numeric
        // inference keys expressions by location). Rebase them onto the string's own line at columns
        // no source line reaches, unique per string and per `${..}` part.
        let outer_line = self.current_location().map(|l| l.line).unwrap_or(0);
        let outer_key = self.position;
        let mut expr_index = 0usize;
        for (part_index, part) in parts.into_iter().enumerate() {
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
                    // The body is verbatim source text: lex it at its real position so every token
                    // carries true byte offsets and `src_line`/`src_column` (spans, diagnostics).
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
                        if tok_with_loc.token == crate::lexer::Token::Eof {
                            break;
                        }
                        let mut tok_with_loc = tok_with_loc;
                        tok_with_loc.line = outer_line;
                        tok_with_loc.column +=
                            1_000_000 + (outer_key * 64 + part_index.min(63)) * 10_000;
                        expr_tokens.push(tok_with_loc);
                    }

                    // Nodes are located at the token that follows them; without a trailing token the
                    // last expression of the part would have no location (and no numeric identity).
                    let end_column = expr_tokens.last().map_or(1_000_000, |t| t.column + 1);
                    let end_pos = expr_lexer.position();
                    expr_tokens.push(crate::lexer::TokenWithLocation {
                        token: crate::lexer::Token::Eof,
                        line: outer_line,
                        column: end_column,
                        src_line: end_pos.0,
                        src_column: end_pos.1,
                        byte_start: expr_lexer.byte_offset(),
                        byte_end: expr_lexer.byte_offset(),
                        end_line: end_pos.0,
                        end_column: end_pos.1,
                        interpolation_starts: Vec::new(),
                    });
                    // Allocate into this parser's arena bundle so the nodes are sealed into the same Program.
                    let mut expr_parser = Parser::new_sharing_arenas(expr_tokens, self);
                    // ADR-0018: named arguments inside `${..}` resolve like anywhere else.
                    expr_parser.call_table = self.call_table.clone();
                    if self.span_log.borrow().is_some() {
                        expr_parser.enable_span_log();
                    }
                    let expr = expr_parser.parse_expression().map_err(|e| {
                        format!("invalid expression in string interpolation `{}`: {e}", trimmed.escape_debug())
                    })?;
                    self.saw_label |= expr_parser.saw_label;
                    self.adopt_spans(&expr_parser);
                    args.push(expr);
                }
            }
        }

        let format_lit = self.alloc_expr(Expression::Literal {
            value: Literal::String(format_string),
            location: self.current_location(),
        });
        let mut macro_args = vec![format_lit];
        macro_args.extend(args);

        Ok(self.alloc_expr(Expression::MacroInvocation {
            name: "format".to_string(),
            args: macro_args,
            delimiter: MacroDelimiter::Parens,
            is_repeat: false,
            location: self.current_location(),
        }))
    }
}
