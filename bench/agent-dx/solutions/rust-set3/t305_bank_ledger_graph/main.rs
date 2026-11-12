use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read};

struct Account {
    balance: i64,
    journal: Vec<String>,
}

#[derive(Default)]
struct Bank {
    accounts: HashMap<String, Account>,
    book: Vec<(String, String, i64)>,
    out: Vec<String>,
    errors: usize,
}

impl Bank {
    fn error(&mut self, msg: String) {
        self.errors += 1;
        self.out.push(format!("error: {}", msg));
    }

    fn move_money(&mut self, from: &str, to: &str, amt: i64) {
        {
            let a = self.accounts.get_mut(from).unwrap();
            a.balance -= amt;
            a.journal.push(format!("out {} to {}", amt, to));
        }
        let b = self.accounts.get_mut(to).unwrap();
        b.balance += amt;
        b.journal.push(format!("in {} from {}", amt, from));
    }

    fn check_pair(&mut self, a: &str, b: &str) -> bool {
        if !self.accounts.contains_key(a) {
            self.error(format!("unknown account {}", a));
        } else if !self.accounts.contains_key(b) {
            self.error(format!("unknown account {}", b));
        } else if a == b {
            self.error("self transfer".to_string());
        } else {
            return true;
        }
        false
    }

    fn settle(&mut self) {
        let mut net: BTreeMap<String, i64> = BTreeMap::new();
        for (d, c, n) in self.book.drain(..) {
            *net.entry(d).or_insert(0) -= n;
            *net.entry(c).or_insert(0) += n;
        }
        let mut steps = 0;
        loop {
            let debtor = net
                .iter()
                .filter(|(_, &v)| v < 0)
                .min_by_key(|(k, &v)| (v, (*k).clone()))
                .map(|(k, &v)| (k.clone(), -v));
            let (d, debt) = match debtor {
                Some(x) => x,
                None => break,
            };
            let (c, credit) = net
                .iter()
                .filter(|(_, &v)| v > 0)
                .min_by_key(|(k, &v)| (-v, (*k).clone()))
                .map(|(k, &v)| (k.clone(), v))
                .expect("balanced book has a creditor");
            let amt = debt.min(credit);
            let mut line = format!("{} pays {} {}", d, c, amt);
            if self.accounts[&d].balance >= amt {
                self.move_money(&d, &c, amt);
            } else {
                line.push_str(" (failed: overdraft)");
            }
            *net.get_mut(&d).unwrap() += amt;
            *net.get_mut(&c).unwrap() -= amt;
            self.out.push(line);
            steps += 1;
        }
        if steps == 0 {
            self.out.push("nothing to settle".to_string());
        }
    }

    fn run(&mut self, line: &str) {
        let p: Vec<&str> = line.split(' ').collect();
        let amount = |s: &str, lo: i64| -> Option<i64> {
            if s.is_empty() || s.len() > 9 || !s.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            s.parse::<i64>().ok().filter(|&v| v >= lo)
        };
        if p.iter().any(|t| t.is_empty()) {
            return self.error("bad command".to_string());
        }
        match (p[0], p.len()) {
            ("open", 3) if amount(p[2], 0).is_some() => {
                let n = amount(p[2], 0).unwrap();
                if self.accounts.contains_key(p[1]) {
                    self.error(format!("account exists {}", p[1]));
                } else {
                    self.accounts.insert(p[1].to_string(), Account { balance: n, journal: vec![format!("open {}", n)] });
                }
            }
            ("transfer", 4) | ("iou", 4) if amount(p[3], 1).is_some() => {
                let n = amount(p[3], 1).unwrap();
                if self.check_pair(p[1], p[2]) {
                    if p[0] == "iou" {
                        self.book.push((p[1].to_string(), p[2].to_string(), n));
                    } else if self.accounts[p[1]].balance < n {
                        self.error(format!("overdraft {}", p[1]));
                    } else {
                        self.move_money(p[1], p[2], n);
                    }
                }
            }
            ("balance", 2) => match self.accounts.get(p[1]) {
                Some(a) => {
                    let s = format!("{}: {}", p[1], a.balance);
                    self.out.push(s);
                }
                None => self.error(format!("unknown account {}", p[1])),
            },
            ("all", 1) => {
                if self.accounts.is_empty() {
                    self.out.push("(no accounts)".to_string());
                }
                let mut names: Vec<&String> = self.accounts.keys().collect();
                names.sort();
                let lines: Vec<String> = names.iter().map(|n| format!("{}: {}", n, self.accounts[*n].balance)).collect();
                self.out.extend(lines);
            }
            ("history", 2) => match self.accounts.get(p[1]) {
                Some(a) => {
                    let j = a.journal.clone();
                    self.out.extend(j);
                }
                None => self.error(format!("unknown account {}", p[1])),
            },
            ("settle", 1) => self.settle(),
            _ => self.error("bad command".to_string()),
        }
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut bank = Bank::default();
    for line in input.lines() {
        if !line.is_empty() {
            bank.run(line);
        }
    }
    for l in &bank.out {
        println!("{}", l);
    }
    std::process::exit(if bank.errors > 0 { 1 } else { 0 });
}
