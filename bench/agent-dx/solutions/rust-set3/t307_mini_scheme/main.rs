use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::rc::Rc;

const RESERVED: [&str; 21] = [
    "define", "set!", "lambda", "let", "if", "begin", "while", "and", "or", "not", "print", "+", "*", "-", "/", "%", "<", ">", "<=", ">=", "=",
];
const LIMIT: i64 = 1_000_000_000_000_000;

#[derive(Debug)]
enum Form {
    Int(i64, usize),
    Bool(bool, usize),
    Sym(String, usize),
    List(Vec<Form>, usize),
}

struct Env {
    vars: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Env>>>,
}

#[derive(Clone)]
enum Value {
    Int(i64),
    Bool(bool),
    Closure(Rc<Closure>),
}

struct Closure {
    params: Vec<String>,
    body: Vec<Rc<Form>>,
    env: Rc<RefCell<Env>>,
}

enum Fail {
    Parse(usize, &'static str),
    Run(usize, String),
}

fn parse(src: &str) -> Result<Vec<Form>, Fail> {
    let mut stack: Vec<(Vec<Form>, usize)> = Vec::new();
    let mut top: Vec<Form> = Vec::new();
    let mut line = 1usize;
    let cs: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == ';' {
            while i < cs.len() && cs[i] != '\n' {
                i += 1;
            }
        } else if c == '(' {
            stack.push((std::mem::take(&mut top), line));
            i += 1;
        } else if c == ')' {
            let (mut parent, open_line) = stack.pop().ok_or(Fail::Parse(line, "unexpected )"))?;
            parent.push(Form::List(std::mem::take(&mut top), open_line));
            top = parent;
            i += 1;
        } else {
            let st = i;
            while i < cs.len() && !cs[i].is_whitespace() && !"();".contains(cs[i]) {
                i += 1;
            }
            let t: String = cs[st..i].iter().collect();
            let digits = t.strip_prefix('-').unwrap_or(&t);
            if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                if digits.len() > 15 {
                    return Err(Fail::Parse(line, "number too large"));
                }
                top.push(Form::Int(t.parse().unwrap(), line));
            } else if t == "#t" {
                top.push(Form::Bool(true, line));
            } else if t == "#f" {
                top.push(Form::Bool(false, line));
            } else {
                top.push(Form::Sym(t, line));
            }
        }
    }
    if let Some((_, l)) = stack.first() {
        return Err(Fail::Parse(*l, "unclosed ("));
    }
    Ok(top)
}

struct Interp {
    steps: u32,
    depth: u32,
    out: String,
}

fn lookup(env: &Rc<RefCell<Env>>, name: &str) -> Option<Value> {
    let mut cur = Some(env.clone());
    while let Some(e) = cur {
        if let Some(v) = e.borrow().vars.get(name) {
            return Some(v.clone());
        }
        cur = e.borrow().parent.clone();
    }
    None
}

fn assign(env: &Rc<RefCell<Env>>, name: &str, v: Value) -> Option<Value> {
    let mut cur = Some(env.clone());
    while let Some(e) = cur {
        if let Some(slot) = e.borrow_mut().vars.get_mut(name) {
            return Some(std::mem::replace(slot, v));
        }
        cur = e.borrow().parent.clone();
    }
    None
}

fn new_env(parent: &Rc<RefCell<Env>>) -> Rc<RefCell<Env>> {
    Rc::new(RefCell::new(Env { vars: HashMap::new(), parent: Some(parent.clone()) }))
}

fn is_reserved(s: &str) -> bool {
    RESERVED.contains(&s)
}

fn bad(line: usize) -> Fail {
    Fail::Run(line, "bad syntax".to_string())
}

fn names(forms: &[Form], line: usize) -> Result<Vec<String>, Fail> {
    let mut out: Vec<String> = Vec::new();
    for f in forms {
        match f {
            Form::Sym(s, _) if !is_reserved(s) && !out.contains(s) => out.push(s.clone()),
            _ => return Err(bad(line)),
        }
    }
    Ok(out)
}

fn int_of(v: &Value, line: usize) -> Result<i64, Fail> {
    match v {
        Value::Int(n) => Ok(*n),
        _ => Err(Fail::Run(line, "type error".to_string())),
    }
}

fn bool_of(v: &Value, line: usize) -> Result<bool, Fail> {
    match v {
        Value::Bool(b) => Ok(*b),
        _ => Err(Fail::Run(line, "type error".to_string())),
    }
}

fn checked(v: Option<i64>, line: usize) -> Result<i64, Fail> {
    match v {
        Some(x) if x.abs() <= LIMIT => Ok(x),
        _ => Err(Fail::Run(line, "overflow".to_string())),
    }
}

impl Interp {
    fn show(v: &Value) -> String {
        match v {
            Value::Int(n) => n.to_string(),
            Value::Bool(true) => "#t".to_string(),
            Value::Bool(false) => "#f".to_string(),
            Value::Closure(_) => "<closure>".to_string(),
        }
    }

