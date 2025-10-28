trait From<T> {
    fn from(value: &T) -> Self {
        panic!("not implemented");
    }
}

impl From<i64> for String {
fn from(value: i64) -> Self {
        value.to_string()
}
}

trait Converter<Input, Output> {
    fn convert(&self, input: &Input) -> Output {
        panic!("not implemented");
    }
}

struct IntToString {
    prefix: String,
}

impl Converter<i64, String> for IntToString {
fn convert(&self, input: i64) -> String {
        format!("{}: {}", self.prefix, input)
}
}

trait Into<T> {
    fn into(self) -> T {
        panic!("not implemented");
    }
}

struct MyNumber {
    value: i64,
}

impl Into<String> for MyNumber {
fn into(self) -> String {
        self.value::to_string()
}
}

fn main() {
    let s1 = String::from(42);
    println!("From<int>: {}", s1);
    let converter = IntToString { prefix: "Number" };
    let s2 = converter.convert(123);
    println!("Converter: {}", s2);
    let num = MyNumber { value: 456 };
    let s3: String = num.into();
    println!("Into<string>: {}", s3);
    println!("All generic trait implementations working!")
}

