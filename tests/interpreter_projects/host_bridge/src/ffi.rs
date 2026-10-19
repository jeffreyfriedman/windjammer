// Hand-written Rust linked by the compiled build and by the interpreter's host bridge (ADR-0020).
use windjammer_runtime::ffi::FfiString;

#[no_mangle]
pub extern "C" fn host_add(a: i64, b: i64) -> i64 {
    a + b
}

#[no_mangle]
pub extern "C" fn host_greet(name: FfiString) -> FfiString {
    let s = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(name.ptr, name.len)) };
    FfiString::from_string(format!("hello, {} ({} bytes)", s, s.len()))
}

#[no_mangle]
pub extern "C" fn host_scale(x: f64, on: bool) -> f64 {
    if on { x * 2.5 } else { x }
}
