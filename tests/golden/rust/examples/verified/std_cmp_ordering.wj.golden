use std::cmp::Ordering;
#[inline]
fn name(o: &Ordering) -> String {
    match o {
        Ordering::Less => String::from("Less"),
        Ordering::Greater => String::from("Greater"),
        Ordering::Equal => String::from("Equal"),
    }
}

fn main() {
    println!("{}", name(&1.cmp(&2_i64)));
    println!("{}", name(&5.cmp(&2_i64)));
    println!("{}", name(&3.cmp(&3_i64)));
    let mut v = vec![(2_i32, "c"), (1_i32, "b"), (2_i32, "a")];
    v.sort_by(|a, b| a.0.cmp(&b.0.clone()).then(a.1.cmp(&b.1.clone())));
    let strs: Vec<String> = { v.iter().map(|p| format!("({}, {})", p.0, p.1)) }.collect::<Vec<String>>();
    println!("[{}]", strs.join(", "));
}

