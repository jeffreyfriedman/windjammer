#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Player {
    hp: i64,
}









impl Player {
#[inline]
fn new(hp: i64) -> Player {
        Player { hp }
}
#[inline]
fn damage(&mut self, amount: i64) {
        self.hp -= amount;
}
#[inline]
fn heal(&mut self, amount: i64) {
        self.hp += amount;
}
}

#[inline]
fn test_mut_basic() {
    let mut x = 10_i32;
    println!("[mut_basic] before: x={}", x);
    x = 20_i32;
    println!("[mut_basic] after: x={}", x);
}

#[inline]
fn test_mut_compound() {
    let mut x = 10_i32;
    println!("[mut_compound] x={}", x);
    x += 5;
    println!("[mut_compound] after +=5: x={}", x);
    x -= 3;
    println!("[mut_compound] after -=3: x={}", x);
    x *= 2;
    println!("[mut_compound] after *=2: x={}", x);
}

#[inline]
fn sum_items(items: &Vec<i64>) -> i64 {
    let mut total = 0_i64;
    for item in items.iter().copied() {
        total += item;
    }
    total
}

#[inline]
fn test_fn_read() {
    let items = vec![1_i64, 2_i64, 3_i64];
    println!("[mut_fn_read] items before: len={}", items.len());
    let total = sum_items(&items);
    println!("[mut_fn_read] total={}", total);
    println!("[mut_fn_read] items after: len={}", items.len());
}

#[inline]
fn add_item(items: &mut Vec<i64>, item: i64) {
    items.push(item);
}

#[inline]
fn test_fn_push() {
    let mut items = vec![10_i64, 20_i64];
    println!("[mut_fn_push] items before: len={}", items.len());
    add_item(&mut items, 30_i64);
    println!("[mut_fn_push] items after: len={}", items.len());
    println!("[mut_fn_push] last item: {}", items[2]);
}

#[inline]
fn test_struct_mutation() {
    let mut player = Player::new(100_i64);
    println!("[mut_struct] before: hp={}", player.hp);
    player.damage(30_i64);
    println!("[mut_struct] after damage: hp={}", player.hp);
    player.heal(20_i64);
    println!("[mut_struct] after heal: hp={}", player.hp);
}

fn main() {
    test_mut_basic();
    test_mut_compound();
    test_fn_read();
    test_fn_push();
    test_struct_mutation();
    println!("[mut_all] PASSED");
}