    fn eval(&mut self, f: &Rc<Form>, env: &Rc<RefCell<Env>>) -> Result<Value, Fail> {
        match &**f {
            Form::Int(n, _) => Ok(Value::Int(*n)),
            Form::Bool(b, _) => Ok(Value::Bool(*b)),
            Form::Sym(s, line) => {
                if is_reserved(s) {
                    return Err(Fail::Run(*line, format!("unbound variable {}", s)));
                }
                lookup(env, s).ok_or_else(|| Fail::Run(*line, format!("unbound variable {}", s)))
            }
            Form::List(items, line) => self.eval_list(items, *line, env),
        }
    }

    fn eval_list(&mut self, items: &[Form], line: usize, env: &Rc<RefCell<Env>>) -> Result<Value, Fail> {
        self.steps += 1;
        if self.steps > 200_000 {
            return Err(Fail::Run(line, "step limit".to_string()));
        }
        let head = items.first().ok_or_else(|| bad(line))?;
        let args = &items[1..];
        if let Form::Sym(op, _) = head {
            if is_reserved(op) {
                return self.special(op, args, line, env);
            }
        }
        let callee = self.eval_ref(head, env)?;
        let clo = match callee {
            Value::Closure(c) => c,
            _ => return Err(Fail::Run(line, "not a function".to_string())),
        };
        let mut vals = Vec::new();
        for a in args {
            vals.push(self.eval_ref(a, env)?);
        }
        if vals.len() != clo.params.len() {
            return Err(Fail::Run(line, "wrong number of arguments".to_string()));
        }
        if self.depth >= 100 {
            return Err(Fail::Run(line, "stack overflow".to_string()));
        }
        let ne = new_env(&clo.env);
        for (p, v) in clo.params.iter().zip(vals) {
            ne.borrow_mut().vars.insert(p.clone(), v);
        }
        self.depth += 1;
        let mut last = Value::Int(0);
        for b in &clo.body {
            last = match self.eval(b, &ne) {
                Ok(v) => v,
                Err(e) => {
                    self.depth -= 1;
                    return Err(e);
                }
            };
        }
        self.depth -= 1;
        Ok(last)
    }

    // evaluate a borrowed form; lists are re-wrapped lazily by cloning the structure
    fn eval_ref(&mut self, f: &Form, env: &Rc<RefCell<Env>>) -> Result<Value, Fail> {
        match f {
            Form::List(items, line) => self.eval_list(items, *line, env),
            Form::Int(n, _) => Ok(Value::Int(*n)),
            Form::Bool(b, _) => Ok(Value::Bool(*b)),
            Form::Sym(s, line) => {
                if is_reserved(s) {
                    return Err(Fail::Run(*line, format!("unbound variable {}", s)));
                }
                lookup(env, s).ok_or_else(|| Fail::Run(*line, format!("unbound variable {}", s)))
            }
        }
    }

