
fn risky(n: i64) -> i64 {
    println!("open");
    let _wj_defer_10005 = {
        struct WjDefer<F: FnMut()>(F);
        impl<F: FnMut()> Drop for WjDefer<F> {
            fn drop(&mut self) {
                (self.0)()
            }
        }
        WjDefer(|| {
            println!("first-cleanup");
        })
    };
    let _wj_defer_11005 = {
        struct WjDefer<F: FnMut()>(F);
        impl<F: FnMut()> Drop for WjDefer<F> {
            fn drop(&mut self) {
                (self.0)()
            }
        }
        WjDefer(|| {
            println!("second-cleanup");
        })
    };
    if n > 0_i64 {
        let _ = panic!("boom");
    }
    n
}

fn main() {
    println!("{}", risky(1_i64));
}

