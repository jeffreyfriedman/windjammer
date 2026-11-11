use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::{self, Read};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tif {
    Gtc,
    Ioc,
    Fok,
    Market,
}

struct Order {
    id: u64,
    trader: String,
    qty: u64,
}

#[derive(Default)]
struct Position {
    qty: i64,
    cash: i64,
}

#[derive(Default)]
struct Book {
    bids: BTreeMap<u64, VecDeque<Order>>,
    asks: BTreeMap<u64, VecDeque<Order>>,
    resting: HashMap<u64, (Side, u64)>,
    positions: BTreeMap<String, Position>,
    next_id: u64,
    out: Vec<String>,
    errors: usize,
}

fn num(s: &str, lo: u64, hi: u64) -> Option<u64> {
    if s.is_empty() || s.len() > 7 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let v: u64 = s.parse().ok()?;
    if v >= lo && v <= hi {
        Some(v)
    } else {
        None
    }
}

impl Book {
    fn error(&mut self, m: String) {
        self.errors += 1;
        self.out.push(format!("error: {}", m));
    }

    fn side_mut(&mut self, s: Side) -> &mut BTreeMap<u64, VecDeque<Order>> {
        match s {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        }
    }

    fn submit(&mut self, side: Side, trader: &str, qty: u64, limit: Option<u64>, tif: Tif) {
        self.positions.entry(trader.to_string()).or_default();
        self.next_id += 1;
        let id = self.next_id;
        self.out.push(format!("order {}", id));
        let crosses = |px: u64| match (side, limit) {
            (_, None) => true,
            (Side::Buy, Some(l)) => px <= l,
            (Side::Sell, Some(l)) => px >= l,
        };
        let opp_side = if side == Side::Buy { Side::Sell } else { Side::Buy };
        if tif == Tif::Fok {
            let opp = if side == Side::Buy { &self.asks } else { &self.bids };
            let avail: u64 = opp
                .iter()
                .filter(|(px, _)| crosses(**px))
                .flat_map(|(_, q)| q.iter())
                .filter(|o| o.trader != trader)
                .map(|o| o.qty)
                .sum();
            if avail < qty {
                self.out.push(format!("reject {} fok", id));
                return;
            }
        }
        let mut rem = qty;
        while rem > 0 {
            let best = {
                let opp = if side == Side::Buy { &self.asks } else { &self.bids };
                if side == Side::Buy { opp.keys().next().copied() } else { opp.keys().next_back().copied() }
            };
            let px = match best {
                Some(p) if crosses(p) => p,
                _ => break,
            };
            let level = self.side_mut(opp_side).get_mut(&px).unwrap();
            let front = level.front_mut().unwrap();
            let rid = front.id;
            if front.trader == trader {
                level.pop_front();
                if level.is_empty() {
                    self.side_mut(opp_side).remove(&px);
                }
                self.resting.remove(&rid);
                self.out.push(format!("cancel {} self-trade", rid));
                continue;
            }
            let q = rem.min(front.qty);
            let other = front.trader.clone();
            front.qty -= q;
            let done = front.qty == 0;
            if done {
                level.pop_front();
                if level.is_empty() {
                    self.side_mut(opp_side).remove(&px);
                }
                self.resting.remove(&rid);
            }
            let (bid, sid) = if side == Side::Buy { (id, rid) } else { (rid, id) };
            self.out.push(format!("trade {} {} {} @ {}", bid, sid, q, px));
            let (buyer, seller) = if side == Side::Buy { (trader.to_string(), other) } else { (other, trader.to_string()) };
            let b = self.positions.get_mut(&buyer).unwrap();
            b.qty += q as i64;
            b.cash -= (q * px) as i64;
            let s = self.positions.get_mut(&seller).unwrap();
            s.qty -= q as i64;
            s.cash += (q * px) as i64;
            rem -= q;
        }
        if rem > 0 {
            if tif == Tif::Gtc {
                let px = limit.unwrap();
                self.side_mut(side).entry(px).or_default().push_back(Order { id, trader: trader.to_string(), qty: rem });
                self.resting.insert(id, (side, px));
                self.out.push(format!("rest {} {}", id, rem));
            } else {
                self.out.push(format!("drop {} {}", id, rem));
            }
        }
    }

