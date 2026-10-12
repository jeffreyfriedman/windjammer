#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Account {
    balance: i64,
}









impl Account {
#[inline]
fn deposit(&mut self, amt: i64) {
        self.balance += amt;
}
#[inline]
fn withdraw(&mut self, amt: i64) -> Result<i64, String> {
        if amt > self.balance {
            return { let _temp0 = format!("insufficient funds (have {}, need {})", self.balance, amt); Err(_temp0) };
        }
        self.balance -= amt;
        Ok(self.balance)
}
}

fn main() {
    let mut acct = Account { balance: 0_i64 };
    acct.deposit(100_i64);
    println!("balance {}", acct.balance);
    match acct.withdraw(30_i64) {
        Ok(left) => println!("left {}", left),
        Err(e) => println!("error: {}", e),
    }
    println!("balance {}", acct.balance);
    match acct.withdraw(100_i64) {
        Ok(_) => println!("ok"),
        Err(e) => println!("error: {}", e),
    }
}

