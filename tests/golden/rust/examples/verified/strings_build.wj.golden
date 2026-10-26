fn main() {
    let mut s = String::new();
    s.push('a');
    s.push_str("bc");
    println!("{}", s);
    let mut out = String::new();
    for (i, chunk) in ["ab", "cd", "ef"].iter().enumerate() {
        if i > 0_usize {
            { use std::fmt::Write as _; write!(out, "{}", "-").expect("a formatting trait implementation returned an error"); };
        }
        out = out + &chunk;
    }
    println!("{}", out);
    println!("{}", "*".repeat(3_usize));
    let name: String = String::from("Bob");
    println!("{}{}{}", "Hi, ", name, "!");
}

