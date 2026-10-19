use windjammer_runtime::test::*;
extern "C" {
    pub fn test_wjsl_let_mut_simple();
    pub fn test_wjsl_let_mut_with_usage();
    pub fn test_wjsl_let_mut_in_loop();
}

#[test]

pub fn test_all() {
    (unsafe { test_wjsl_let_mut_simple() });
    (unsafe { test_wjsl_let_mut_with_usage() });
    (unsafe { test_wjsl_let_mut_in_loop() });
    println!("✅ All WJSL let mut tests passed");
}

