use windjammer_runtime::test::*;
extern "C" {
    pub fn printf(format: windjammer_runtime::ffi::FfiString);
    pub fn malloc(size: i64) -> i64;
    pub fn free(ptr: i64);
    pub fn memcpy(dest: i64, src: i64, n: i64) -> i64;
}

#[test]

pub fn test_extern() {
    (unsafe { printf(windjammer_runtime::ffi::string_to_ffi("Hello from Windjammer!".to_string())) });
}

