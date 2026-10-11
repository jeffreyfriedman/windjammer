use std::collections::HashMap;
use std::io::Read;

fn render(line: &str, vars: &HashMap<String, String>) -> String {
    let c: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if let Some((name, end)) = placeholder(&c, i) {
            match vars.get(&name) {
                Some(v) => out.push_str(v),
                None => out.push_str(&format!("<missing:{}>", name)),
            }
            i = end;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

fn placeholder(c: &[char], start: usize) -> Option<(String, usize)> {
    if c.get(start) != Some(&'{') || c.get(start + 1) != Some(&'{') {
        return None;
    }
    let mut i = start + 2;
    while c.get(i) == Some(&' ') {
        i += 1;
    }
    let name_start = i;
    while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
        i += 1;
    }
    if i == name_start {
        return None;
    }
    let name: String = c[name_start..i].iter().collect();
    while c.get(i) == Some(&' ') {
        i += 1;
    }
    if c.get(i) == Some(&'}') && c.get(i + 1) == Some(&'}') {
        Some((name, i + 2))
    } else {
        None
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.split('\n');
    let k: usize = lines.next().unwrap().trim().parse().unwrap();
    let mut vars = HashMap::new();
    for _ in 0..k {
        let l = lines.next().unwrap().trim_end_matches('\r');
        let (a, b) = l.split_once('=').unwrap_or((l, ""));
        vars.insert(a.to_string(), b.to_string());
    }
    let rest: Vec<&str> = lines.collect();
    let rendered: Vec<String> = rest.iter().map(|l| render(l, &vars)).collect();
    print!("{}", rendered.join("\n"));
}
