#[derive(Clone, Debug, PartialEq)]
enum ParseError {
    Empty,
    BadNumber(String),
    TooLarge(i64),
}

#[inline]
fn describe(e: ParseError) -> String {
    match e {
        ParseError::Empty => String::from("empty input"),
        ParseError::BadNumber(s) => format!("bad number '{}'", s),
        ParseError::TooLarge(n) => format!("too large: {}", n),
    }
}

#[inline]
fn parse_amount(s: &str) -> Result<i64, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }
    let n = match s.parse::<i64>() {
        Ok(n) => n,
        Err(_) => {
            return Err(ParseError::BadNumber(s.to_string()));
        },
    };
    if n > 1000_i64 {
        return Err(ParseError::TooLarge(n));
    }
    Ok(n)
}

#[inline]
fn double_amount(s: String) -> Result<i64, ParseError> {
    let n = parse_amount(&s)?;
    Ok(n * 2)
}

fn main() {
    for input in ["10", "", "x1", "5000"] {
        match double_amount(input) {
            Ok(v) => println!("ok: {}", v),
            Err(e) => println!("error: {}", describe(e)),
        }
    }
}

