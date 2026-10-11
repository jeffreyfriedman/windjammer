use std::io::Read;

#[derive(Default)]
struct Node {
    key: i64,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

type Tree = Option<Box<Node>>;

fn insert(t: &mut Tree, x: i64) -> bool {
    match t {
        None => {
            *t = Some(Box::new(Node { key: x, left: None, right: None }));
            true
        }
        Some(n) => {
            if x == n.key {
                false
            } else if x < n.key {
                insert(&mut n.left, x)
            } else {
                insert(&mut n.right, x)
            }
        }
    }
}

fn contains(t: &Tree, x: i64) -> bool {
    match t {
        None => false,
        Some(n) => {
            if x == n.key {
                true
            } else if x < n.key {
                contains(&n.left, x)
            } else {
                contains(&n.right, x)
            }
        }
    }
}

fn take_min(t: &mut Tree) -> i64 {
    if t.as_ref().unwrap().left.is_some() {
        take_min(&mut t.as_mut().unwrap().left)
    } else {
        let n = t.take().unwrap();
        *t = n.right;
        n.key
    }
}

fn delete(t: &mut Tree, x: i64) -> bool {
    match t {
        None => false,
        Some(n) => {
            if x < n.key {
                delete(&mut n.left, x)
            } else if x > n.key {
                delete(&mut n.right, x)
            } else {
                match (n.left.is_some(), n.right.is_some()) {
                    (true, true) => {
                        n.key = take_min(&mut n.right);
                    }
                    (true, false) => {
                        let l = n.left.take();
                        *t = l;
                    }
                    _ => {
                        let r = n.right.take();
                        *t = r;
                    }
                }
                true
            }
        }
    }
}

fn inorder(t: &Tree, out: &mut Vec<String>) {
    if let Some(n) = t {
        inorder(&n.left, out);
        out.push(n.key.to_string());
        inorder(&n.right, out);
    }
}

fn height(t: &Tree) -> usize {
    match t {
        None => 0,
        Some(n) => 1 + height(&n.left).max(height(&n.right)),
    }
}

fn extreme(t: &Tree, left: bool) -> Option<i64> {
    let mut cur = t.as_ref()?;
    loop {
        let next = if left { &cur.left } else { &cur.right };
        match next {
            Some(n) => cur = n,
            None => return Some(cur.key),
        }
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut t: Tree = None;
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        let arg = || p[1].parse::<i64>().unwrap();
        match p[0] {
            "insert" => println!("{}", if insert(&mut t, arg()) { "ok" } else { "dup" }),
            "contains" => println!("{}", if contains(&t, arg()) { "yes" } else { "no" }),
            "delete" => println!("{}", if delete(&mut t, arg()) { "ok" } else { "missing" }),
            "inorder" => {
                let mut v = Vec::new();
                inorder(&t, &mut v);
                if v.is_empty() {
                    println!("<empty>");
                } else {
                    println!("{}", v.join(" "));
                }
            }
            "height" => println!("{}", height(&t)),
            "min" | "max" => match extreme(&t, p[0] == "min") {
                Some(v) => println!("{}", v),
                None => println!("empty"),
            },
            _ => {}
        }
    }
}
