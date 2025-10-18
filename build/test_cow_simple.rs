#[inline]
fn process_text(text: &String, make_uppercase: bool) -> String {
    if make_uppercase {
        text
    } else {
        text
    }
}

fn main() {
    let result1 = process_text(&"hello", true);
    let result2 = process_text(&"world", false);
    println!("Result1: {}", result1);
    println!("Result2: {}", result2)
}

