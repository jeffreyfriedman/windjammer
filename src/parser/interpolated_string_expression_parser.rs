// Interpolated string → `format!(...)` expression lowering.

use crate::parser::ast::*;
use crate::parser_impl::Parser;

impl Parser {
    pub(in crate::parser) fn finish_interpolated_string(
        &mut self,
        parts: Vec<crate::lexer::StringPart>,
    ) -> Result<&'static Expression<'static>, String> {
        let mut format_string = String::new();
        let mut args = Vec::new();

        for part in parts {
            match part {
                crate::lexer::StringPart::Literal(lit) => {
                    let escaped = lit.replace('{', "{{").replace('}', "}}");
                    format_string.push_str(&escaped);
                }
                crate::lexer::StringPart::Expression(expr_str) => {
                    format_string.push_str("{}");
                    let trimmed = expr_str.trim();
                    let mut expr_lexer = crate::lexer::Lexer::new(trimmed);
                    let mut expr_tokens = Vec::new();
                    loop {
                        let tok_with_loc = expr_lexer.next_token_with_location();
                        if tok_with_loc.token == crate::lexer::Token::Eof {
                            break;
                        }
                        expr_tokens.push(tok_with_loc);
                    }

                    let expr_parser = Box::leak(Box::new(Parser::new(expr_tokens)));
                    let expr = expr_parser.parse_expression().map_err(|e| {
                        format!("invalid expression in string interpolation `{trimmed}`: {e}")
                    })?;
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
