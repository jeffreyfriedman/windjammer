use std::collections::{BTreeMap, HashSet};
use std::io::{self, Read};

const LIMIT: i64 = 1_000_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct Cell {
    col: u8, // 0..=9
    row: u8, // 1..=20
}

impl Cell {
    fn parse(s: &str) -> Option<Cell> {
        let b = s.as_bytes();
        if b.len() < 2 || b.len() > 3 || !(b'A'..=b'J').contains(&b[0]) {
            return None;
        }
        let digits = &s[1..];
        if digits.starts_with('0') || !digits.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let row: u8 = digits.parse().ok()?;
        if (1..=20).contains(&row) {
            Some(Cell { col: b[0] - b'A', row })
        } else {
            None
        }
    }
    fn name(&self) -> String {
        format!("{}{}", (b'A' + self.col) as char, self.row)
    }
}

#[derive(Clone, Debug)]
enum Ast {
    Num(i64),
    Ref(Cell),
    Neg(Box<Ast>),
    Bin(char, Box<Ast>, Box<Ast>),
    Call(String, Vec<Arg>),
}

#[derive(Clone, Debug)]
enum Arg {
    Range(Cell, Cell),
    Expr(Ast),
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(i64),
    Word(String),
    Sym(char),
}

fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < cs.len() {
        let c = cs[i];
        if c == ' ' {
            i += 1;
        } else if "+-*/(),:".contains(c) {
            out.push(Tok::Sym(c));
            i += 1;
        } else if c.is_ascii_digit() {
            let st = i;
            while i < cs.len() && cs[i].is_ascii_digit() {
                i += 1;
            }
            if i - st > 9 {
                return None;
            }
            let t: String = cs[st..i].iter().collect();
            out.push(Tok::Num(t.parse().ok()?));
        } else if c.is_ascii_alphabetic() {
            let st = i;
            while i < cs.len() && cs[i].is_ascii_alphanumeric() {
                i += 1;
            }
            out.push(Tok::Word(cs[st..i].iter().collect()));
        } else {
            return None;
        }
    }
    Some(out)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn sym(&mut self, c: char) -> Option<()> {
        if self.peek() == Some(&Tok::Sym(c)) {
            self.pos += 1;
            Some(())
        } else {
            None
        }
    }
    fn expr(&mut self) -> Option<Ast> {
        let mut a = self.term()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Sym(c)) if *c == '+' || *c == '-' => *c,
                _ => break,
            };
            self.pos += 1;
            let b = self.term()?;
            a = Ast::Bin(op, Box::new(a), Box::new(b));
        }
        Some(a)
    }
    fn term(&mut self) -> Option<Ast> {
        let mut a = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Sym(c)) if *c == '*' || *c == '/' => *c,
                _ => break,
            };
            self.pos += 1;
            let b = self.unary()?;
            a = Ast::Bin(op, Box::new(a), Box::new(b));
        }
        Some(a)
    }
    fn unary(&mut self) -> Option<Ast> {
        if self.sym('-').is_some() {
            return Some(Ast::Neg(Box::new(self.unary()?)));
        }
        self.primary()
    }
    fn primary(&mut self) -> Option<Ast> {
        let t = self.peek()?.clone();
        self.pos += 1;
        match t {
            Tok::Num(n) => Some(Ast::Num(n)),
            Tok::Sym('(') => {
                let e = self.expr()?;
                self.sym(')')?;
                Some(e)
            }
            Tok::Word(w) => {
                if self.peek() == Some(&Tok::Sym('(')) {
                    if !["SUM", "MIN", "MAX", "COUNT", "ABS"].contains(&w.as_str()) {
                        return None;
                    }
                    self.pos += 1;
                    let mut args = vec![self.arg()?];
                    while self.sym(',').is_some() {
                        args.push(self.arg()?);
                    }
                    self.sym(')')?;
                    if w == "ABS" && (args.len() != 1 || matches!(args[0], Arg::Range(..))) {
                        return None;
                    }
                    Some(Ast::Call(w, args))
                } else {
                    Some(Ast::Ref(Cell::parse(&w)?))
                }
            }
            _ => None,
        }
    }
    fn arg(&mut self) -> Option<Arg> {
        if let (Some(Tok::Word(a)), Some(Tok::Sym(':'))) = (self.toks.get(self.pos), self.toks.get(self.pos + 1)) {
            let a = Cell::parse(a)?;
            let b = match self.toks.get(self.pos + 2) {
                Some(Tok::Word(w)) => Cell::parse(w)?,
                _ => return None,
            };
            self.pos += 3;
            return Some(Arg::Range(a, b));
        }
        Some(Arg::Expr(self.expr()?))
    }
}

