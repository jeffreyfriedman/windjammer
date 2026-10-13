// MODULE: named arguments and parameter defaults (ADR-0018 feature 3, spec area ARGS).
//
//     fn connect(host: string, timeout: int = 30, retries: int = 3) -> string { .. }
//     connect("db", retries: 5)          // == connect("db", 30, 5)
//
// Lowering happens in the parser, so every backend sees positional calls:
// 1. `parse_parameters` accepts `name: Type = expr` and stores the default as a parameter decorator
//    `__default` (no AST shape change; backends ignore unknown parameter decorators).
// 2. `Parser::parse` builds a table of callees declared in this file (free functions, `Type::assoc`, and methods
//    whose name is unique among this file's impls) once the first pass succeeds; if any callee has a default or a
//    call used a label, the file is parsed again with the table installed.
// 3. At each call to a known callee, `resolve_call_arguments` binds positional then named arguments, fills the
//    defaults (the default expression is placed at the call site, so it is evaluated on every call), and reports
//    unknown / duplicate / missing / positional-after-named arguments.
// Defaults may not refer to other parameters (checked when the default is parsed).

use crate::lexer::Token;
use crate::parser::ast::*;
use crate::parser_impl::Parser;
use std::collections::HashMap;
use std::rc::Rc;

/// Decorator name that carries a parameter's default value.
pub(crate) const DEFAULT_DECORATOR: &str = "__default";

#[derive(Debug, Clone)]
pub(crate) struct ParamSig {
    name: String,
    default: Option<&'static Expression<'static>>,
}

/// Callee key -> its parameters (receiver excluded). Keys: `f`, `Type::f`, `.method`.
pub(crate) type CallTable = Rc<HashMap<String, Vec<ParamSig>>>;

type Args = Vec<(Option<String>, &'static Expression<'static>)>;

fn sig_of(f: &FunctionDecl<'static>) -> Vec<ParamSig> {
    f.parameters
        .iter()
        .filter(|p| p.name != "self")
        .map(|p| ParamSig {
            name: p.name.clone(),
            default: p
                .decorators
                .iter()
                .find(|d| d.name == DEFAULT_DECORATOR)
                .and_then(|d| d.arguments.first())
                .map(|(_, e)| *e),
        })
        .collect()
}

/// Build the callee table of a parsed program. `None` when nothing in the file needs named-argument lowering.
pub(crate) fn call_table(program: &Program<'static>, saw_label: bool) -> Option<CallTable> {
    let mut table: HashMap<String, Vec<ParamSig>> = HashMap::new();
    let mut method_counts: HashMap<String, usize> = HashMap::new();
    let mut any_default = false;
    let mut add =
        |key: String, f: &FunctionDecl<'static>, table: &mut HashMap<String, Vec<ParamSig>>| {
            let sig = sig_of(f);
            any_default |= sig.iter().any(|p| p.default.is_some());
            table.insert(key, sig);
        };
    for item in &program.items {
        match item {
            Item::Function { decl, .. } => add(decl.name.clone(), decl, &mut table),
            Item::Impl { block, .. } => {
                for f in &block.functions {
                    add(format!("{}::{}", block.type_name, f.name), f, &mut table);
                    if f.parameters.first().is_some_and(|p| p.name == "self") {
                        *method_counts.entry(f.name.clone()).or_insert(0) += 1;
                        add(format!(".{}", f.name), f, &mut table);
                    }
                }
            }
            _ => {}
        }
    }
    // A method name defined by several impls is ambiguous without types: positional calls only.
    for (name, n) in method_counts {
        if n > 1 {
            table.remove(&format!(".{name}"));
        }
    }
    (any_default || saw_label).then(|| Rc::new(table))
}

