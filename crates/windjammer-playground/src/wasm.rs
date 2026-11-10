//! The WebAssembly surface (ADR-0034): plain exports over linear memory, no imports, no bindings generator.
//! JavaScript side: docs/book/playground/wj-wasm.mjs.

use super::{run, Options};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

thread_local! {
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A buffer of `len` bytes for the host to write the source into.
#[no_mangle]
pub extern "C" fn wj_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// Release a buffer from `wj_alloc`.
///
/// # Safety
/// `ptr` and `len` must come from one `wj_alloc` call, released once.
#[no_mangle]
pub unsafe extern "C" fn wj_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

/// Run the UTF-8 source at `ptr..ptr+len`; returns the length of the result JSON at `wj_result_ptr()`.
/// Numbers arrive as JavaScript numbers (f64).
///
/// # Safety
/// `ptr..ptr+len` must be readable (a `wj_alloc` buffer the host filled).
#[no_mangle]
pub unsafe extern "C" fn wj_run(
    ptr: *const u8,
    len: usize,
    fuel: f64,
    output_bytes: f64,
    seed: f64,
) -> usize {
    std::panic::set_hook(Box::new(|info| {
        // `panic = "abort"` traps right after this hook; the host reads this result from the trapped instance's memory
        let o = super::Outcome {
            stderr: format!("internal error in the playground interpreter: {info}\n"),
            exit_code: 70,
            ..Default::default()
        };
        RESULT.with(|r| *r.borrow_mut() = o.to_json().into_bytes());
    }));
    let source = String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len)).into_owned();
    let opts = Options {
        fuel: fuel as u64,
        output_bytes: output_bytes as usize,
        seed: seed as u64,
    };
    let json = run(&source, &opts).to_json().into_bytes();
    let n = json.len();
    RESULT.with(|r| *r.borrow_mut() = json);
    n
}

/// Length of the last result JSON (also after a trap: see the panic hook in `wj_run`).
#[no_mangle]
pub extern "C" fn wj_result_len() -> usize {
    RESULT.with(|r| r.borrow().len())
}

/// Where the last result JSON is.
#[no_mangle]
pub extern "C" fn wj_result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

// The playground has no entropy source. Crates that ask the operating system for random bytes (password-hash salts)
// get a fixed stream instead, so a run stays a function of its source and seed; programs draw their own random
// numbers from the seeded world, never from here.
static STREAM: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

fn fixed_bytes(buf: &mut [u8]) {
    for chunk in buf.chunks_mut(8) {
        // splitmix64
        let mut z = STREAM
            .fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed)
            .wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        chunk.copy_from_slice(&z.to_le_bytes()[..chunk.len()]);
    }
}

fn getrandom_v02(buf: &mut [u8]) -> Result<(), getrandom02::Error> {
    fixed_bytes(buf);
    Ok(())
}
getrandom02::register_custom_getrandom!(getrandom_v02);

/// getrandom 0.3's custom backend (`--cfg getrandom_backend="custom"` in `.cargo/config.toml`).
///
/// # Safety
/// Called by getrandom with a writable `dest..dest+len`.
#[no_mangle]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom03::Error> {
    fixed_bytes(std::slice::from_raw_parts_mut(dest, len));
    Ok(())
}
