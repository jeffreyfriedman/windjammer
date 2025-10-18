struct SimpleArgs {
    input: String,
    output: Option<String>,
    verbose: bool,
}

#[inline]
fn parse<T>() -> T {
}

#[inline]
fn parse_from<T>(args: &Vec<String>) -> T {
}

#[inline]
fn try_parse<T>() -> Result<T, String> {
    Err("Parse error")
}

#[inline]
fn args() -> Vec<String> {
    vec![]
}

#[inline]
fn arg(index: i64) -> Option<String> {
    None
}

