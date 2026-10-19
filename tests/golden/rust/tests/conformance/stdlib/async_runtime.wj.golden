use windjammer_runtime::async_runtime::*;
#[inline]
fn test_sleep_ms_blocking() {
    async_runtime::sleep_ms_blocking(10_u64);
    windjammer_runtime::io::println("[async_sleep] ok");
}

fn main() {
    test_sleep_ms_blocking();
    windjammer_runtime::io::println("[async_all] PASSED");
}

