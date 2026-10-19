#[inline]
fn describe(o: Option<i64>) -> String {
    let Some(v) = o else {
        return String::from("nothing");
    };
    format!("got {}", v)
}

fn main() {
    println!("{}", describe(Some(5_i64)));
    println!("{}", describe(None));
}

