//! Rust's panic messages for out-of-range slicing (`v[a..b]`, `s[a..b]`).
//!
//! The parser lowers `x[a..]` to `x.slice(a, x.len())`, so an end equal to the length with a start
//! past it is reported the way Rust reports `x[a..]` ("range start index").

/// `None` when `v[a..b]` is valid for a slice of length `len`.
pub(crate) fn slice_error(a: i128, b: i128, len: usize) -> Option<String> {
    let len_i = len as i128;
    if a > b {
        if b == len_i && a > len_i {
            return Some(format!("range start index {} out of range for slice of length {}", a, len));
        }
        return Some(format!("slice index starts at {} but ends at {}", a, b));
    }
    if b > len_i {
        return Some(format!("range end index {} out of range for slice of length {}", b, len));
    }
    None
}

/// `None` when `s[a..b]` is valid. Messages are those of the current Rust standard library.
pub(crate) fn str_slice_error(s: &str, a: i128, b: i128) -> Option<String> {
    let len = s.len() as i128;
    if a > len {
        return Some(format!("start byte index {} is out of bounds for string of length {}", a, len));
    }
    if b > len {
        return Some(format!("end byte index {} is out of bounds for string of length {}", b, len));
    }
    if a > b {
        return Some(format!("byte range starts at {} but ends at {}", a, b));
    }
    for (which, i) in [("start", a), ("end", b)] {
        let i = i as usize;
        if !s.is_char_boundary(i) {
            let start = (0..=i).rev().find(|k| s.is_char_boundary(*k)).unwrap_or(0);
            let ch = s[start..].chars().next().unwrap_or('?');
            return Some(format!(
                "{} byte index {} is not a char boundary; it is inside {:?} (bytes {}..{} of string)",
                which,
                i,
                ch,
                start,
                start + ch.len_utf8()
            ));
        }
    }
    None
}
