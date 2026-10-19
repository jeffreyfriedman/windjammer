fn main() {
    let s: String = String::from("hello");
    println!("{}", s.chars().count());
    let rev: String = s.chars().rev().collect::<String>();
    println!("{}", rev);
    let vowels = s.chars().filter(|c| *c == 'e' || *c == 'o').count();
    println!("{}", vowels);
    let spaced: Vec<String> = "Hello".chars().map(|c| c.to_string()).collect::<Vec<String>>();
    println!("{}", spaced.join(" "));
}

