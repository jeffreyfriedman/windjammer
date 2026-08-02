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

//! Gate (dogfood): AccountRail with balances must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn account_rail_should_codegen() {
    let source = r##"
pub struct AccountRailItem {
    code: string,
    label: string,
    balance_html: string,
}

impl AccountRailItem {
    pub fn new(code: string, label: string, balance_html: string) -> AccountRailItem {
        AccountRailItem { code: code, label: label, balance_html: balance_html }
    }
}

pub struct AccountRail {
    items: Vec<AccountRailItem>,
}

impl AccountRail {
    pub fn new() -> AccountRail {
        AccountRail { items: Vec::new() }
    }
    pub fn item(self, item: AccountRailItem) -> AccountRail {
        self.items.push(item)
        self
    }
    pub fn render(self) -> string {
        let mut body = "".to_string()
        for i in self.items {
            body = body + "<li class=\"wj-account-rail-item\" data-code=\"" + i.code + "\">"
                + i.label + "<span class=\"wj-account-rail-bal\">" + i.balance_html + "</span></li>"
        }
        "<ul class=\"wj-account-rail\" data-wj-account-rail>".to_string() + body + "</ul>"
    }
}

fn main() {
    println!("{}", AccountRail::new()
        .item(AccountRailItem::new("1000".to_string(), "Cash".to_string(), "$9,802.00".to_string()))
        .render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-account-rail")
        && result.contains("wj-account-rail-item")
        && result.contains("wj-account-rail-bal")
        && result.contains("data-code")
        && !result.contains("error[E");
    assert!(
        ok,
        "AccountRail compose should codegen. Got:\n{}",
        result
    );
}
