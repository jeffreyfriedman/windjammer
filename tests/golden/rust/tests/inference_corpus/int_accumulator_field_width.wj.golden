#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Line {
    pub amount: i64,
}
impl Line {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.amount.to_ne_bytes());
        __bytes
    }
}


#[inline]
pub fn shift_by_total(lines: &Vec<Line>) -> Vec<Line> {
    let mut total = 0_i64;
    for line in lines {
        total += line.amount;
    }
    let mut out: Vec<Line> = vec![];
    for line in lines.iter().copied() {
        out.push(Line { amount: line.amount - total });
    }
    out
}

fn main() {
    let out: Vec<Line> = shift_by_total(&vec![Line { amount: 5000000000_i64 }, Line { amount: 1_i64 }]);
    println!("{}", out[0].amount);
    println!("{}", out[1].amount);
}