fn parse_formula(s: &str) -> Option<Ast> {
    let mut p = Parser { toks: tokenize(s)?, pos: 0 };
    let e = p.expr()?;
    if p.pos != p.toks.len() {
        return None;
    }
    Some(e)
}

fn range_cells(a: Cell, b: Cell) -> Vec<Cell> {
    let (c1, c2) = (a.col.min(b.col), a.col.max(b.col));
    let (r1, r2) = (a.row.min(b.row), a.row.max(b.row));
    let mut v = Vec::new();
    for r in r1..=r2 {
        for c in c1..=c2 {
            v.push(Cell { col: c, row: r });
        }
    }
    v
}

fn deps(ast: &Ast, out: &mut HashSet<Cell>) {
    match ast {
        Ast::Num(_) => {}
        Ast::Ref(c) => {
            out.insert(*c);
        }
        Ast::Neg(a) => deps(a, out),
        Ast::Bin(_, a, b) => {
            deps(a, out);
            deps(b, out);
        }
        Ast::Call(_, args) => {
            for a in args {
                match a {
                    Arg::Range(x, y) => out.extend(range_cells(*x, *y)),
                    Arg::Expr(e) => deps(e, out),
                }
            }
        }
    }
}

fn sorted_deps(ast: &Ast) -> Vec<Cell> {
    let mut s = HashSet::new();
    deps(ast, &mut s);
    let mut v: Vec<Cell> = s.into_iter().collect();
    v.sort();
    v
}

enum Content {
    Num(i64),
    Text(String),
    Formula(Ast),
}

#[derive(Clone, Debug, PartialEq)]
enum Val {
    N(i64),
    T(String),
    E(&'static str),
}

struct Sheet {
    cells: BTreeMap<Cell, Content>,
}

fn checked(v: i64) -> Val {
    if v.abs() > LIMIT {
        Val::E("#OVERFLOW")
    } else {
        Val::N(v)
    }
}

impl Sheet {
    fn value(&self, c: Cell) -> Val {
        match self.cells.get(&c) {
            None => Val::N(0),
            Some(Content::Num(n)) => Val::N(*n),
            Some(Content::Text(t)) => Val::T(t.clone()),
            Some(Content::Formula(a)) => self.eval(a),
        }
    }

    fn eval(&self, a: &Ast) -> Val {
        match a {
            Ast::Num(n) => Val::N(*n),
            Ast::Ref(c) => match self.value(*c) {
                Val::T(_) => Val::E("#VALUE"),
                v => v,
            },
            Ast::Neg(x) => match self.eval(x) {
                Val::N(n) => Val::N(-n),
                v => v,
            },
            Ast::Bin(op, x, y) => {
                let l = match self.eval(x) {
                    Val::N(n) => n,
                    v => return v,
                };
                let r = match self.eval(y) {
                    Val::N(n) => n,
                    v => return v,
                };
                match op {
                    '+' => checked(l + r),
                    '-' => checked(l - r),
                    '*' => l.checked_mul(r).map_or(Val::E("#OVERFLOW"), checked),
                    _ => {
                        if r == 0 {
                            Val::E("#DIV0")
                        } else {
                            checked(l / r)
                        }
                    }
                }
            }
            Ast::Call(f, args) => {
                let mut nums: Vec<i64> = Vec::new();
                for a in args {
                    match a {
                        Arg::Range(x, y) => {
                            for c in range_cells(*x, *y) {
                                match self.cells.get(&c) {
                                    None | Some(Content::Text(_)) => continue,
                                    _ => {}
                                }
                                match self.value(c) {
                                    Val::N(n) => nums.push(n),
                                    Val::E(e) => return Val::E(e),
                                    Val::T(_) => {}
                                }
                            }
                        }
                        Arg::Expr(e) => match self.eval(e) {
                            Val::N(n) => nums.push(n),
                            other => return other,
                        },
                    }
                }
                match f.as_str() {
                    "ABS" => Val::N(nums[0].abs()),
                    "COUNT" => Val::N(nums.len() as i64),
                    "SUM" => {
                        let mut t = 0i64;
                        for n in nums {
                            t += n;
                            if t.abs() > LIMIT {
                                return Val::E("#OVERFLOW");
                            }
                        }
                        Val::N(t)
                    }
                    "MIN" => nums.iter().min().map_or(Val::E("#VALUE"), |m| Val::N(*m)),
                    _ => nums.iter().max().map_or(Val::E("#VALUE"), |m| Val::N(*m)),
                }
            }
        }
    }

