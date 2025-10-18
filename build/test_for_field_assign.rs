struct Item {
    completed: bool,
}

fn main() {
    let items = vec![Item { completed: false }];
    for item in items {
        item.completed = true;
    }
}

