#[inline]
fn sorted_chars(s: &str) -> Vec<char> {
    let mut cs: Vec<char> = s.chars().collect::<Vec<char>>();
    cs.sort();
    cs
}

fn main() {
    println!("listen/silent: {}", is_anagram("listen", "silent"));
    println!("apple/pale: {}", is_anagram("apple", "pale"));
}

#[inline]
fn is_anagram(a: &str, b: &str) -> bool {
    sorted_chars(a) == sorted_chars(b)
}

