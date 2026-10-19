use windjammer_runtime::test::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Rect {
    w: i64,
    h: i64,
}










impl Rect {
#[inline]
fn area(&self) -> i64 {
        self.w * self.h
}
#[inline]
fn is_square(&self) -> bool {
        self.w == self.h
}
}

#[test]
fn area_multiplies() {
    let r = Rect { w: 3_i64, h: 4_i64 };
    assert_eq!(r.area(), 12_i64);
}

#[test]
fn square_detection() {
    assert!(Rect { w: 2_i64, h: 2_i64 }.is_square());
    assert!(!Rect { w: 2_i64, h: 3_i64 }.is_square());
}

