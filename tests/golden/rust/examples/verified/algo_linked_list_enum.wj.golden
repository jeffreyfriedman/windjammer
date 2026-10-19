#[derive(Clone, Debug)]
enum List {
    Cons(i64, Box<List>),
    Nil,
}

#[inline]
fn show(l: List) -> String {
    match l {
        List::Cons(v, rest) => format!("{} -> {}", v, show(*rest)),
        List::Nil => String::from("nil"),
    }
}

#[inline]
fn sum(l: List) -> i64 {
    match l {
        List::Cons(v, rest) => v + sum(*rest),
        List::Nil => 0_i64,
    }
}

#[inline]
fn len(l: List) -> i64 {
    match l {
        List::Cons(_, rest) => 1_i64 + len(*rest),
        List::Nil => 0_i64,
    }
}

#[inline]
fn build() -> List {
    List::Cons(1_i64, Box::new(List::Cons(2_i64, Box::new(List::Cons(3_i64, Box::new(List::Nil))))))
}

fn main() {
    println!("{}", show(build()));
    println!("sum = {}", sum(build()));
    println!("len = {}", len(build()));
}

