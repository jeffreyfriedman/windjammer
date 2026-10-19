#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Address {
    city: String,
    country: String,
}

#[derive(Debug, Clone, PartialEq)]

struct User {
    name: String,
    addr: Address,
}

fn main() {
    let mut u = User { name: String::from("Ann"), addr: Address { city: String::from("Oslo"), country: String::from("NO") } };
    println!("{} lives in {} ({})", u.name, u.addr.city, u.addr.country);
    u.addr.city = String::from("Bergen");
    println!("{} moved to {}", u.name, u.addr.city);
}

