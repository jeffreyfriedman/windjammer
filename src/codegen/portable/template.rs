//! The template language of the std lowering table (docs/design/table-driven-std-surface.md, 2.3): the text of an
//! `emit = "..."` rule, parsed once and rendered with the operands of one call.
//!
//! ```text
//! {self}       the receiver's value          {&self}     the receiver as a place (assignable / addressable)
//! {name}       an argument, by the parameter name the declaration gives it
//! {name:f64}   the same, with a modifier (`Modifier`)
//! {T.max}      a fact (`Fact`) about the receiver's type `T`; `{R.go}` about the result `R`, `{O.go}` about the `Ok`
//!              type of a `Result` result (`parse::<i32>()`)
//! {{  }}       a literal brace
//! ```
//!
//! The language is closed: no conditionals, no loops, no calls. `${` is refused, so a template can never be mistaken
//! for a string interpolation. Anything a template cannot say is a runtime helper the template calls by name.

use super::ir::PResult;
use super::ty::{IntKind, Ty};
use crate::stdlib_surface::lowering::{Fact, Guard, Modifier};

/// The guards that are about the call's own types, answered from them: the receiver's integer type (`when = "t_wide"`) and
/// the `Ok` type of a `Result` result (`when = "ok_int"`); the guards about the receiver's element are the emitter's
/// (they need its notion of a primitive).
pub fn type_guard(guard: Guard, recv: &Ty, result: &Ty) -> bool {
    let ok = || match result {
        Ty::Result(ok, _) => Some(&**ok),
        _ => None,
    };
    match (guard, recv) {
        (Guard::RecvWide, Ty::Int(k)) => k.is_wide(),
        (Guard::RecvI32, Ty::Int(k)) => *k == IntKind::I32,
        (Guard::RecvU32, Ty::Int(k)) => *k == IntKind::U32,
        (Guard::OkInt, _) => matches!(ok(), Some(Ty::Int(_))),
        (Guard::OkF32, _) => matches!(ok(), Some(Ty::F32)),
        (Guard::OkF64, _) => matches!(ok(), Some(Ty::F64)),
        (Guard::OkBool, _) => matches!(ok(), Some(Ty::Bool)),
        _ => false,
    }
}

/// The integer type a fact is asked about: the receiver's or the result's, when it is an integer.
pub fn int_kind_of<'a>(of: TypeRef, receiver: &'a Ty, result: &'a Ty) -> Option<IntKind> {
    let ty = match (of, result) {
        (TypeRef::Receiver, _) => receiver,
        (TypeRef::Result, _) => result,
        (TypeRef::Ok, Ty::Result(ok, _)) => &**ok,
        (TypeRef::Ok, _) => return None,
    };
    match ty {
        Ty::Int(k) => Some(*k),
        _ => None,
    }
}

/// The facts every target answers the same way.
pub fn common_int_fact(k: IntKind, fact: Fact) -> Option<String> {
    match fact {
        Fact::Bits => Some(k.bits().to_string()),
        Fact::Signed => Some(k.signed().to_string()),
        _ => None,
    }
}

/// Which type a fact is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRef {
    /// The receiver's type (`T` of the declaration).
    Receiver,
    /// The call's result type.
    Result,
    /// The `Ok` type of a `Result` result.
    Ok,
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
            ['O'] => TypeRef::Ok,
            _ => return Err(format!("unknown type `{ty}` in `{{{body}}}` (T, R or O)")),
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
    fn type_guards_read_the_receivers_and_the_results_types() {
        let none = Ty::Unit;
        assert!(type_guard(Guard::RecvWide, &Ty::Int(IntKind::Usize), &none));
        assert!(!type_guard(Guard::RecvWide, &Ty::Int(IntKind::U32), &none));
        assert!(type_guard(Guard::RecvI32, &Ty::Int(IntKind::I32), &none));
        assert!(!type_guard(Guard::RecvI32, &Ty::Int(IntKind::U32), &none));
        assert!(type_guard(Guard::RecvU32, &Ty::Int(IntKind::U32), &none));
        assert!(!type_guard(Guard::RecvU32, &Ty::F32, &none));
        assert!(!type_guard(Guard::ElemPrim, &Ty::Int(IntKind::I32), &none));
        let parsed = |ok: Ty| Ty::Result(Box::new(ok), Box::new(Ty::Str));
        assert!(type_guard(Guard::OkInt, &Ty::Str, &parsed(Ty::Int(IntKind::U8))));
        assert!(!type_guard(Guard::OkInt, &Ty::Str, &parsed(Ty::F64)));
        assert!(type_guard(Guard::OkF32, &Ty::Str, &parsed(Ty::F32)));
        assert!(type_guard(Guard::OkF64, &Ty::Str, &parsed(Ty::F64)));
        assert!(type_guard(Guard::OkBool, &Ty::Str, &parsed(Ty::Bool)));
        assert!(!type_guard(Guard::OkBool, &Ty::Str, &Ty::Bool));
    }

    #[test]
    fn the_ok_type_of_a_result_is_asked_by_o() {
        let parsed = Ty::Result(Box::new(Ty::Int(IntKind::U16)), Box::new(Ty::Str));
        assert_eq!(int_kind_of(TypeRef::Ok, &Ty::Str, &parsed), Some(IntKind::U16));
        assert_eq!(int_kind_of(TypeRef::Ok, &Ty::Str, &Ty::Int(IntKind::U16)), None);
        assert_eq!(Template::parse("{O.go}").unwrap().holes().next(), Some(&Hole::Fact(TypeRef::Ok, Fact::Go)));
    }

    #[test]
    fn common_facts_are_the_kinds_bits_and_signedness() {
        assert_eq!(common_int_fact(IntKind::U16, Fact::Bits).as_deref(), Some("16"));
        assert_eq!(common_int_fact(IntKind::I8, Fact::Signed).as_deref(), Some("true"));
        assert_eq!(common_int_fact(IntKind::U8, Fact::Signed).as_deref(), Some("false"));
        assert_eq!(common_int_fact(IntKind::U8, Fact::Max), None);
    }

    #[test]
    fn an_undeclared_parameter_or_missing_argument_is_unsupported_not_a_panic() {
        assert!(render("{nope}", &["a"], "x", &["1"]).is_err());
        assert!(render("{a}", &["a"], "x", &[]).is_err());
    }
}