    fn special(&mut self, op: &str, a: &[Form], line: usize, env: &Rc<RefCell<Env>>) -> Result<Value, Fail> {
        match op {
            "define" | "set!" => {
                let name = match a {
                    [Form::Sym(n, _), _] if !is_reserved(n) => n.clone(),
                    _ => return Err(bad(line)),
                };
                let v = self.eval_ref(&a[1], env)?;
                if op == "define" {
                    env.borrow_mut().vars.insert(name, v);
                    Ok(Value::Int(0))
                } else {
                    assign(env, &name, v).ok_or_else(|| Fail::Run(line, format!("unbound variable {}", name)))
                }
            }
            "lambda" => match a {
                [Form::List(ps, _), body @ ..] if !body.is_empty() => {
                    let params = names(ps, line)?;
                    Ok(Value::Closure(Rc::new(Closure { params, body: body.iter().map(|f| Rc::new(clone_form(f))).collect(), env: env.clone() })))
                }
                _ => Err(bad(line)),
            },
            "let" => match a {
                [Form::List(bs, _), body @ ..] if !body.is_empty() => {
                    let mut ns = Vec::new();
                    let mut exprs = Vec::new();
                    for b in bs {
                        match b {
                            Form::List(p, _) if p.len() == 2 => {
                                ns.push(p[0].clone_sym().ok_or_else(|| bad(line))?);
                                exprs.push(&p[1]);
                            }
                            _ => return Err(bad(line)),
                        }
                    }
                    let mut seen: Vec<&String> = Vec::new();
                    for n in &ns {
                        if is_reserved(n) || seen.contains(&n) {
                            return Err(bad(line));
                        }
                        seen.push(n);
                    }
                    let mut vals = Vec::new();
                    for e in exprs {
                        vals.push(self.eval_ref(e, env)?);
                    }
                    let ne = new_env(env);
                    for (n, v) in ns.iter().zip(vals) {
                        ne.borrow_mut().vars.insert(n.clone(), v);
                    }
                    let mut last = Value::Int(0);
                    for b in body {
                        last = self.eval_ref(b, &ne)?;
                    }
                    Ok(last)
                }
                _ => Err(bad(line)),
            },
            "if" => {
                if a.len() != 2 && a.len() != 3 {
                    return Err(bad(line));
                }
                let c = self.eval_ref(&a[0], env)?;
                if bool_of(&c, line)? {
                    self.eval_ref(&a[1], env)
                } else if a.len() == 3 {
                    self.eval_ref(&a[2], env)
                } else {
                    Ok(Value::Int(0))
                }
            }
            "begin" => {
                let mut last = Value::Int(0);
                for b in a {
                    last = self.eval_ref(b, env)?;
                }
                Ok(last)
            }
            "while" => {
                if a.len() < 2 {
                    return Err(bad(line));
                }
                loop {
                    let c = self.eval_ref(&a[0], env)?;
                    if !bool_of(&c, line)? {
                        return Ok(Value::Int(0));
                    }
                    for b in &a[1..] {
                        self.eval_ref(b, env)?;
                    }
                }
            }
            "and" | "or" => {
                let is_and = op == "and";
                let mut result = is_and;
                for b in a {
                    let v = self.eval_ref(b, env)?;
                    result = bool_of(&v, line)?;
                    if result != is_and {
                        break;
                    }
                }
                Ok(Value::Bool(result))
            }
            "not" => {
                if a.len() != 1 {
                    return Err(bad(line));
                }
                let v = self.eval_ref(&a[0], env)?;
                Ok(Value::Bool(!bool_of(&v, line)?))
            }
            "print" => {
                if a.len() != 1 {
                    return Err(bad(line));
                }
                let v = self.eval_ref(&a[0], env)?;
                self.out.push_str(&Self::show(&v));
                self.out.push('\n');
                Ok(Value::Int(0))
            }
            _ => {
                let fixed2 = matches!(op, "/" | "%" | "<" | ">" | "<=" | ">=" | "=");
                if (fixed2 && a.len() != 2) || (op == "-" && a.is_empty()) {
                    return Err(bad(line));
                }
                let mut vals = Vec::new();
                for b in a {
                    vals.push(self.eval_ref(b, env)?);
                }
                let mut ns = Vec::new();
                for v in &vals {
                    ns.push(int_of(v, line)?);
                }
                match op {
                    "+" => {
                        let mut t = 0i64;
                        for n in ns {
                            t = checked(t.checked_add(n), line)?;
                        }
                        Ok(Value::Int(t))
                    }
                    "*" => {
                        let mut t = 1i64;
                        for n in ns {
                            t = checked(t.checked_mul(n), line)?;
                        }
                        Ok(Value::Int(t))
                    }
                    "-" => {
                        if ns.len() == 1 {
                            return Ok(Value::Int(-ns[0]));
                        }
                        let mut t = ns[0];
                        for n in &ns[1..] {
                            t = checked(t.checked_sub(*n), line)?;
                        }
                        Ok(Value::Int(t))
                    }
                    "/" | "%" => {
                        if ns[1] == 0 {
                            return Err(Fail::Run(line, "division by zero".to_string()));
                        }
                        Ok(Value::Int(if op == "/" { ns[0] / ns[1] } else { ns[0] % ns[1] }))
                    }
                    "<" => Ok(Value::Bool(ns[0] < ns[1])),
                    ">" => Ok(Value::Bool(ns[0] > ns[1])),
                    "<=" => Ok(Value::Bool(ns[0] <= ns[1])),
                    ">=" => Ok(Value::Bool(ns[0] >= ns[1])),
                    _ => Ok(Value::Bool(ns[0] == ns[1])),
                }
            }
        }
    }
}

impl Form {
    fn clone_sym(&self) -> Option<String> {
        match self {
            Form::Sym(s, _) => Some(s.clone()),
            _ => None,
        }
    }
}

fn clone_form(f: &Form) -> Form {
    match f {
        Form::Int(n, l) => Form::Int(*n, *l),
        Form::Bool(b, l) => Form::Bool(*b, *l),
        Form::Sym(s, l) => Form::Sym(s.clone(), *l),
        Form::List(v, l) => Form::List(v.iter().map(clone_form).collect(), *l),
    }
}

fn run() -> i32 {
    let mut src = String::new();
    io::stdin().read_to_string(&mut src).unwrap();
    let prog = match parse(&src) {
        Ok(p) => p,
        Err(Fail::Parse(l, m)) => {
            println!("parse error line {}: {}", l, m);
            return 2;
        }
        Err(Fail::Run(..)) => unreachable!(),
    };
    let global = Rc::new(RefCell::new(Env { vars: HashMap::new(), parent: None }));
    let mut it = Interp { steps: 0, depth: 0, out: String::new() };
    let mut code = 0;
    for f in &prog {
        if let Err(Fail::Run(l, m)) = it.eval_ref(f, &global) {
            it.out.push_str(&format!("error line {}: {}\n", l, m));
            code = 1;
            break;
        }
    }
    io::stdout().write_all(it.out.as_bytes()).unwrap();
    code
}

fn main() {
    // deep recursion in the evaluator: run on a big stack
    let h = std::thread::Builder::new().stack_size(256 * 1024 * 1024).spawn(run).unwrap();
    let code = h.join().unwrap();
    std::process::exit(code);
}
