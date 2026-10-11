use std::io::Read;

enum Expr {
    Lit(i64),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
}

impl Expr {
    fn eval(&self) -> i64 {
        match self {
            Expr::Lit(v) => *v,
            Expr::Neg(a) => a.eval().wrapping_neg(),
            Expr::Bin(op, a, b) => {
                let (x, y) = (a.eval(), b.eval());
                match op {
                    '+' => x.wrapping_add(y),
                    '-' => x.wrapping_sub(y),
                    _ => x.wrapping_mul(y),
                }
            }
        }
    }
    fn depth(&self) -> usize {
        match self {
            Expr::Lit(_) => 1,
            Expr::Neg(a) => 1 + a.depth(),
            Expr::Bin(_, a, b) => 1 + a.depth().max(b.depth()),
        }
    }
    fn nodes(&self) -> usize {
        match self {
            Expr::Lit(_) => 1,
            Expr::Neg(a) => 1 + a.nodes(),
            Expr::Bin(_, a, b) => 1 + a.nodes() + b.nodes(),
        }
    }
    fn render(&self) -> String {
        match self {
            Expr::Lit(v) => v.to_string(),
            Expr::Neg(a) => format!("-({})", a.render()),
            Expr::Bin(op, a, b) => format!("({} {} {})", a.render(), op, b.render()),
        }
    }
}

fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c == '(' || c == ')' || c == ' ' {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            if c != ' ' {
                out.push(c.to_string());
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse(t: &[String], pos: &mut usize) -> Expr {
    let tok = &t[*pos];
    *pos += 1;
    if tok != "(" {
        return Expr::Lit(tok.parse().unwrap());
    }
    let op = t[*pos].clone();
    *pos += 1;
    let e = if op == "neg" {
        Expr::Neg(Box::new(parse(t, pos)))
    } else {
        let a = parse(t, pos);
        let b = parse(t, pos);
        Expr::Bin(op.chars().next().unwrap(), Box::new(a), Box::new(b))
    };
    *pos += 1; // ')'
    e
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let toks = tokenize(line);
        let mut pos = 0;
        let e = parse(&toks, &mut pos);
        println!("{} {} {} {}", e.eval(), e.depth(), e.nodes(), e.render());
    }
}
