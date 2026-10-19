#[inline]
fn encode(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect::<Vec<char>>();
    let mut out = String::new();
    let mut i: usize = 0_usize;
    while i < cs.len() {
        let mut j = i;
        while j < cs.len() && cs[j] == cs[i] {
            j += 1;
        }
        out = out + &cs[i].to_string() + &(j - i).to_string();
        i = j;
    }
    out
}

#[inline]
fn decode(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect::<Vec<char>>();
    let mut out = String::new();
    let mut i: usize = 0_usize;
    while i < cs.len() {
        let n = cs[i + 1].to_digit(10).unwrap_or(0) as i64;
        out = out + &cs[i].to_string().repeat(n as usize);
        i += 2;
    }
    out
}

fn main() {
    let e = encode("aaabbc");
    println!("{}", e);
    println!("{}", decode(&e));
}

