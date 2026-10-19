use windjammer_runtime::ffi;
use windjammer_runtime::ffi::FfiString;
use windjammer_runtime::ffi::FfiBytes;
use windjammer_runtime::test::*;
mod ffi {
    #[inline]
fn my_extern_function(_value: i64) {
}}

extern "C" {
    pub fn my_extern_function(value: i64);
}

#[test]

pub fn test_function() {
    (unsafe { ffi::my_extern_function(42) });
}