    fn change(&mut self, trader: &str, id: u64, new_qty: Option<u64>) {
        let (side, px) = match self.resting.get(&id) {
            Some(&r) => r,
            None => return self.error(format!("no such order {}", id)),
        };
        let level = self.side_mut(side).get_mut(&px).unwrap();
        let pos = level.iter().position(|o| o.id == id).unwrap();
        if level[pos].trader != trader {
            return self.error("not owner".to_string());
        }
        match new_qty {
            None => {
                let o = level.remove(pos).unwrap();
                if level.is_empty() {
                    self.side_mut(side).remove(&px);
                }
                self.resting.remove(&id);
                self.out.push(format!("cancelled {} {}", id, o.qty));
            }
            Some(q) => {
                if q > level[pos].qty {
                    let mut o = level.remove(pos).unwrap();
                    o.qty = q;
                    level.push_back(o);
                } else {
                    level[pos].qty = q;
                }
                self.out.push(format!("amended {} {}", id, q));
            }
        }
    }

    fn show(&mut self) {
        let mut lines = vec!["asks:".to_string()];
        let render = |m: &BTreeMap<u64, VecDeque<Order>>, lines: &mut Vec<String>| {
            if m.is_empty() {
                lines.push("  (none)".to_string());
            }
            for (px, q) in m.iter().rev() {
                lines.push(format!("  {} {} {}", px, q.iter().map(|o| o.qty).sum::<u64>(), q.len()));
            }
        };
        render(&self.asks, &mut lines);
        lines.push("bids:".to_string());
        render(&self.bids, &mut lines);
        self.out.extend(lines);
    }

    fn positions(&mut self) {
        if self.positions.is_empty() {
            self.out.push("(no traders)".to_string());
        }
        let lines: Vec<String> = self.positions.iter().map(|(t, p)| format!("{} {} {}", t, p.qty, p.cash)).collect();
        self.out.extend(lines);
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut book = Book::default();
    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.split(' ').collect();
        if p.iter().any(|t| t.is_empty()) {
            book.error("bad command".to_string());
            continue;
        }
        let side = match p[0] {
            "buy" | "mbuy" => Some(Side::Buy),
            "sell" | "msell" => Some(Side::Sell),
            _ => None,
        };
        match (p[0], p.len(), side) {
            ("buy", 4 | 5, Some(s)) | ("sell", 4 | 5, Some(s)) => {
                let tif = match p.get(4) {
                    None => Some(Tif::Gtc),
                    Some(&"ioc") => Some(Tif::Ioc),
                    Some(&"fok") => Some(Tif::Fok),
                    Some(_) => None,
                };
                match (num(p[2], 1, 1_000_000), num(p[3], 1, 100_000), tif) {
                    (Some(q), Some(px), Some(t)) => book.submit(s, p[1], q, Some(px), t),
                    _ => book.error("bad command".to_string()),
                }
            }
            ("mbuy", 3, Some(s)) | ("msell", 3, Some(s)) => match num(p[2], 1, 1_000_000) {
                Some(q) => book.submit(s, p[1], q, None, Tif::Market),
                None => book.error("bad command".to_string()),
            },
            ("cancel", 3, _) => match num(p[2], 1, 9_999_999) {
                Some(id) => book.change(p[1], id, None),
                None => book.error("bad command".to_string()),
            },
            ("amend", 4, _) => match (num(p[2], 1, 9_999_999), num(p[3], 1, 1_000_000)) {
                (Some(id), Some(q)) => book.change(p[1], id, Some(q)),
                _ => book.error("bad command".to_string()),
            },
            ("book", 1, _) => book.show(),
            ("pos", 1, _) => book.positions(),
            _ => book.error("bad command".to_string()),
        }
    }
    for l in &book.out {
        println!("{}", l);
    }
    std::process::exit(if book.errors > 0 { 1 } else { 0 });
}
