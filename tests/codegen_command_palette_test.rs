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

//! FAILING REPRO (dogfood): CommandPalette item compose must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn command_palette_items_should_codegen() {
    let source = r##"
pub struct CommandPaletteItem {
    label: string,
    href: string,
}

impl CommandPaletteItem {
    pub fn new(label: string, href: string) -> CommandPaletteItem {
        CommandPaletteItem { label: label, href: href }
    }
}

pub struct CommandPalette {
    items: Vec<CommandPaletteItem>,
}

impl CommandPalette {
    pub fn new() -> CommandPalette {
        CommandPalette { items: Vec::new() }
    }
    pub fn item(self, item: CommandPaletteItem) -> CommandPalette {
        self.items.push(item)
        self
    }
    pub fn render(self) -> string {
        let mut body = "".to_string()
        for item in self.items {
            body = body + "<button data-href=\"" + item.href + "\">" + item.label + "</button>"
        }
        "<div class=\"wj-command-palette\">".to_string() + body + "</div>"
    }
}

fn main() {
    println!("{}", CommandPalette::new().item(CommandPaletteItem::new("Home".to_string(), "#/".to_string())).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("wj-command-palette")
        && (result.contains("CommandPalette") || result.contains("fn render"))
        && !result.contains("error[E");
    assert!(
        ok,
        "CommandPalette compose should codegen. Got:\n{}",
        result
    );
}
