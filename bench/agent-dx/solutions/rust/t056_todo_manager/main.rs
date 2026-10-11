use std::io::Read;

struct Item {
    id: u64,
    text: String,
    done: bool,
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut items: Vec<Item> = Vec::new();
    let mut next_id = 1;
    for line in s.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (cmd, arg) = match line.split_once(' ') {
            Some((c, a)) => (c, a),
            None => (line, ""),
        };
        let find = |items: &Vec<Item>| arg.trim().parse::<u64>().ok().and_then(|id| items.iter().position(|i| i.id == id));
        match cmd {
            "add" => {
                if arg.is_empty() {
                    println!("error: missing argument");
                } else {
                    items.push(Item { id: next_id, text: arg.to_string(), done: false });
                    println!("added {}", next_id);
                    next_id += 1;
                }
            }
            "done" | "del" => {
                if arg.trim().is_empty() {
                    println!("error: missing argument");
                    continue;
                }
                match find(&items) {
                    None => println!("error: no such item {}", arg.trim()),
                    Some(p) => {
                        let id = items[p].id;
                        if cmd == "del" {
                            items.remove(p);
                            println!("deleted {}", id);
                        } else if items[p].done {
                            println!("error: already done {}", id);
                        } else {
                            items[p].done = true;
                            println!("done {}", id);
                        }
                    }
                }
            }
            "list" => {
                let f = if arg.trim().is_empty() { "all" } else { arg.trim() };
                if !["all", "open", "done"].contains(&f) {
                    println!("error: bad filter {}", f);
                    continue;
                }
                let sel: Vec<&Item> = items
                    .iter()
                    .filter(|i| f == "all" || (f == "done") == i.done)
                    .collect();
                if sel.is_empty() {
                    println!("(none)");
                }
                for i in sel {
                    println!("{} [{}] {}", i.id, if i.done { "x" } else { " " }, i.text);
                }
            }
            "clear" => {
                let before = items.len();
                items.retain(|i| !i.done);
                println!("cleared {}", before - items.len());
            }
            "count" => {
                let d = items.iter().filter(|i| i.done).count();
                println!("open={} done={}", items.len() - d, d);
            }
            _ => println!("error: unknown command {}", cmd),
        }
    }
}
