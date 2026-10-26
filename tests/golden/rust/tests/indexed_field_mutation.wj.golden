#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Item {
    value: i32,
}









impl Item {
#[inline]
pub fn increment(&mut self) {
        self.value += 1;
}
#[inline]
pub fn set_value(&mut self, v: i32) {
        self.value = v;
}
#[inline]
pub fn get_value(&self) -> i32 {
        self.value
}
}

#[derive(Debug, Clone, PartialEq, Default)]

struct Container {
    items: Vec<Item>,
    count: usize,
}

impl Container {
#[inline]
pub fn new() -> Container {
        Container { items: Vec::new(), count: 0_usize }
}
#[inline]
pub fn update_item(&mut self, index: usize) {
        if index < self.items.len() {
            let _ = self.items[index].increment();
        }
}
#[inline]
pub fn set_item_value(&mut self, index: usize, value: i32) {
        if index < self.items.len() {
            self.items[index].value = value;
        }
}
#[inline]
pub fn set_item(&mut self, index: usize, value: i32) {
        if index < self.items.len() {
            let _ = self.items[index].set_value(value);
        }
}
#[inline]
pub fn get_item_value(&self, index: usize) -> i32 {
        if index < self.items.len() {
            return self.items[index].get_value();
        }
        0_i32
}
#[inline]
pub fn item_count(&self) -> usize {
        self.items.len()
}
}

fn main() {
    let mut container = Container::new();
    container.items.push(Item { value: 10_i32 });
    container.items.push(Item { value: 20_i32 });
    container.items.push(Item { value: 30_i32 });
    container.update_item(0_usize);
    container.set_item_value(1_usize, 25_i32);
    container.set_item(2_usize, 35_i32);
    let v0 = container.get_item_value(0_usize);
    let v1 = container.get_item_value(1_usize);
    let v2 = container.get_item_value(2_usize);
    println!("Item 0: {}", v0);
    println!("Item 1: {}", v1);
    println!("Item 2: {}", v2);
    println!("✅ Indexed field mutations work!");
}

