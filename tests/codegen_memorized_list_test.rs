#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! FAILING REPRO (dogfood): MemorizedList compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn memorized_list_should_codegen() {
    let source = r##"
pub struct MemorizedItem {
    title: string,
}

impl MemorizedItem {
    pub fn new(title: string) -> MemorizedItem {
        MemorizedItem { title: title }
    }
}

pub struct MemorizedList {
    items: Vec<MemorizedItem>,
}

impl MemorizedList {
    pub fn new() -> MemorizedList {
        MemorizedList { items: Vec::new() }
    }
    pub fn item(self, item: MemorizedItem) -> MemorizedList {
        self.items.push(item)
        self
    }
    pub fn render(self) -> string {
        let mut body = "".to_string()
        for i in self.items {
            body = body + "<li class=\"wj-memorized-item\">" + i.title + "</li>"
        }
        "<div class=\"wj-memorized-list\">".to_string() + body + "</div>"
    }
}

fn main() {
    println!("{}", MemorizedList::new().item(MemorizedItem::new("Rent".to_string())).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-memorized-list")
        && result.contains("wj-memorized-item")
        && !result.contains("error[E");
    assert!(
        ok,
        "MemorizedList compose should codegen. Got:\n{}",
        result
    );
}
