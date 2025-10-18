fn main() {
    match foo() {
        Ok(x) => {
            bar().with_header("A", "B");
        },
        Err(e) => {
            baz().with_header("C", "D");
        },
    }
}