impl Parser {
    /// After a parameter's type: parse `= default` if present and attach it as a `__default` decorator.
    pub(in crate::parser) fn parse_parameter_default(
        &mut self,
        earlier: &[Parameter<'static>],
        decorators: &mut Vec<Decorator<'static>>,
    ) -> Result<(), String> {
        if self.current_token() != &Token::Assign {
            return Ok(());
        }
        self.advance();
        let start = self.position;
        let expr = self.parse_expression()?;
        for i in start..self.position {
            if let Some(Token::Ident(n)) = self.tokens.get(i).map(|t| &t.token) {
                if earlier.iter().any(|p| &p.name == n) {
                    return Err(self.pipe_error(
                        i,
                        &format!("a parameter default cannot refer to parameter `{n}` (defaults are evaluated at the call site, before the call)"),
                    ));
                }
            }
        }
        decorators.push(Decorator {
            name: DEFAULT_DECORATOR.to_string(),
            arguments: vec![(String::new(), expr)],
        });
        Ok(())
    }

    /// Check that parameters with defaults come after those without.
    pub(in crate::parser) fn check_default_order(
        &self,
        params: &[Parameter<'static>],
    ) -> Result<(), String> {
        let mut seen_default = false;
        for p in params.iter().filter(|p| p.name != "self") {
            let has = p.decorators.iter().any(|d| d.name == DEFAULT_DECORATOR);
            if seen_default && !has {
                return Err(format!(
                    "parameter `{}` without a default follows a parameter with a default (at {}); put parameters with defaults last",
                    p.name,
                    self.pos_desc()
                ));
            }
            seen_default |= has;
        }
        Ok(())
    }

    /// Bind `args` (as written) to the parameters of `key`'s callee, in parameter order. Unknown callee or no
    /// table: the arguments are returned unchanged.
    pub(in crate::parser) fn resolve_call_arguments(
        &mut self,
        key: &str,
        args: Args,
    ) -> Result<Args, String> {
        let Some(table) = self.call_table.clone() else {
            return Ok(args);
        };
        let Some(params) = table.get(key) else {
            return Ok(args);
        };
        let needs = args.iter().any(|(l, _)| l.is_some()) || args.len() < params.len();
        if !needs {
            return Ok(args);
        }
        let callee = key.trim_start_matches('.');
        let list = params
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let mut slots: Vec<Option<&'static Expression<'static>>> = vec![None; params.len()];
        let mut named_seen = false;
        for (i, (label, e)) in args.iter().enumerate() {
            match label {
                None => {
                    if named_seen {
                        return Err(format!("`{callee}`: positional argument after a named one (at {}); write positional arguments first", self.pos_desc()));
                    }
                    if i >= params.len() {
                        return Err(format!(
                            "`{callee}` takes {} argument(s) ({list}), got {}",
                            params.len(),
                            args.len()
                        ));
                    }
                    slots[i] = Some(e);
                }
                Some(name) => {
                    named_seen = true;
                    let Some(j) = params.iter().position(|p| &p.name == name) else {
                        return Err(format!(
                            "`{callee}` has no parameter `{name}` (parameters: {list})"
                        ));
                    };
                    if slots[j].is_some() {
                        return Err(format!("`{callee}`: parameter `{name}` is given twice"));
                    }
                    slots[j] = Some(e);
                }
            }
        }
        let mut out = Vec::with_capacity(params.len());
        for (p, slot) in params.iter().zip(slots) {
            match slot.or(p.default) {
                Some(e) => out.push((None, e)),
                None => {
                    return Err(format!(
                        "`{callee}`: missing argument `{}` (parameters: {list})",
                        p.name
                    ))
                }
            }
        }
        Ok(out)
    }
}

impl Parser {
    /// After `callee(args)`: a trailing block (parser/trailing_block.rs), then named-argument lowering.
    pub(in crate::parser) fn finish_call_arguments(
        &mut self,
        key: Option<String>,
        args: Args,
    ) -> Result<Args, String> {
        let args = self.append_trailing_block(args)?;
        match key {
            Some(k) => self.resolve_call_arguments(&k, args),
            None => Ok(args),
        }
    }

    /// `x??` with nothing after it to default to: two try operators (the meaning before `??` existed).
    pub(in crate::parser) fn double_try_follows(&self) -> bool {
        matches!(
            self.peek(1),
            Some(
                Token::Dot
                    | Token::RParen
                    | Token::RBracket
                    | Token::RBrace
                    | Token::Comma
                    | Token::Semicolon
                    | Token::Eof
            )
        )
    }

    pub(in crate::parser) fn double_try(
        &mut self,
        expr: &'static Expression<'static>,
    ) -> &'static Expression<'static> {
        self.advance();
        let once = self.alloc_expr(Expression::TryOp {
            expr,
            location: self.current_location(),
        });
        self.alloc_expr(Expression::TryOp {
            expr: once,
            location: self.current_location(),
        })
    }
}

/// Callee key of a call's function expression: `f` or `Type::f`.
pub(crate) fn call_key(function: &Expression<'_>) -> Option<String> {
    match function {
        Expression::Identifier { name, .. } => Some(name.clone()),
        _ => None,
    }
}
