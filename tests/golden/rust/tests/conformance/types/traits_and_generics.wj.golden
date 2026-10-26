pub trait Describe {
    fn describe(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Circle {
    radius: i64,
}
impl Circle {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.radius.to_ne_bytes());
        __bytes
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Square {
    side: i64,
}
impl Square {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.side.to_ne_bytes());
        __bytes
    }
}


impl Describe for Circle {
#[inline]
fn describe(&self) -> String {
        format!("Circle(r={})", self.radius)
}
}

impl Describe for Square {
#[inline]
fn describe(&self) -> String {
        format!("Square(s={})", self.side)
}
}

pub trait HasArea {
    fn area(&self) -> i64;
}

impl HasArea for Circle {
#[inline]
fn area(&self) -> i64 {
        3_i64 * self.radius * self.radius
}
}

impl HasArea for Square {
#[inline]
fn area(&self) -> i64 {
        self.side * self.side
}
}

pub trait Named {
    fn name(&self) -> String;
}

pub trait Aged {
    fn age(&self) -> i64;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Person {
    person_name: String,
    person_age: i64,
}

impl Named for Person {
#[inline]
fn name(&self) -> String {
        self.person_name.clone()
}
}

impl Aged for Person {
#[inline]
fn age(&self) -> i64 {
        self.person_age
}
}

pub trait Animal {
    fn speak(&self) -> String;
    fn is_loud(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Dog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Cat;

impl Animal for Dog {
#[inline]
fn speak(&self) -> String {
        String::from("Woof!")
}
#[inline]
fn is_loud(&self) -> bool {
        true
}
}

impl Animal for Cat {
#[inline]
fn speak(&self) -> String {
        String::from("Meow!")
}
}

#[derive(Debug, Clone)]

struct Pair<A, B> {
    first: A,
    second: B,
}

impl<A, B> Pair<A, B> {
#[inline]
fn new(first: A, second: B) -> Pair<A, B> {
        Pair { first, second }
}
}

#[inline]
fn test_basic_trait() {
    let c = Circle { radius: 5_i64 };
    let s = Square { side: 4_i64 };
    println!("[trait_basic] circle: {}", c.describe());
    println!("[trait_basic] square: {}", s.describe());
}

#[inline]
fn test_trait_area() {
    let c = Circle { radius: 5_i64 };
    let s = Square { side: 4_i64 };
    println!("[trait_area] circle area: {}", c.area());
    println!("[trait_area] square area: {}", s.area());
}

#[inline]
fn test_multiple_traits() {
    let alice = Person { person_name: String::from("Alice"), person_age: 30_i64 };
    let bob = Person { person_name: String::from("Bob"), person_age: 25_i64 };
    println!("[trait_multi] {}: age={}", alice.name(), alice.age());
    println!("[trait_multi] {}: age={}", bob.name(), bob.age());
    let diff = alice.age() - bob.age();
    println!("[trait_multi] compare: {}", diff);
}

#[inline]
fn test_default_impl() {
    let dog = Dog {  };
    let cat = Cat {  };
    println!("[trait_default] dog says: {}", dog.speak());
    println!("[trait_default] cat says: {}", cat.speak());
    println!("[trait_default] dog is_loud: {}", dog.is_loud());
    println!("[trait_default] cat is_loud: {}", cat.is_loud());
}

#[inline]
fn identity<T>(x: T) -> T {
    x
}

#[inline]
fn test_generic_identity() {
    let a = identity(42_i64);
    let b = identity(String::from("hello"));
    println!("[generic_identity] int: {}", a);
    println!("[generic_identity] string: {}", b);
}

#[inline]
fn test_generic_pair() {
    let p1 = Pair::new(1_i32, String::from("two"));
    let p2 = Pair::new(true, 42_i32);
    println!("[generic_pair] first={}, second={}", p1.first, p1.second);
    println!("[generic_pair] first={}, second={}", p2.first, p2.second);
}

fn main() {
    test_basic_trait();
    test_trait_area();
    test_multiple_traits();
    test_default_impl();
    test_generic_identity();
    test_generic_pair();
    println!("[trait_all] PASSED");
}

