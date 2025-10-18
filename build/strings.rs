#[inline]
fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

#[inline]
fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

#[inline]
fn trim(s: &str) -> String {
    s.trim().to_string()
}

#[inline]
fn trim_start(s: &str) -> String {
    s.trim_start().to_string()
}

#[inline]
fn trim_end(s: &str) -> String {
    s.trim_end().to_string()
}

#[inline]
fn is_empty(s: &str) -> bool {
    s.is_empty()
}

#[inline]
fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

#[inline]
fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

#[inline]
fn contains(s: &str, substring: &str) -> bool {
    s.contains(substring)
}

#[inline]
fn replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

#[inline]
fn replacen(s: &str, from: &str, to: &str, count: usize) -> String {
    s.replacen(from, to, count)
}

#[inline]
fn len(s: &str) -> usize {
    s.len()
}

#[inline]
fn char_count(s: &str) -> usize {
    s.chars().count()
}

#[inline]
fn repeat(s: &str, n: usize) -> String {
    s.repeat(n)
}

