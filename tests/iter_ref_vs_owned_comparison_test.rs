#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// BUG: Comparing iteration variable (&String) with struct field (String)
//
// When iterating over a collection destructured from an enum variant match,
// the iteration variable is a &String, but the struct field being compared
// is an owned String. The comparison `o == self.value` generates
// `&String == String` which Rust doesn't implement directly.
//
// Root cause: The codegen doesn't add * dereference when comparing
// a borrowed iteration variable with an owned value.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_compared_to_struct_field() {
    // This reproduces the propertyeditor.wj bug:
    // for o in opts { if o == self.value { ... } }
    // where opts comes from enum destructuring (borrowed)
    let source = r#"
pub enum WidgetType {
    Text,
    Dropdown { options: Vec<string> },
}

pub struct Widget {
    pub value: string,
    pub widget_type: WidgetType,
}

impl Widget {
    pub fn render(self) -> string {
        match self.widget_type {
            WidgetType::Dropdown { options: opts } => {
                let mut result = ""
                for o in opts {
                    if o == self.value {
                        result.push_str("selected")
                    }
                }
                result
            },
            _ => "text",
        }
    }
}

fn main() {
    let w = Widget {
        value: "a",
        widget_type: WidgetType::Dropdown { options: vec!["a", "b", "c"] },
    }
    let r = w.render()
}
"#;

    let rust_code = test_utils::compile_single(source);

    // The generated Rust must compile. The comparison between
    // an iteration variable (possibly &String) and an owned field (String)
    // must be handled correctly by the codegen.
    assert!(
        !rust_code.contains("can't compare"),
        "Should not have comparison errors in generated code"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_for_loop_str_comparison_with_field() {
    let source = r#"
pub struct Filter {
    pub items: Vec<string>,
    pub current: string,
}

impl Filter {
    pub fn find_current(self) -> bool {
        for item in self.items {
            if item == self.current {
                return true
            }
        }
        false
    }
}

fn main() {
    let f = Filter { items: vec!["a", "b"], current: "a" }
    let found = f.find_current()
}
"#;

    test_utils::compile_single(source);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_destructured_enum_iter_comparison() {
    // Reproduces the propertyeditor.wj bug:
    // match self.widget_type {
    //     Dropdown { options: opts } => { for o in opts { if o == self.value { ... } } }
    // }
    // opts comes from match destructuring a borrowed enum variant,
    // so it's in borrowed_iterator_vars. When used as for-loop iterable,
    // is_iterating_over_borrowed must recognize it.
    let source = r#"
pub enum WidgetType {
    Text,
    Dropdown { options: Vec<string> },
}

pub struct Widget {
    pub value: string,
    pub widget_type: WidgetType,
}

impl Widget {
    pub fn find_selected(self) -> string {
        match self.widget_type {
            WidgetType::Dropdown { options: opts } => {
                let mut result = ""
                for o in opts {
                    if o == self.value {
                        result = "found"
                    }
                }
                result
            },
            _ => "",
        }
    }
}

fn main() {
    let w = Widget {
        value: "b",
        widget_type: WidgetType::Dropdown { options: vec!["a", "b", "c"] },
    }
    let r = w.find_selected()
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Verify the generated code doesn't have &String == String comparison
    assert!(
        !rust_code.contains("can't compare"),
        "Generated Rust should compile without comparison errors:\n{}",
        rust_code
    );
}
