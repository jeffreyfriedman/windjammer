#[derive(Clone, Debug, PartialEq, Copy)]
enum Token {
    Num(i64),
    Plus,
    Star,
    LParen,
    RParen,
}

#[inline]
fn show(t: Token) -> String {
    match t {
        Token::Num(n) => format!("Num({})", n),
        Token::Plus => String::from("Plus"),
        Token::Star => String::from("Star"),
        Token::LParen => String::from("LParen"),
        Token::RParen => String::from("RParen"),
    }
}


fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect::<Vec<char>>();
    let mut toks: Vec<Token> = vec![];
    let mut i: usize = 0_usize;
    while i < chars.len() {
        let mut c = chars[i];
        if c.is_numeric() {
            let mut n: i32 = 0_i32;
            while i < chars.len() && chars[i].is_numeric() {
                n = n * 10_i32 + (chars[i].to_digit(10).unwrap_or(0) as i64) as i32;
                i += 1;
            }
            let _ = toks.push(Token::Num(n));
            continue;
        }
        if c == '+' {
            let _ = toks.push(Token::Plus);
        }
        if c == '*' {
            let _ = toks.push(Token::Star);
        }
        if c == '(' {
            let _ = toks.push(Token::LParen);
        }
        if c == ')' {
            let _ = toks.push(Token::RParen);
        }
        i += 1;
    }
    toks
}

fn main() {
    let strs: Vec<String> = tokenize("12 + 7 * (3)").into_iter().map(|t| show(t)).collect::<Vec<String>>();
    println!("{}", strs.join(" "));
}

