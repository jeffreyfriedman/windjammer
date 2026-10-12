fn main() {
    let x = 1_i32;
    {
        let x = 2_i32;
        println!("inner {}", x)
    };
    println!("outer {}", x);
    let y = {
        let a = 5_i32;
        let b = 10_i32;
        a + b
    };
    println!("{}", y);
}

