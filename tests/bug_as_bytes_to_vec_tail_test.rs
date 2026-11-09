//! `text.as_bytes().to_vec()` returned from a function was emitted as `*text.as_bytes().to_vec()` (and a local bound to
//! it as `*v`): the signature metadata typed `String::as_bytes` as `&u8`, so `.to_vec()` looked like a Copy `u8` behind a
//! reference and the tail got a deref, which rustc rejects (`expected Vec<u8>, found [u8]`). Found while building the
//! `wj-redis` example of ADR-0032. Oracle: Rust's `str::as_bytes` returns `&[u8]` and `<[u8]>::to_vec` an owned `Vec<u8>`.

use std::process::Command;

const PROGRAM: &str = r#"fn direct(text: string) -> Vec<u8> {
    text.as_bytes().to_vec()
}

fn bound(text: string) -> Vec<u8> {
    let v = text.as_bytes().to_vec()
    v
}

fn main() {
    println("${direct("hi").len()} ${bound("hey").len()}")
}
"#;

#[test]
fn a_byte_vector_from_as_bytes_is_returned_without_a_deref() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("main.wj"), PROGRAM).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", "main.wj", "-o", "out", "--no-cargo"])
        .current_dir(d.path())
        .output()
        .unwrap();
    let rust = std::fs::read_to_string(d.path().join("out/main.rs")).unwrap_or_else(|e| panic!("{e}\n{}", String::from_utf8_lossy(&o.stderr)));
    assert!(rust.contains("text.as_bytes().to_vec()"), "{rust}");
    assert!(!rust.contains("*text.as_bytes()"), "the tail must not be dereferenced:\n{rust}");
    assert!(!rust.contains("    *v\n"), "the bound local must not be dereferenced:\n{rust}");
}
