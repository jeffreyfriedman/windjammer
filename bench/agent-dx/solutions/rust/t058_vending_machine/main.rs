use std::io::Read;

fn coins(mut amount: u32) -> String {
    let mut out: Vec<String> = Vec::new();
    for d in [100, 25, 10, 5] {
        while amount >= d {
            out.push(d.to_string());
            amount -= d;
        }
    }
    if out.is_empty() { "none".to_string() } else { out.join(" ") }
}

struct Slot {
    name: &'static str,
    price: u32,
    stock: u32,
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut slots = vec![
        Slot { name: "cola", price: 150, stock: 3 },
        Slot { name: "chips", price: 100, stock: 2 },
        Slot { name: "gum", price: 35, stock: 5 },
    ];
    let mut credit = 0u32;
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match (p[0], p.get(1)) {
            ("coin", Some(v)) => match v.parse::<u32>() {
                Ok(c) if [5, 10, 25, 100].contains(&c) => {
                    credit += c;
                    println!("credit {}", credit);
                }
                _ => println!("rejected {}", v),
            },
            ("select", Some(item)) => match slots.iter_mut().find(|s| s.name == *item) {
                None => println!("error: unknown item {}", item),
                Some(sl) if sl.stock == 0 => println!("error: sold out {}", item),
                Some(sl) if credit < sl.price => println!("error: need {} more", sl.price - credit),
                Some(sl) => {
                    sl.stock -= 1;
                    println!("dispensed {} change: {}", item, coins(credit - sl.price));
                    credit = 0;
                }
            },
            ("refund", _) => {
                println!("refund: {}", coins(credit));
                credit = 0;
            }
            ("status", _) => println!(
                "credit={} cola={} chips={} gum={}",
                credit, slots[0].stock, slots[1].stock, slots[2].stock
            ),
            _ => {}
        }
    }
}
