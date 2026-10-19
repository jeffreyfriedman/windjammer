#[inline]
fn parse_all(items: &Vec<String>) -> Result<Vec<i64>, String> {
    { items.as_ref().iter().map(|s| { s.parse::<i64>().map_err(|_| format!("bad: {}", s)) }) }.collect::<Vec<_>>()
}

fn main() {
    println!("{:?}", parse_all(&["1", "2", "3"]));
    println!("{:?}", parse_all(&["1", "x", "3"]));
}

