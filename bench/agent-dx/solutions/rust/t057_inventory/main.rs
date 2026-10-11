use std::collections::BTreeMap;
use std::io::Read;

struct Item {
    qty: i64,
    price: i64,
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut inv: BTreeMap<String, Item> = BTreeMap::new();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        let num = |i: usize| p.get(i).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
        match p[0] {
            "stock" if p.len() >= 4 => {
                let (qty, price) = (num(2), num(3));
                if qty <= 0 {
                    println!("error: bad quantity");
                } else if let Some(it) = inv.get_mut(p[1]) {
                    if it.price != price {
                        println!("error: price mismatch for {}", p[1]);
                    } else {
                        it.qty += qty;
                        println!("ok {}", it.qty);
                    }
                } else {
                    inv.insert(p[1].to_string(), Item { qty, price });
                    println!("ok {}", qty);
                }
            }
            "sell" if p.len() >= 3 => {
                let qty = num(2);
                if qty <= 0 {
                    println!("error: bad quantity");
                } else {
                    match inv.get_mut(p[1]) {
                        None => println!("error: unknown sku {}", p[1]),
                        Some(it) if it.qty < qty => {
                            println!("error: insufficient stock for {} (have {}, need {})", p[1], it.qty, qty)
                        }
                        Some(it) => {
                            it.qty -= qty;
                            println!("sold {} for {}", qty, qty * it.price);
                        }
                    }
                }
            }
            "remove" if p.len() >= 2 => match inv.get(p[1]) {
                None => println!("error: unknown sku {}", p[1]),
                Some(it) if it.qty > 0 => println!("error: not empty {}", p[1]),
                Some(_) => {
                    inv.remove(p[1]);
                    println!("removed {}", p[1]);
                }
            },
            "report" => {
                if inv.is_empty() {
                    println!("(empty)");
                } else {
                    let mut total = 0;
                    for (k, it) in &inv {
                        println!("{} {} x {} = {}", k, it.qty, it.price, it.qty * it.price);
                        total += it.qty * it.price;
                    }
                    println!("total value {}", total);
                }
            }
            _ => {}
        }
    }
}
