//! The template language of the std lowering table (docs/design/table-driven-std-surface.md, 2.3): the text of an
//! `emit = "..."` rule, parsed once and rendered with the operands of one call.
//!
//! ```text
//! {self}       the receiver's value          {&self}     the receiver as a place (assignable / addressable)
//! {name}       an argument, by the parameter name the declaration gives it
//! {name:f64}   the same, with a modifier (`Modifier`)
//! {T.max}      a fact (`Fact`) about the receiver's type `T`; `{R.go}` about the result `R`
//! {{  }}       a literal brace
//! ```
//!
//! The language is closed: no conditionals, no loops, no calls. `${` is refused, so a template can never be mistaken
//! for a string interpolation. Anything a template cannot say is a runtime helper the template calls by name.

use super::ir::PResult;
use crate::stdlib_surface::lowering::{Fact, Modifier};

/// Which type a fact is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRef {
    /// The receiver's type (`T` of the declaration).
    Receiver,
    /// The call's result type.
    Result,
}

/// One `{...}` of a template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hole {
    SelfValue(Option<Modifier>),
    SelfPlace,
    Arg(String, Option<Modifier>),
    Fact(TypeRef, Fact),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Seg {
    Text(String),
    Hole(Hole),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    segs: Vec<Seg>,
}

/// The values a template is rendered with. Each emitter implements it over its own operands.
pub trait Operands {
    fn receiver(&mut self, place: bool, modifier: Option<Modifier>) -> PResult<String>;
    /// The `index`-th declared parameter.
    fn arg(&mut self, index: usize, modifier: Option<Modifier>) -> PResult<String>;
    fn fact(&mut self, of: TypeRef, fact: Fact) -> PResult<String>;
}

impl Template {
    pub fn parse(text: &str) -> Result<Template, String> {
        if text.contains("${") {
            return Err(format!("`${{` in template `{text}`: templates use `{{..}}` only"));
        }
        let mut segs = Vec::new();
        let mut lit = String::new();
        let mut it = text.chars().peekable();
        while let Some(c) = it.next() {
            match c {
                '{' if it.peek() == Some(&'{') => {
                    it.next();
                    lit.push('{');
                }
                '}' if it.peek() == Some(&'}') => {
                    it.next();
                    lit.push('}');
                }
                '{' => {
                    let mut body = String::new();
                    loop {
                        match it.next() {
                            Some('}') => break,
                            Some(c) => body.push(c),
                            None => return Err(format!("unclosed `{{` in template `{text}`")),
                        }
                    }
                    if !lit.is_empty() {
                        segs.push(Seg::Text(std::mem::take(&mut lit)));
                    }
                    segs.push(Seg::Hole(parse_hole(&body).map_err(|e| format!("{e} in template `{text}`"))?));
                }
                '}' => return Err(format!("stray `}}` in template `{text}`")),
                c => lit.push(c),
            }
        }
        if !lit.is_empty() {
            segs.push(Seg::Text(lit));
        }
        Ok(Template { segs })
    }

    /// The holes, in order.
    pub fn holes(&self) -> impl Iterator<Item = &Hole> {
        self.segs.iter().filter_map(|s| match s {
            Seg::Hole(h) => Some(h),
            Seg::Text(_) => None,
        })
    }

    /// Render with the operands of one call. `params` are the declared parameter names (`{name}` is their index).
    pub fn render(&self, params: &[String], ops: &mut dyn Operands) -> PResult<String> {
        let mut out = String::new();
        for s in &self.segs {
            match s {
                Seg::Text(t) => out.push_str(t),
                Seg::Hole(Hole::SelfValue(m)) => out.push_str(&ops.receiver(false, *m)?),
                Seg::Hole(Hole::SelfPlace) => out.push_str(&ops.receiver(true, None)?),
                Seg::Hole(Hole::Arg(name, m)) => match params.iter().position(|p| p == name) {
                    Some(i) => out.push_str(&ops.arg(i, *m)?),
                    None => return super::ir::unsupported(format!("template names the parameter `{name}`, which the declaration lacks")),
                },
                Seg::Hole(Hole::Fact(of, f)) => out.push_str(&ops.fact(*of, *f)?),
            }
        }
        Ok(out)
    }
}

