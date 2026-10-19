#[inline]
fn is_palindrome(s: &str) -> bool {
    let cleaned: Vec<char> = s.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect::<Vec<char>>();
    let n = cleaned.len();
    for i in 0_usize..n / 2_usize {
        if cleaned[i] != cleaned[n - 1 - i] {
            return false;
        }
    }
    true
}

fn main() {
    for s in ["racecar", "hello", "A man a plan a canal Panama"] {
        println!("{}: {}", s, is_palindrome(&s));
    }
}

