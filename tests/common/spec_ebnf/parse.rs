use super::{Expr, Grammar, Production};

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Name(String),
    Str(String),
    Class(Vec<(char, char)>),
    Sym(char),
}

fn lex(src: &str, directives: &mut Vec<(usize, String)>) -> Result<Vec<(usize, Tok)>, String> {
    let cs: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1;
    let mut out = Vec::new();
    while i < cs.len() {
        let c = cs[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '(' && cs.get(i + 1) == Some(&'*') {
            let start = i + 2;
            let mut j = start;
            while j + 1 < cs.len() && !(cs[j] == '*' && cs[j + 1] == ')') {
                if cs[j] == '\n' {
                    line += 1;
                }
                j += 1;
            }
            if j + 1 >= cs.len() {
                return Err(format!("line {line}: unterminated comment"));
            }
            let body: String = cs[start..j].iter().collect();
            if let Some(d) = body.strip_prefix('@') {
                directives.push((out.len(), d.trim().to_string()));
            }
            i = j + 2;
        } else if c == '"' || c == '\'' {
            let q = c;
            let mut j = i + 1;
            let mut s = String::new();
            while j < cs.len() && cs[j] != q {
                if cs[j] == '\\' && j + 1 < cs.len() {
                    j += 1;
                    s.push(match cs[j] {
                        'n' => '\n',
                        't' => '\t',
                        o => o,
                    });
                } else {
                    s.push(cs[j]);
                }
                j += 1;
            }
            out.push((line, Tok::Str(s)));
            i = j + 1;
        } else if c == '/' {
            let mut j = i + 1;
            let mut body = Vec::new();
            while j < cs.len() && cs[j] != '/' {
                body.push(cs[j]);
                j += 1;
            }
            let mut ranges = Vec::new();
            let mut k = 0;
            while k < body.len() {
                if k + 2 < body.len() && body[k + 1] == '-' {
                    ranges.push((body[k], body[k + 2]));
                    k += 3;
                } else {
                    ranges.push((body[k], body[k]));
                    k += 1;
                }
            }
            out.push((line, Tok::Class(ranges)));
            i = j + 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let mut j = i;
            while j < cs.len() && (cs[j].is_ascii_alphanumeric() || cs[j] == '_') {
                j += 1;
            }
            out.push((line, Tok::Name(cs[i..j].iter().collect())));
            i = j;
        } else if "=|;()[]{}".contains(c) {
            out.push((line, Tok::Sym(c)));
            i += 1;
        } else {
            return Err(format!("line {line}: unexpected character {c:?}"));
        }
    }
    Ok(out)
}

struct P {
    toks: Vec<(usize, Tok)>,
    pos: usize,
}

impl P {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.1)
    }
    fn alt(&mut self) -> Result<Expr, String> {
        let mut alts = vec![self.seq()?];
        while self.peek() == Some(&Tok::Sym('|')) {
            self.pos += 1;
            alts.push(self.seq()?);
        }
        Ok(if alts.len() == 1 { alts.pop().unwrap() } else { Expr::Alt(alts) })
    }
    fn seq(&mut self) -> Result<Expr, String> {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None | Some(Tok::Sym('|')) | Some(Tok::Sym(')')) | Some(Tok::Sym(']'))
                | Some(Tok::Sym('}')) | Some(Tok::Sym(';')) => break,
                // a Name followed by '=' starts the next production (missing ';')
                Some(Tok::Name(_)) if self.toks.get(self.pos + 1).map(|t| &t.1) == Some(&Tok::Sym('=')) => break,
                _ => items.push(self.atom()?),
            }
        }
        Ok(if items.len() == 1 { items.pop().unwrap() } else { Expr::Seq(items) })
    }
    fn atom(&mut self) -> Result<Expr, String> {
        let (line, t) = self.toks[self.pos].clone();
        self.pos += 1;
        match t {
            Tok::Name(n) => Ok(Expr::Ref(n)),
            Tok::Str(s) => Ok(Expr::Lit(s)),
            Tok::Class(c) => Ok(Expr::Class(c)),
            Tok::Sym('(') => self.group(')', line, |e| e),
            Tok::Sym('[') => self.group(']', line, |e| Expr::Opt(Box::new(e))),
            Tok::Sym('{') => self.group('}', line, |e| Expr::Rep(Box::new(e))),
            Tok::Sym(c) => Err(format!("line {line}: unexpected {c:?}")),
        }
    }
    fn group(&mut self, close: char, line: usize, wrap: fn(Expr) -> Expr) -> Result<Expr, String> {
        let e = self.alt()?;
        if self.peek() != Some(&Tok::Sym(close)) {
            return Err(format!("line {line}: missing {close:?}"));
        }
        self.pos += 1;
        Ok(wrap(e))
    }
}

pub fn parse_grammar(src: &str) -> Result<Grammar, String> {
    let mut directives = Vec::new();
    let toks = lex(src, &mut directives)?;
    let mut p = P { toks, pos: 0 };
    let mut g = Grammar::default();
    while p.pos < p.toks.len() {
        let start = p.pos;
        let name = match &p.toks[p.pos].1 {
            Tok::Name(n) => n.clone(),
            t => return Err(format!("line {}: expected production name, got {t:?}", p.toks[p.pos].0)),
        };
        p.pos += 1;
        if p.peek() != Some(&Tok::Sym('=')) {
            return Err(format!("line {}: expected '=' after {name}", p.toks[start].0));
        }
        p.pos += 1;
        let body = p.alt()?;
        if p.peek() == Some(&Tok::Sym(';')) {
            p.pos += 1;
        }
        let mut prod = Production { name: name.clone(), body: Some(body), status: "parsed".into(), ..Default::default() };
        for (_, d) in directives.iter().filter(|(at, _)| *at == start) {
            apply(&mut prod, d)?;
        }
        if g.prods.insert(name.clone(), prod).is_some() {
            return Err(format!("duplicate production {name}"));
        }
        g.order.push(name);
    }
    // validate references
    for pr in g.prods.values() {
        check_refs(pr.body.as_ref().unwrap(), &g, &pr.name)?;
    }
    Ok(g)
}

fn apply(p: &mut Production, d: &str) -> Result<(), String> {
    let (k, v) = d.split_once(char::is_whitespace).map(|(a, b)| (a, b.trim())).unwrap_or((d, ""));
    match k {
        "ctx" => p.ctx = Some(v.to_string()),
        "nl" => p.nl = true,
        "status" => p.status = v.to_string(),
        "lex" => p.lex = Some(v.to_string()),
        "reject-keywords" => p.reject_keywords = true,
        _ => return Err(format!("unknown directive @{k} on {}", p.name)),
    }
    Ok(())
}

fn check_refs(e: &Expr, g: &Grammar, owner: &str) -> Result<(), String> {
    match e {
        Expr::Seq(v) | Expr::Alt(v) => v.iter().try_for_each(|x| check_refs(x, g, owner)),
        Expr::Opt(x) | Expr::Rep(x) => check_refs(x, g, owner),
        Expr::Ref(n) if !g.prods.contains_key(n) => Err(format!("{owner}: undefined production {n}")),
        _ => Ok(()),
    }
}
