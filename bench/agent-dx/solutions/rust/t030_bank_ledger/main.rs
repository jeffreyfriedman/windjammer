use std::collections::BTreeMap;
use std::io::Read;

struct Bank {
    accounts: BTreeMap<String, i64>,
}

impl Bank {
    fn exists(&self, n: &str) -> Result<(), String> {
        if self.accounts.contains_key(n) {
            Ok(())
        } else {
            Err(format!("error: unknown account {}", n))
        }
    }

    fn run(&mut self, p: &[&str]) -> Result<Option<i64>, String> {
        let amt = |i: usize| -> i64 { p.get(i).and_then(|x| x.parse().ok()).unwrap_or(0) };
        match p[0] {
            "open" => {
                if amt(2) < 0 {
                    return Err("error: invalid amount".into());
                }
                if self.accounts.contains_key(p[1]) {
                    return Err("error: duplicate account".into());
                }
                self.accounts.insert(p[1].to_string(), amt(2));
                Ok(None)
            }
            "deposit" | "withdraw" => {
                let a = amt(2);
                if a <= 0 {
                    return Err("error: invalid amount".into());
                }
                self.exists(p[1])?;
                let bal = self.accounts.get_mut(p[1]).unwrap();
                if p[0] == "deposit" {
                    *bal += a;
                } else {
                    if *bal < a {
                        return Err("error: insufficient funds".into());
                    }
                    *bal -= a;
                }
                Ok(None)
            }
            "transfer" => {
                let a = amt(3);
                if a <= 0 {
                    return Err("error: invalid amount".into());
                }
                self.exists(p[1])?;
                self.exists(p[2])?;
                if p[1] == p[2] {
                    return Err("error: same account".into());
                }
                if self.accounts[p[1]] < a {
                    return Err("error: insufficient funds".into());
                }
                *self.accounts.get_mut(p[1]).unwrap() -= a;
                *self.accounts.get_mut(p[2]).unwrap() += a;
                Ok(None)
            }
            "balance" => {
                self.exists(p[1])?;
                Ok(Some(self.accounts[p[1]]))
            }
            _ => Ok(None),
        }
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut bank = Bank { accounts: BTreeMap::new() };
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() {
            continue;
        }
        match bank.run(&p) {
            Ok(Some(v)) => println!("{}", v),
            Ok(None) => println!("ok"),
            Err(e) => println!("{}", e),
        }
    }
    for (n, b) in &bank.accounts {
        println!("{} {}", n, b);
    }
}
