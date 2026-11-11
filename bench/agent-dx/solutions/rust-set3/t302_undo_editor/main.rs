use std::io::{self, Read};

trait Edit {
    fn apply(&self, buf: &mut Vec<String>);
    fn revert(&self, buf: &mut Vec<String>);
}

struct Insert {
    at: usize,
    text: String,
}
struct Delete {
    at: usize,
    old: String,
}
struct Replace {
    at: usize,
    old: String,
    new: String,
}
struct Swap {
    a: usize,
    b: usize,
}

impl Edit for Insert {
    fn apply(&self, buf: &mut Vec<String>) {
        buf.insert(self.at, self.text.clone());
    }
    fn revert(&self, buf: &mut Vec<String>) {
        buf.remove(self.at);
    }
}
impl Edit for Delete {
    fn apply(&self, buf: &mut Vec<String>) {
        buf.remove(self.at);
    }
    fn revert(&self, buf: &mut Vec<String>) {
        buf.insert(self.at, self.old.clone());
    }
}
impl Edit for Replace {
    fn apply(&self, buf: &mut Vec<String>) {
        buf[self.at] = self.new.clone();
    }
    fn revert(&self, buf: &mut Vec<String>) {
        buf[self.at] = self.old.clone();
    }
}
impl Edit for Swap {
    fn apply(&self, buf: &mut Vec<String>) {
        buf.swap(self.a, self.b);
    }
    fn revert(&self, buf: &mut Vec<String>) {
        buf.swap(self.a, self.b);
    }
}

struct Editor {
    buf: Vec<String>,
    undo: Vec<Box<dyn Edit>>,
    redo: Vec<Box<dyn Edit>>,
}

impl Editor {
    fn commit(&mut self, e: Box<dyn Edit>) {
        e.apply(&mut self.buf);
        self.undo.push(e);
        if self.undo.len() > 5 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

fn index(s: &str) -> Option<usize> {
    if s.is_empty() || s.len() > 6 || !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn run(ed: &mut Editor, line: &str, out: &mut String) -> Result<(), &'static str> {
    let w: Vec<&str> = line.split(' ').collect();
    match w[0] {
        "insert" | "replace" => {
            let p: Vec<&str> = line.splitn(3, ' ').collect();
            if p.len() != 3 || p[2].is_empty() {
                return Err("bad arguments");
            }
            let n = index(p[1]).ok_or("bad arguments")?;
            if w[0] == "insert" {
                if n > ed.buf.len() {
                    return Err("bad index");
                }
                ed.commit(Box::new(Insert { at: n, text: p[2].to_string() }));
            } else {
                if n >= ed.buf.len() {
                    return Err("bad index");
                }
                let old = ed.buf[n].clone();
                ed.commit(Box::new(Replace { at: n, old, new: p[2].to_string() }));
            }
        }
        "delete" => {
            if w.len() != 2 {
                return Err("bad arguments");
            }
            let n = index(w[1]).ok_or("bad arguments")?;
            if n >= ed.buf.len() {
                return Err("bad index");
            }
            let old = ed.buf[n].clone();
            ed.commit(Box::new(Delete { at: n, old }));
        }
        "swap" => {
            if w.len() != 3 {
                return Err("bad arguments");
            }
            let a = index(w[1]).ok_or("bad arguments")?;
            let b = index(w[2]).ok_or("bad arguments")?;
            if a >= ed.buf.len() || b >= ed.buf.len() {
                return Err("bad index");
            }
            ed.commit(Box::new(Swap { a, b }));
        }
        "undo" | "redo" | "print" => {
            if w.len() != 1 {
                return Err("bad arguments");
            }
            match w[0] {
                "print" => {
                    if ed.buf.is_empty() {
                        out.push_str("(empty)\n");
                    }
                    for (i, t) in ed.buf.iter().enumerate() {
                        out.push_str(&format!("{}: {}\n", i, t));
                    }
                }
                "undo" => {
                    let e = ed.undo.pop().ok_or("nothing to undo")?;
                    e.revert(&mut ed.buf);
                    ed.redo.push(e);
                }
                _ => {
                    let e = ed.redo.pop().ok_or("nothing to redo")?;
                    e.apply(&mut ed.buf);
                    ed.undo.push(e);
                }
            }
        }
        _ => return Err("unknown command"),
    }
    Ok(())
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut ed = Editor { buf: Vec::new(), undo: Vec::new(), redo: Vec::new() };
    let mut out = String::new();
    let mut errors = 0;
    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        if let Err(m) = run(&mut ed, line, &mut out) {
            errors += 1;
            out.push_str(&format!("error: {}\n", m));
        }
    }
    print!("{}", out);
    std::process::exit(if errors > 0 { 1 } else { 0 });
}
