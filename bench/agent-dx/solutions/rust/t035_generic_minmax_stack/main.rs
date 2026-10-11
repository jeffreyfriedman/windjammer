use std::fmt::Display;
use std::io::Read;

struct MinMaxStack<T: Ord + Clone + Display> {
    items: Vec<(T, T, T)>, // value, min so far, max so far
}

impl<T: Ord + Clone + Display> MinMaxStack<T> {
    fn new() -> Self {
        MinMaxStack { items: Vec::new() }
    }
    fn push(&mut self, v: T) {
        let (mn, mx) = match self.items.last() {
            Some((_, mn, mx)) => (v.clone().min(mn.clone()), v.clone().max(mx.clone())),
            None => (v.clone(), v.clone()),
        };
        self.items.push((v, mn, mx));
    }
    fn run(&mut self, cmd: &str, arg: Option<T>) {
        match cmd {
            "push" => {
                if let Some(a) = arg {
                    self.push(a)
                }
            }
            "pop" => match self.items.pop() {
                Some((v, _, _)) => println!("{}", v),
                None => println!("empty"),
            },
            "min" => match self.items.last() {
                Some((_, mn, _)) => println!("{}", mn),
                None => println!("empty"),
            },
            "max" => match self.items.last() {
                Some((_, _, mx)) => println!("{}", mx),
                None => println!("empty"),
            },
            "size" => println!("{}", self.items.len()),
            _ => {}
        }
    }
}

fn drive<T: Ord + Clone + Display>(lines: &[&str], parse: impl Fn(&str) -> T) {
    let mut st = MinMaxStack::new();
    for line in lines {
        if let Some(rest) = line.strip_prefix("push ") {
            st.run("push", Some(parse(rest)));
        } else if *line == "push" {
            st.run("push", Some(parse("")));
        } else {
            st.run(line.trim(), None);
        }
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    if lines.is_empty() {
        return;
    }
    match lines[0].trim() {
        "int" => drive(&lines[1..], |x| x.trim().parse::<i64>().unwrap()),
        _ => drive(&lines[1..], |x| x.to_string()),
    }
}
