use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, MarkupContent,
    MarkupKind, Position,
};
use windjammer::parser::{Item, Program};

/// Completion provider for Windjammer code
pub struct CompletionProvider {
    program: Option<Program>,
}

impl CompletionProvider {
    pub fn new() -> Self {
        Self { program: None }
    }

    /// Update the parsed program
    pub fn update_program(&mut self, program: Program) {
        self.program = Some(program);
    }

    /// Get completion items at a position
    pub fn get_completions(&self, _position: Position) -> Option<CompletionResponse> {
        let mut items = Vec::new();

        // Add keywords
        items.extend(self.keyword_completions());

        // Add decorators (UI framework)
        items.extend(self.decorator_completions());

        // Add stdlib modules
        items.extend(self.stdlib_completions());

        // Add UI framework types
        items.extend(self.ui_framework_completions());

        // Add user-defined items from the program
        if let Some(program) = &self.program {
            items.extend(self.program_completions(program));
        }

        Some(CompletionResponse::Array(items))
    }

    /// Windjammer language keywords
    fn keyword_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "fn".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("function declaration".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nfn name(params) { }\n```".to_string(),
                })),
                insert_text: Some("fn $1($2) {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "let".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("variable declaration".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nlet x = value\n```".to_string(),
                })),
                insert_text: Some("let $1 = $0".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "mut".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("mutable variable".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nlet mut x = value\n```".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "struct".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("struct declaration".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nstruct Name {\n\tfield: Type\n}\n```".to_string(),
                })),
                insert_text: Some("struct $1 {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "enum".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("enum declaration".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nenum Name {\n\tVariant1,\n\tVariant2\n}\n```"
                        .to_string(),
                })),
                insert_text: Some("enum $1 {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "trait".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("trait declaration".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\ntrait Name {\n\tfn method(&self)\n}\n```".to_string(),
                })),
                insert_text: Some("trait $1 {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "impl".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("implementation block".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nimpl Type {\n\tfn method(&self) { }\n}\n```".to_string(),
                })),
                insert_text: Some("impl $1 {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "match".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("pattern matching".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nmatch value {\n\tpattern => expr\n}\n```".to_string(),
                })),
                insert_text: Some("match $1 {\n\t$2 => $0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "if".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("conditional".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nif condition {\n\t// code\n}\n```".to_string(),
                })),
                insert_text: Some("if $1 {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "else".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("else branch".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "for".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("for loop".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nfor item in collection {\n\t// code\n}\n```".to_string(),
                })),
                insert_text: Some("for $1 in $2 {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "while".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("while loop".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nwhile condition {\n\t// code\n}\n```".to_string(),
                })),
                insert_text: Some("while $1 {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "loop".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("infinite loop".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nloop {\n\t// code\n}\n```".to_string(),
                })),
                insert_text: Some("loop {\n\t$0\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "return".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("return statement".to_string()),
                insert_text: Some("return $0".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "break".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("break from loop".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "continue".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("continue to next iteration".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "use".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("import statement".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nuse std.module\n```".to_string(),
                })),
                insert_text: Some("use $0".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "pub".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("public visibility".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "const".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("constant declaration".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nconst NAME: Type = value\n```".to_string(),
                })),
                insert_text: Some("const $1: $2 = $0".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "static".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("static variable".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nstatic NAME: Type = value\n```".to_string(),
                })),
                insert_text: Some("static $1: $2 = $0".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "true".to_string(),
                kind: Some(CompletionItemKind::VALUE),
                detail: Some("boolean true".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "false".to_string(),
                kind: Some(CompletionItemKind::VALUE),
                detail: Some("boolean false".to_string()),
                ..Default::default()
            },
        ]
    }

    /// UI framework decorator completions
    fn decorator_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "@component".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("UI component decorator".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Marks a struct as a UI component. Automatically generates `new()` and `with_state()` methods.\n\n```windjammer\n@component\nstruct Counter {\n    count: int\n}\n```".to_string(),
                })),
                insert_text: Some("@component\n".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "@game".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Game entity decorator".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Marks a struct as a game entity with ECS support.\n\n```windjammer\n@game\nstruct Player {\n    position: Vec2\n    health: int\n}\n```".to_string(),
                })),
                insert_text: Some("@game\n".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "@derive".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Derive trait implementations".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Automatically implement common traits.\n\n```windjammer\n@derive(Debug, Clone, PartialEq)\nstruct Point { x: int, y: int }\n```".to_string(),
                })),
                insert_text: Some("@derive($0)".to_string()),
            ..Default::default()
            },
        ]
    }

    /// UI framework type and module completions
    fn ui_framework_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "windjammer_ui.prelude.*".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("UI framework prelude (all common types)".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Import all commonly used UI framework types and traits.\n\n```windjammer\nuse windjammer_ui.prelude.*\n```".to_string(),
                })),
                insert_text: Some("use windjammer_ui.prelude.*".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "windjammer_ui.vdom".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Virtual DOM types".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Virtual DOM types: VElement, VNode, VText\n\n```windjammer\nuse windjammer_ui.vdom.{VElement, VNode}\n```".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "windjammer_ui.game".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Game framework types".to_string(),) ,
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Game development types: Vec2, Vec3, GameLoop, Input, RenderContext\n\n```windjammer\nuse windjammer_ui.game.*\n```".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "VElement".to_string(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("Virtual DOM element".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Create virtual DOM elements.\n\n```windjammer\nVElement::new(\"div\")\n    .attr(\"class\", \"container\")\n    .child(VNode::Text(VText::new(\"Hello\")))\n```".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "VNode".to_string(),
                kind: Some(CompletionItemKind::ENUM),
                detail: Some("Virtual DOM node".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Represents a node in the virtual DOM tree.\n\nVariants: Element, Text, Component, Empty".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "Vec2".to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("2D vector for game development".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "2D vector with x and y components.\n\n```windjammer\nlet pos = Vec2 { x: 10.0, y: 20.0 }\n```".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "Vec3".to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("3D vector for game development".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "3D vector with x, y, and z components.\n\n```windjammer\nlet pos = Vec3 { x: 1.0, y: 2.0, z: 3.0 }\n```".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "GameLoop".to_string(),
                kind: Some(CompletionItemKind::INTERFACE),
                detail: Some("Game loop trait".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Implement this trait for game update and render logic.\n\n```windjammer\nimpl GameLoop for MyGame {\n    fn update(delta: f32) { }\n    fn render(ctx: RenderContext) { }\n}\n```".to_string(),
                })),
                ..Default::default()
            },
        ]
    }

    /// Windjammer standard library completions
    fn stdlib_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "std.http".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("HTTP client and server".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "HTTP module for making requests and creating servers".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.json".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("JSON parsing and serialization".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Parse and stringify JSON data".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.fs".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("File system operations".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Read, write, and manipulate files and directories".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.collections".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Data structures".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "HashMap, HashSet, BTreeMap, BTreeSet, VecDeque".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.time".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Time and date utilities".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Work with timestamps, dates, and durations".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.crypto".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Cryptographic operations".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Hashing (SHA256), password hashing (bcrypt), base64 encoding"
                        .to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.db".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Database access".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "PostgreSQL, MySQL, SQLite database access".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.env".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Environment variables".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Get and set environment variables".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.process".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Process execution".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Run external processes and commands".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.random".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Random number generation".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Generate random numbers, booleans, and shuffle collections".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.log".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Logging utilities".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Structured logging with multiple levels".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.regex".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Regular expressions".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Pattern matching with regular expressions".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.cli".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("CLI argument parsing".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Parse command-line arguments".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "std.async".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Async utilities".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Async/await utilities like sleep".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "println!".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Print to stdout with newline".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nprintln!(\"Hello, {}!\", name)\n```".to_string(),
                })),
                insert_text: Some("println!(\"$1\")".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "print!".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Print to stdout without newline".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nprint!(\"Hello, {}!\", name)\n```".to_string(),
                })),
                insert_text: Some("print!(\"$1\")".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "format!".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Format string".to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "```windjammer\nlet s = format!(\"Hello, {}!\", name)\n```".to_string(),
                })),
                insert_text: Some("format!(\"$1\")".to_string()),
                ..Default::default()
            },
        ]
    }

    /// Completions from the parsed program
    fn program_completions(&self, program: &Program) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        for item in &program.items {
            match item {
                Item::Function { decl: func, location: _ } => {
                    items.push(CompletionItem {
                        label: func.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("fn {}", func.name)),
                        documentation: Some(Documentation::String(
                            "Function defined in this file".to_string(),
                        )),
                        ..Default::default()
                    });
                }
                Item::Struct { decl: struct_decl, location: _ } => {
                    items.push(CompletionItem {
                        label: struct_decl.name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        detail: Some(format!("struct {}", struct_decl.name)),
                        documentation: Some(Documentation::String(
                            "Struct defined in this file".to_string(),
                        )),
                        ..Default::default()
                    });
                }
                Item::Enum { decl: enum_decl, location: _ } => {
                    items.push(CompletionItem {
                        label: enum_decl.name.clone(),
                        kind: Some(CompletionItemKind::ENUM),
                        detail: Some(format!("enum {}", enum_decl.name)),
                        documentation: Some(Documentation::String(
                            "Enum defined in this file".to_string(),
                        )),
                        ..Default::default()
                    });

                    // Add enum variants
                    for variant in &enum_decl.variants {
                        items.push(CompletionItem {
                            label: format!("{}::{}", enum_decl.name, variant.name),
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            detail: Some("enum variant".to_string()),
                            documentation: Some(Documentation::String(format!(
                                "Variant of enum {}",
                                enum_decl.name
                            ))),
                            ..Default::default()
                        });
                    }
                }
                Item::Trait { decl: trait_decl, location: _ } => {
                    items.push(CompletionItem {
                        label: trait_decl.name.clone(),
                        kind: Some(CompletionItemKind::INTERFACE),
                        detail: Some(format!("trait {}", trait_decl.name)),
                        documentation: Some(Documentation::String(
                            "Trait defined in this file".to_string(),
                        )),
                        ..Default::default()
                    });
                }
                _ => {}
            }
        }

        items
    }
}

impl Default for CompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}
