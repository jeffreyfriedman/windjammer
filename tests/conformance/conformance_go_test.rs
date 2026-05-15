/// Cross-backend conformance: mutation, loops, and continue.
#[path = "cross_backend_conformance_harness.rs"]
mod cross_backend_conformance_harness;
use cross_backend_conformance_harness::assert_backends_agree;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_struct_mutation() {
    assert_backends_agree(
        "struct_mutation",
        r#"
struct Counter {
    value: int
}

impl Counter {
    fn get(self) -> int {
        self.value
    }

    fn increment(self) {
        self.value += 1
    }
}

fn main() {
    let mut c = Counter { value: 0 }
    println("{}", c.get())
    c.increment()
    println("{}", c.get())
    c.increment()
    println("{}", c.get())
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_for_sum() {
    assert_backends_agree(
        "for_sum",
        r#"
fn main() {
    let mut sum = 0
    for i in 0..10 {
        sum += i
    }
    println("{}", sum)
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_continue_while() {
    assert_backends_agree(
        "continue_while",
        r#"
fn main() {
    let mut i = 0
    while i < 6 {
        i += 1
        if i % 2 == 0 {
            continue
        }
        println("{}", i)
    }
    println("PASSED")
}
"#,
        "PASSED",
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_conformance_continue_for() {
    assert_backends_agree(
        "continue_for",
        r#"
fn main() {
    for i in 0..8 {
        if i % 3 == 0 {
            continue
        }
        println("{}", i)
    }
    println("PASSED")
}
"#,
        "PASSED",
    );
}
