//! FFI-safe string types for extern "C" function boundaries.
//!
//! Rust `String` and `&str` are NOT FFI-safe (they have unspecified layout).
//! Use `FfiString` for passing strings across extern "C" boundaries.

/// C-compatible string representation: pointer + length.
/// Caller owns the allocation; use `ffi_free_string` to free.
#[repr(C)]
pub struct FfiString {
    pub ptr: *const u8,
    pub len: usize,
}

impl FfiString {
    /// Create from Rust string. Caller must call `ffi_free_string` on the result.
    #[inline]
    pub fn from_string(s: String) -> Self {
        let bytes = s.into_bytes();
        let len = bytes.len();
        let ptr = Box::into_raw(bytes.into_boxed_slice()) as *const u8;
        Self { ptr, len }
    }

    /// Create from &str (copies). Caller must call `ffi_free_string` on the result.
    #[inline]
    pub fn from_str(s: &str) -> Self {
        Self::from_string(s.to_string())
    }

    /// Convert to Rust String. Consumes the FfiString (memory is freed).
    #[inline]
    pub fn to_string(self) -> String {
        ffi_to_string(self)
    }

    /// Create empty FfiString (no allocation to free).
    #[inline]
    pub fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }
}

/// Convert Rust String to FFI-safe FfiString.
/// Caller must call `ffi_free_string` on the result when done.
#[inline]
pub fn string_to_ffi(s: String) -> FfiString {
    FfiString::from_string(s)
}

/// Convert FfiString to Rust String.
/// The FfiString is consumed (memory is freed as part of this call).
#[inline]
pub fn ffi_to_string(ffi: FfiString) -> String {
    if ffi.ptr.is_null() || ffi.len == 0 {
        return String::new();
    }
    let bytes = unsafe { Vec::from_raw_parts(ffi.ptr as *mut u8, ffi.len, ffi.len) };
    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// Free an FfiString returned from an extern function.
/// Safe to call with null ptr or empty FfiString.
#[no_mangle]
pub extern "C" fn ffi_free_string(ffi: FfiString) {
    if !ffi.ptr.is_null() && ffi.len > 0 {
        let _ = unsafe { Vec::from_raw_parts(ffi.ptr as *mut u8, ffi.len, ffi.len) };
    }
}

// =============================================================================
// FfiBytes - FFI-safe Vec<u8> for extern "C" boundaries
// =============================================================================

/// C-compatible byte buffer: pointer + length + capacity.
/// Caller owns the allocation; use `ffi_free_bytes` to free.
#[repr(C)]
pub struct FfiBytes {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

/// Convert Rust Vec<u8> to FFI-safe FfiBytes.
/// Caller must call `ffi_free_bytes` on the result when done.
#[inline]
pub fn vec_to_ffi(mut v: Vec<u8>) -> FfiBytes {
    let ptr = v.as_mut_ptr();
    let len = v.len();
    let cap = v.capacity();
    std::mem::forget(v);
    FfiBytes { ptr, len, cap }
}

/// Convert FfiBytes to Rust Vec<u8>.
/// The FfiBytes is consumed (memory is freed as part of this call).
#[inline]
pub fn ffi_to_vec(ffi: FfiBytes) -> Vec<u8> {
    if ffi.ptr.is_null() || ffi.len == 0 {
        return Vec::new();
    }
    unsafe { Vec::from_raw_parts(ffi.ptr, ffi.len, ffi.cap) }
}

/// Free an FfiBytes returned from an extern function.
/// Safe to call with null ptr or empty FfiBytes.
#[no_mangle]
pub extern "C" fn ffi_free_bytes(ffi: FfiBytes) {
    if !ffi.ptr.is_null() && (ffi.len > 0 || ffi.cap > 0) {
        let _ = unsafe { Vec::from_raw_parts(ffi.ptr, ffi.len, ffi.cap) };
    }
}
