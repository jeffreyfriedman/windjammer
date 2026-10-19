use windjammer_runtime::test::*;
#[inline]
fn parse_age(s: &str) -> Result<i64, String> {
    match s.parse::<i64>() {
        Ok(n) => {
            if n < 0 {
                Err(String::from("negative"))
            } else {
                Ok(n)
            }
        },
        Err(_) => Err(String::from("not a number")),
    }
}

#[inline]
fn first_word(s: &str) -> Option<String> {
    s.split_whitespace().next().map(|w| w.to_string())
}

#[test]
fn parses_valid() {
    assert_eq!(parse_age("42"), Ok(42));
}

#[test]
fn rejects_bad_input() {
    assert_eq!(parse_age("abc"), Err(String::from("not a number")));
    assert_eq!(parse_age("-1"), Err(String::from("negative")));
}

#[test]
fn first_word_works() {
    assert_eq!(first_word("hello world"), Some(String::from("hello")));
    assert_eq!(first_word("   "), None);
}

