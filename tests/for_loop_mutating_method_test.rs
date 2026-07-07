#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Bug: `for s in self.sections` generates `for s in &self.sections`, giving
/// `&Section` references. But `s.render()` takes `&mut self`, so the borrow
/// is insufficient. The compiler should emit `&mut self.sections` when the
/// loop body calls a method requiring `&mut self` on the iteration variable.
#[test]
fn test_for_loop_calls_mut_method_on_element() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "section.wj",
        r##"
pub struct Section {
    title: string,
    content: string,
}

impl Section {
    pub fn new(title: string) -> Section {
        Section {
            title: title,
            content: "".to_string(),
        }
    }

    pub fn render(self) -> string {
        let mut html = String::new()
        html.push_str("<section><h2>")
        html.push_str(self.title)
        html.push_str("</h2><p>")
        html.push_str(self.content)
        html.push_str("</p></section>")
        html
    }
}

pub struct SectionGroup {
    sections: Vec<Section>,
}

impl SectionGroup {
    pub fn new() -> SectionGroup {
        SectionGroup {
            sections: Vec::new(),
        }
    }

    pub fn render(self) -> string {
        let mut result = String::new()
        for s in self.sections {
            result = result + s.render() + "\n"
        }
        result
    }
}
"##,
    );

    // The generated for-loop iteration must be compatible with s.render()
    t.assert_compiles_without_error();
}
