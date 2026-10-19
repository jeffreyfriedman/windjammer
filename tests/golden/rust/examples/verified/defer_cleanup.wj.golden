fn main() {
    println!("start");
    let _wj_defer_9005 = {
        struct WjDefer<F: FnMut()>(F);
        impl<F: FnMut()> Drop for WjDefer<F> {
            fn drop(&mut self) {
                (self.0)()
            }
        }
        WjDefer(|| {
            println!("cleanup");
        })
    };
    println!("work");
}

