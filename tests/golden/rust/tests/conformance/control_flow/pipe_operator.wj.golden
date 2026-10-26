#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Counter {
    n: i64,
}









impl Counter {
#[inline]
fn new(n: i64) -> Counter {
        Counter { n }
}
#[inline]
fn add(self, k: i64) -> Counter {
        Counter { n: self.n + k }
}
#[inline]
fn double(self) -> Counter {
        Counter { n: self.n * 2_i64 }
}
#[inline]
fn value(&self) -> i64 {
        self.n
}
}

#[inline]
fn double(x: i64) -> i64 {
    x * 2_i64
}

#[inline]
fn add(x: i64, y: i64) -> i64 {
    x + y
}

#[inline]
fn sub(x: i64, y: i64) -> i64 {
    x - y
}

#[inline]
fn square(x: i64) -> i64 {
    x * x
}

#[inline]
fn evens(v: &Vec<i64>) -> Vec<i64> {
    let mut out: Vec<i64> = vec![];
    for n in v.iter().copied() {
        if n % 2_i64 == 0_i64 {
            let _ = out.push(n);
        }
    }
    out
}

#[inline]
fn scale(v: &Vec<i64>, k: i64) -> Vec<i64> {
    let mut out: Vec<i64> = vec![];
    for n in v.iter().copied() {
        out.push(n * k);
    }
    out
}

#[inline]
fn total(v: &Vec<i64>) -> i64 {
    let mut sum = 0_i64;
    for n in v.iter().copied() {
        sum += n;
    }
    sum
}

#[inline]
fn quarter(n: i64) -> Result<i64, String> {
    let r = half(half(n)?)?;
    Ok(r)
}

fn main() {
    show_num(1_i64);
    show_num(5_i64);
    show_num(-2_i64);
    show_prec(3_i64);
    show_prec(0_i64);
    show_prec(10_i64);
    show_hole(1_i64);
    show_hole(50_i64);
    show_hole(200_i64);
    show_str("bob");
    show_str("Al");
    show_str("");
    show_interp("x");
    show_interp("yz");
    show_vec("1 2 3 4 5 6", vec![1_i64, 2_i64, 3_i64, 4_i64, 5_i64, 6_i64]);
    show_vec("7 9", vec![7_i64, 9_i64]);
    show_vec("10", vec![10_i64]);
    show_method(4_i64);
    show_method(0_i64);
    show_method(-3_i64);
    show_closure(4_i64);
    show_closure(0_i64);
    show_closure(-5_i64);
    show_try(8_i64);
    show_try(12_i64);
    show_try(6_i64);
    show_try(7_i64);
    println!("[pipe_all] PASSED");
}

#[inline]
fn greet(name: &str) -> String {
    format!("hello {}", name)
}

#[inline]
fn shout(s: &str) -> String {
    format!("{}!", s)
}

#[inline]
fn wrap(s: &str, l: &str, r: &str) -> String {
    format!("{}{}{}", l, s, r)
}

#[inline]
fn half(n: i64) -> Result<i64, String> {
    if n % 2_i64 != 0_i64 {
        { let _temp0 = format!("odd {}", n); Err(_temp0) }
    } else {
        Ok(n / 2_i64)
    }
}

#[inline]
fn show_num(x: i64) {
    let r = square(sub(add(double(x), 3_i64), 1_i64));
    println!("[pipe_num] {} -> {}", x, r);
}

#[inline]
fn show_prec(x: i64) {
    let r = double(x + 1_i64);
    println!("[pipe_prec] {} -> {}", x, r);
}

#[inline]
fn show_hole(x: i64) {
    let r = sub(100_i64, x);
    println!("[pipe_hole] {} -> {}", x, r);
}

#[inline]
fn show_str(name: &str) {
    let r = shout(&greet(name));
    let r2 = wrap(&r, "[", "]");
    println!("[pipe_str] {} -> {}", name, r2);
}

#[inline]
fn show_interp(name: &str) {
    let r = { let _temp0 = format!("{}:", name); wrap(name, &_temp0, "|") };
    println!("[pipe_interp] {} -> {}", name, r);
}

#[inline]
fn show_vec(label: &str, v: Vec<i64>) {
    let a = total(&evens(&v));
    let b = total(&scale(&evens(&v), 3_i64));
    println!("[pipe_vec] {} -> {} | {}", label, a, b);
}

#[inline]
fn show_method(x: i64) {
    let r = Counter::new(x).add(2_i64).double().add(-2_i64).value();
    println!("[pipe_method] {} -> {}", x, r);
}

#[inline]
fn show_closure(x: i64) {
    let r = add((|v| v * v)(x), 1_i64);
    println!("[pipe_closure] {} -> {}", x, r);
}

#[inline]
fn show_try(x: i64) {
    match quarter(x) {
        Ok(v) => println!("[pipe_try] {} -> ok {}", x, v),
        Err(e) => println!("[pipe_try] {} -> err {}", x, e),
    }
}