fn parse_hole(body: &str) -> Result<Hole, String> {
    let body = body.trim();
    if body == "&self" {
        return Ok(Hole::SelfPlace);
    }
    let (name, modifier) = match body.split_once(':') {
        Some((n, m)) => {
            let modifier = Modifier::ALL.into_iter().find(|k| k.name() == m.trim()).ok_or_else(|| format!("unknown modifier `{m}` in `{{{body}}}`"))?;
            (n.trim(), Some(modifier))
        }
        None => (body, None),
    };
    if name == "self" {
        return Ok(Hole::SelfValue(modifier));
    }
    if let Some((ty, fact)) = name.split_once('.') {
        if modifier.is_some() {
            return Err(format!("a fact takes no modifier: `{{{body}}}`"));
        }
        let of = match ty.chars().collect::<Vec<char>>().as_slice() {
            ['T'] => TypeRef::Receiver,
            ['R'] => TypeRef::Result,
            _ => return Err(format!("unknown type `{ty}` in `{{{body}}}` (T or R)")),
        };
        let fact = Fact::ALL.into_iter().find(|f| f.name() == fact).ok_or_else(|| format!("unknown fact `{fact}` in `{{{body}}}`"))?;
        return Ok(Hole::Fact(of, fact));
    }
    let ident = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.starts_with(|c: char| c.is_ascii_digit());
    if !ident {
        return Err(format!("`{{{body}}}` is not a placeholder"));
    }
    Ok(Hole::Arg(name.to_string(), modifier))
}

/// Parse and render the template of an `emit` rule. A template that does not parse is a defect of the declaration
/// (gate G1 refuses it before it ships), reported as an unsupported construct rather than a panic.
pub fn render_emit(template: &str, params: &[String], ops: &mut dyn Operands) -> PResult<String> {
    match Template::parse(template) {
        Ok(t) => t.render(params, ops),
        Err(e) => super::ir::unsupported(format!("lowering template: {e}")),
    }
}

/// Operands that are plain text: the receiver and the arguments as the emitter already printed them.
pub struct TextOperands<'a> {
    pub receiver: &'a str,
    pub args: &'a [String],
}

impl Operands for TextOperands<'_> {
    fn receiver(&mut self, place: bool, modifier: Option<Modifier>) -> PResult<String> {
        if place || modifier.is_some() {
            return super::ir::unsupported("this target has no receiver place or modifier for the template");
        }
        Ok(self.receiver.to_string())
    }

    fn arg(&mut self, index: usize, modifier: Option<Modifier>) -> PResult<String> {
        if modifier.is_some() {
            return super::ir::unsupported("this target has no operand modifier for the template");
        }
        match self.args.get(index) {
            Some(a) => Ok(a.clone()),
            None => super::ir::unsupported(format!("the call has no argument {index}")),
        }
    }

    fn fact(&mut self, _of: TypeRef, fact: Fact) -> PResult<String> {
        super::ir::unsupported(format!("the type fact `{}` is not available for this receiver", fact.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(t: &str, params: &[&str], recv: &str, args: &[&str]) -> PResult<String> {
        let params: Vec<String> = params.iter().map(|s| s.to_string()).collect();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        Template::parse(t).unwrap().render(&params, &mut TextOperands { receiver: recv, args: &args })
    }

    #[test]
    fn holes_are_replaced_and_text_is_kept() {
        assert_eq!(render("$fmin($fmax({self}, {lo}), {hi})", &["lo", "hi"], "x", &["1", "2"]).unwrap(), "$fmin($fmax(x, 1), 2)");
        assert_eq!(render("({{ {self} }})", &[], "x", &[]).unwrap(), "({ x })");
    }

    #[test]
    fn holes_parse_to_their_kinds() {
        let t = Template::parse("{self}{&self}{a:f64}{T.max}{R.go}{self:f64}").unwrap();
        let holes: Vec<&Hole> = t.holes().collect();
        assert_eq!(
            holes,
            vec![
                &Hole::SelfValue(None),
                &Hole::SelfPlace,
                &Hole::Arg("a".into(), Some(Modifier::F64)),
                &Hole::Fact(TypeRef::Receiver, Fact::Max),
                &Hole::Fact(TypeRef::Result, Fact::Go),
                &Hole::SelfValue(Some(Modifier::F64)),
            ]
        );
    }

    #[test]
    fn malformed_templates_are_refused() {
        for bad in ["${x}", "{self", "x}", "{}", "{1a}", "{a:nope}", "{X.max}", "{T.nope}", "{T.max:f64}", "{a b}"] {
            assert!(Template::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn an_undeclared_parameter_or_missing_argument_is_unsupported_not_a_panic() {
        assert!(render("{nope}", &["a"], "x", &["1"]).is_err());
        assert!(render("{a}", &["a"], "x", &[]).is_err());
    }
}
