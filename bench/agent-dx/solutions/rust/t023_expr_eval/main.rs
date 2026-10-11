use std::io::Read;

struct Parser {
    toks: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&mut self) -> Option<char> {
        while self.toks.get(self.pos) == Some(&' ') {
            self.pos += 1;
        }
        self.toks.get(self.pos).copied()
    }

    fn expr(&mut self) -> Option<i64> {
        let mut v = self.term()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.pos += 1;
            let r = self.term()?;
            v = if op == '+' { v.checked_add(r)? } else { v.checked_sub(r)? };
        }
        Some(v)
    }

    fn term(&mut self) -> Option<i64> {
        let mut v = self.unary()?;
        while let Some(op @ ('*' | '/' | '%')) = self.peek() {
            self.pos += 1;
            let r = self.unary()?;
            v = match op {
                '*' => v.checked_mul(r)?,
                '/' => v.checked_div(r)?,
                _ => v.checked_rem(r)?,
            };
        }
        Some(v)
    }

    fn unary(&mut self) -> Option<i64> {
        match self.peek()? {
            '-' => {
                self.pos += 1;
                self.unary()?.checked_neg()
            }
            '(' => {
                self.pos += 1;
                let v = self.expr()?;
                if self.peek()? != ')' {
                    return None;
                }
                self.pos += 1;
                Some(v)
            }
            c if c.is_ascii_digit() => {
                let mut v: i64 = 0;
                while let Some(d) = self.toks.get(self.pos).and_then(|c| c.to_digit(10)) {
                    v = v.checked_mul(10)?.checked_add(d as i64)?;
                    self.pos += 1;
                }
                Some(v)
            }
            _ => None,
        }
    }
}

fn eval(line: &str) -> Option<i64> {
    let toks: Vec<char> = line.chars().collect();
    let mut p = Parser { toks, pos: 0 };
    let v = p.expr()?;
    if p.peek().is_none() { Some(v) } else { None }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        match eval(line.trim_end_matches('\r')) {
            Some(v) => println!("{}", v),
            None => println!("error"),
        }
    }
}
