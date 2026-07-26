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

//! FAILING REPRO (LedgerKit R2.1): MemorizedList with seeded items must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn memorized_list_with_items_should_codegen() {
    let source = r##"
pub struct MemorizedItem {
    id: string,
    title: string,
}

impl MemorizedItem {
    pub fn new(id: string, title: string) -> MemorizedItem {
        MemorizedItem { id: id, title: title }
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
            body = body + "<li data-wj-memorized-id=\"" + i.id + "\">" + i.title + "</li>"
        }
        "<div class=\"wj-memorized-list\" data-wj-memorized-list>".to_string() + body + "</div>"
    }
}

fn main() {
    println!("{}", MemorizedList::new()
        .item(MemorizedItem::new("mem-rent".to_string(), "Office rent".to_string()))
        .render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-memorized-list")
        && result.contains("data-wj-memorized-id")
        && result.contains("Office rent")
        && !result.contains("error[E");
    assert!(
        ok,
        "MemorizedList with items should codegen. Got:\n{}",
        result
    );
}
