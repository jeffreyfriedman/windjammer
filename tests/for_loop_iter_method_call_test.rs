#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Bug: when iterating over self.items and calling a method on the loop
/// variable that takes owned self, the codegen must handle the borrow.
/// The for loop generates `for s in &self.items` which yields &T refs,
/// but calling a consuming method on &T fails with E0596 or E0507.
#[test]
fn test_for_loop_var_clone_when_method_needs_owned() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "section.wj",
        r#"
pub struct Section {
    title: string,
}

impl Section {
    pub fn new(title: string) -> Section {
        Section { title: title }
    }

    pub fn render(self) -> string {
        format!("<section>{}</section>", self.title)
    }
}
"#,
    );
    t.add_file(
        "group.wj",
        r#"
use section::Section

pub struct SectionGroup {
    sections: Vec<Section>,
}

impl SectionGroup {
    pub fn new() -> SectionGroup {
        SectionGroup { sections: Vec::new() }
    }

    pub fn render(self) -> string {
        let mut html = String::new()
        for s in self.sections {
            html = html + s.render() + "\n"
        }
        html
    }
}
"#,
    );

    // Should compile without error — codegen handles iterator clone
    let _map = t.compile().expect("should compile successfully");
}
