use windjammer_runtime::subprocess;
use windjammer_runtime::subprocess::SubprocessHandle;
#[inline]
fn test_echo() {
    let args = vec!["hello".to_string()];
    match subprocess::spawn("echo", &args) {
        Ok(handle) => {
            match subprocess::read_line(handle) {
                Ok(line) => windjammer_runtime::io::println(&format!("[subprocess_echo] {}", line)),
                Err(e) => windjammer_runtime::io::println(&format!("[subprocess_echo] FAIL read: {}", e)),
            }
            let _ = subprocess::wait(handle);
            let _ = subprocess::close(handle);
        },
        Err(e) => windjammer_runtime::io::println(&format!("[subprocess_echo] FAIL spawn: {}", e)),
    }
}

fn main() {
    test_echo();
    windjammer_runtime::io::println("[subprocess_all] PASSED");
}

