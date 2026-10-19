#[derive(Clone, Debug)]
enum Tree {
    Node(Box<Tree>, i64, Box<Tree>),
    Leaf,
}

#[inline]
fn insert(t: Tree, v: i64) -> Tree {
    match t {
        Tree::Leaf => Tree::Node(Box::new(Tree::Leaf), v, Box::new(Tree::Leaf)),
        Tree::Node(l, x, r) => {
            if v < x {
                Tree::Node(Box::new(insert(*l, v)), x, r.clone())
            } else {
                Tree::Node(l, x, Box::new(insert(*r, v)))
            }
        },
    }
}

#[inline]
fn walk(t: Tree, out: Vec<i64>) -> Vec<i64> {
    match t {
        Tree::Leaf => out,
        Tree::Node(l, x, r) => {
            let mut acc: Vec<i64> = walk(*l, out);
            acc.push(x);
            walk(*r, acc)
        },
    }
}

#[inline]
fn height(t: Tree) -> i64 {
    match t {
        Tree::Leaf => 0_i64,
        Tree::Node(l, _, r) => 1_i64 + height(*l).max((height(*r) as i32)),
    }
}

#[inline]
fn contains(t: Tree, v: i64) -> bool {
    match t {
        Tree::Leaf => false,
        Tree::Node(l, x, r) => {
            if v == x {
                true
            } else {
                if v < x {
                    contains(*l, v)
                } else {
                    contains(*r, v)
                }
            }
        },
    }
}

#[inline]
fn build() -> Tree {
    let mut t = Tree::Leaf;
    for v in [5, 3, 8, 1, 9] {
        t = insert(t, v);
    }
    t
}

fn main() {
    let strs: Vec<String> = walk(build(), vec![]).iter().map(|x| x.to_string()).collect::<Vec<String>>();
    println!("in-order: {}", strs.join(" "));
    println!("height: {}", height(build()));
    println!("contains 8: {}", contains(build(), 8_i64));
    println!("contains 4: {}", contains(build(), 4_i64));
}

