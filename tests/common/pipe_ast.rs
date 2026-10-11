//! Test helper: parse a source snippet and render the initializer of `let r = <expr>` as a
//! location-free s-expression. Independent of the compiler's own printers.

use windjammer::lexer::Lexer;
use windjammer::parser::ast::{Expression, Literal, Statement, Item};
use windjammer::parser::Parser;

pub fn parse_program_err(src: &str) -> Result<(), String> {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new_with_source(tokens, "t.wj".into(), src.into())));
    parser.parse().map(|_| ())
}

/// `sexp("x |> f")` renders `(call f x)`.
pub fn sexp(expr_src: &str) -> Result<String, String> {
    let src = format!("fn t() {{\n    let r = {expr_src}\n}}\n");
    let tokens = Lexer::new(&src).tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new_with_source(tokens, "t.wj".into(), src.clone())));
    let program = parser.parse()?;
    for item in &program.items {
        if let Item::Function { decl, .. } = item {
            if let Some(Statement::Let { value, .. }) = decl.body.first() {
                return Ok(show(value));
            }
        }
    }
    Err("no let found".into())
}

pub fn show(e: &Expression) -> String {
    match e {
        Expression::Identifier { name, .. } => name.clone(),
        Expression::Literal { value: Literal::Int(n), .. } => n.to_string(),
        Expression::Literal { value: Literal::String(s), .. } => format!("{s:?}"),
        Expression::Literal { value: Literal::Bool(b), .. } => b.to_string(),
        Expression::Literal { .. } => "lit".into(),
        Expression::Binary { left, op, right, .. } => format!("({op:?} {} {})", show(left), show(right)),
        Expression::Unary { op, operand, .. } => format!("({op:?} {})", show(operand)),
        Expression::Call { function, arguments, .. } => {
            let a: Vec<String> = arguments.iter().map(|(_, x)| show(x)).collect();
            format!("(call {} {})", show(function), a.join(" ")).replace(" )", ")")
        }
        Expression::MethodCall { object, method, arguments, .. } => {
            let a: Vec<String> = arguments.iter().map(|(_, x)| show(x)).collect();
            format!("(meth {} {} {})", show(object), method, a.join(" ")).replace(" )", ")")
        }
        Expression::FieldAccess { object, field, .. } => format!("(field {} {field})", show(object)),
        Expression::Closure { parameters, body, .. } => format!("(fn [{}] {})", parameters.join(" "), show(body)),
        Expression::TryOp { expr, .. } => format!("(try {})", show(expr)),
        Expression::Await { expr, .. } => format!("(await {})", show(expr)),
        Expression::Tuple { elements, .. } => {
            let a: Vec<String> = elements.iter().map(|x| show(x)).collect();
            format!("(tuple {})", a.join(" "))
        }
        _ => "<?>".into(),
    }
}
