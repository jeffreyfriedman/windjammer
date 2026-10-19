#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Config {
    name: String,
    timeout: i32,
}

#[inline]
fn is_valid(state: &Config) -> bool {
    if state.name.is_empty() {
        return false;
    }
    if state.timeout < 0_i32 {
        return false;
    }
    true
}

fn main() {
    let cfg = Config { name: String::from("test"), timeout: 100_i32 };
    let valid = is_valid(&cfg);
    println!("Valid: {}", valid);
    println!("✅ Multi-use parameter inference works!");
}