    fn show(&self, c: Cell) -> String {
        match self.cells.get(&c) {
            None => "(empty)".to_string(),
            Some(Content::Text(t)) => t.clone(),
            Some(_) => match self.value(c) {
                Val::N(n) => n.to_string(),
                Val::E(e) => e.to_string(),
                Val::T(t) => t,
            },
        }
    }

    // depth-first search for a path from `x` back to `x` using the new formula for x
    fn cycle(&self, x: Cell, ast: &Ast) -> Option<Vec<Cell>> {
        fn dfs(sheet: &Sheet, x: Cell, cur: Cell, ast: &Ast, seen: &mut HashSet<Cell>, path: &mut Vec<Cell>) -> bool {
            let next: Vec<Cell> = if cur == x {
                sorted_deps(ast)
            } else {
                match sheet.cells.get(&cur) {
                    Some(Content::Formula(a)) => sorted_deps(a),
                    _ => Vec::new(),
                }
            };
            for d in next {
                if d == x {
                    path.push(x);
                    return true;
                }
                if !seen.insert(d) {
                    continue;
                }
                path.push(d);
                if dfs(sheet, x, d, ast, seen, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        let mut seen = HashSet::new();
        let mut path = vec![x];
        if dfs(self, x, x, ast, &mut seen, &mut path) {
            Some(path)
        } else {
            None
        }
    }
}

fn is_number(s: &str) -> bool {
    let d = s.strip_prefix('-').unwrap_or(s);
    !d.is_empty() && d.len() <= 9 && d.bytes().all(|c| c.is_ascii_digit())
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut sheet = Sheet { cells: BTreeMap::new() };
    let mut out = String::new();
    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(' ').collect();
        let cell = p.get(1).and_then(|s| Cell::parse(s));
        match (p[0], p.len(), cell) {
            ("set", n, Some(c)) if n >= 3 => {
                let content = line.splitn(3, ' ').nth(2).unwrap();
                if content.is_empty() {
                    out.push_str("error: bad command\n");
                } else if let Some(f) = content.strip_prefix('=') {
                    match parse_formula(f) {
                        None => out.push_str("error: bad formula\n"),
                        Some(ast) => match sheet.cycle(c, &ast) {
                            Some(path) => {
                                let names: Vec<String> = path.iter().map(|c| c.name()).collect();
                                out.push_str(&format!("error: cycle {}\n", names.join(" -> ")));
                            }
                            None => {
                                sheet.cells.insert(c, Content::Formula(ast));
                            }
                        },
                    }
                } else if is_number(content) {
                    sheet.cells.insert(c, Content::Num(content.parse().unwrap()));
                } else {
                    sheet.cells.insert(c, Content::Text(content.to_string()));
                }
            }
            ("clear", 2, Some(c)) => {
                sheet.cells.remove(&c);
            }
            ("get", 2, Some(c)) => out.push_str(&format!("{} = {}\n", c.name(), sheet.show(c))),
            ("dump", 1, _) => {
                if sheet.cells.is_empty() {
                    out.push_str("(empty sheet)\n");
                }
                let mut keys: Vec<Cell> = sheet.cells.keys().copied().collect();
                keys.sort();
                for k in keys {
                    out.push_str(&format!("{} = {}\n", k.name(), sheet.show(k)));
                }
            }
            _ => out.push_str("error: bad command\n"),
        }
    }
    print!("{}", out);
}
