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

//! Gate (dogfood): ShellNav with active link must codegen.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn shell_nav_active_link_should_codegen() {
    let source = r##"
pub struct ShellNavLink {
    label: string,
    href: string,
    active: bool,
}

impl ShellNavLink {
    pub fn new(label: string, href: string) -> ShellNavLink {
        ShellNavLink { label: label, href: href, active: false }
    }
    pub fn active(self, active: bool) -> ShellNavLink {
        self.active = active
        self
    }
}

pub struct ShellNav {
    links: Vec<ShellNavLink>,
}

impl ShellNav {
    pub fn new() -> ShellNav {
        ShellNav { links: Vec::new() }
    }
    pub fn link(self, link: ShellNavLink) -> ShellNav {
        self.links.push(link)
        self
    }
    pub fn render(self) -> string {
        let mut body = "".to_string()
        for link in self.links {
            let cls = if link.active { " is-active" } else { "" }
            body = body + "<a href=\"" + link.href + "\" class=\"" + cls + "\">" + link.label + "</a>"
        }
        "<nav id=\"shellNav\">".to_string() + body + "</nav>"
    }
}

fn main() {
    println!("{}", ShellNav::new().link(ShellNavLink::new("Home".to_string(), "#/".to_string()).active(true)).render())
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("shellNav")
        && (result.contains("is-active") || result.contains("active"))
        && !result.contains("error[E");
    assert!(
        ok,
        "ShellNav active link compose should codegen. Got:\n{}",
        result
    );
}
