//! Parser for `.wj` component files
//!
//! Supports two syntax styles:
//! 1. Minimal: Top-level declarations
//! 2. Advanced: Struct-based with @component

use super::ast::*;
use crate::lexer::{Lexer, Token};
use crate::parser::{Expression, FunctionDecl, Item, Parser as WindjammerParser, Program, Type};
use anyhow::{bail, Result};

pub struct ComponentParser {
    tokens: Vec<crate::lexer::TokenWithLocation>,
    pos: usize,
}

impl ComponentParser {
    pub fn new(tokens: Vec<crate::lexer::TokenWithLocation>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parse a component file
    pub fn parse(source: &str) -> Result<ComponentFile> {
        // Lex the source
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();

        let mut parser = Self::new(tokens);
        parser.parse_component()
    }

    fn current_token(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn peek(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        if self.current_token() == &expected {
            self.advance();
            Ok(())
        } else {
            bail!("Expected {:?}, found {:?}", expected, self.current_token())
        }
    }

    fn parse_component(&mut self) -> Result<ComponentFile> {
        // Detect which style is being used
        if self.is_minimal_style()? {
            self.parse_minimal()
        } else {
            self.parse_advanced()
        }
    }

    /// Detect if this is minimal style (no @component decorator)
    fn is_minimal_style(&self) -> Result<bool> {
        // Look for @component decorator
        for token_with_loc in &self.tokens {
            if let Token::Decorator(name) = &token_with_loc.token {
                if name == "component" {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Parse minimal syntax component
    fn parse_minimal(&mut self) -> Result<ComponentFile> {
        let mut component = MinimalComponent::new();

        // Parse top-level declarations
        while self.current_token() != &Token::Eof {
            match self.current_token() {
                // State declaration: count: int = 0
                Token::Ident(_) if matches!(self.peek(1), Token::Colon) => {
                    component.state.push(self.parse_state_decl()?);
                }
                // Computed: @computed doubled: int = count * 2
                Token::Decorator(name) if name == "computed" => {
                    self.advance(); // consume @computed decorator
                    component
                        .computed
                        .push(self.parse_computed_decl_after_decorator()?);
                }
                // Function: fn increment() { ... }
                Token::Fn => {
                    // Check for lifecycle decorators
                    let lifecycle = self.check_lifecycle_decorator();
                    self.advance(); // consume 'fn' token
                    let func = self.parse_function_after_fn_keyword()?;

                    if let Some(kind) = lifecycle {
                        component.lifecycle.push(LifecycleHook {
                            kind,
                            body: func.body,
                        });
                    } else {
                        component.functions.push(func);
                    }
                }
                // View block: view { ... }
                Token::Ident(name) if name == "view" => {
                    component.view = self.parse_view_block()?;
                }
                // Lifecycle: @on_mount fn setup() { ... }
                Token::Decorator(name) if name.starts_with("on_") => {
                    let kind = self.parse_lifecycle_kind(name)?;
                    self.advance(); // consume decorator
                    self.expect(Token::Fn)?; // consume 'fn' keyword
                    let func = self.parse_function_after_fn_keyword()?;
                    component.lifecycle.push(LifecycleHook {
                        kind,
                        body: func.body,
                    });
                }
                _ => {
                    bail!(
                        "Unexpected token in minimal component: {:?}",
                        self.current_token()
                    );
                }
            }
        }

        Ok(ComponentFile::minimal(component, None))
    }

    fn check_lifecycle_decorator(&self) -> Option<LifecycleKind> {
        if self.pos > 0 {
            if let Token::Decorator(name) = &self.tokens[self.pos - 1].token {
                return self.parse_lifecycle_kind(name).ok();
            }
        }
        None
    }

    fn parse_lifecycle_kind(&self, name: &str) -> Result<LifecycleKind> {
        match name {
            "on_mount" => Ok(LifecycleKind::OnMount),
            "on_destroy" => Ok(LifecycleKind::OnDestroy),
            "on_update" => Ok(LifecycleKind::OnUpdate),
            _ => bail!("Unknown lifecycle hook: {}", name),
        }
    }

    fn parse_state_decl(&mut self) -> Result<StateDecl> {
        // count: int = 0
        let Token::Ident(name) = self.current_token().clone() else {
            bail!("Expected identifier for state declaration");
        };
        self.advance();

        self.expect(Token::Colon)?;

        let type_ = self.parse_type_with_windjammer_parser()?;

        self.expect(Token::Assign)?;

        let initial_value = self.parse_expression_with_windjammer_parser()?;

        Ok(StateDecl {
            name,
            type_,
            initial_value,
            mutable: true, // All state is mutable in components
        })
    }

    fn _parse_computed_decl(&mut self) -> Result<ComputedDecl> {
        // @computed doubled: int = count * 2
        self.expect(Token::Decorator("computed".to_string()))?;
        self.parse_computed_decl_after_decorator()
    }

    fn parse_computed_decl_after_decorator(&mut self) -> Result<ComputedDecl> {
        // doubled: int = count * 2 (decorator already consumed)
        let Token::Ident(name) = self.current_token().clone() else {
            bail!("Expected identifier for computed declaration");
        };
        self.advance();

        self.expect(Token::Colon)?;

        let type_ = Some(self.parse_type_with_windjammer_parser()?);

        self.expect(Token::Assign)?;

        let expression = self.parse_expression_with_windjammer_parser()?;

        Ok(ComputedDecl {
            name,
            type_,
            expression,
        })
    }

    fn parse_view_block(&mut self) -> Result<ViewBlock> {
        // view { ... }
        let Token::Ident(name) = self.current_token() else {
            bail!("Expected 'view' keyword");
        };
        if name != "view" {
            bail!("Expected 'view' keyword, got {}", name);
        }
        self.advance();

        self.expect(Token::LBrace)?;

        let root = self.parse_view_node()?;

        self.expect(Token::RBrace)?;

        Ok(ViewBlock { root })
    }

    fn parse_view_node(&mut self) -> Result<ViewNode> {
        match self.current_token() {
            // Element: div { ... } or button(on_click: handler) { ... }
            Token::Ident(_) => self.parse_element_or_component(),
            // Text: "Hello {name}"
            Token::StringLiteral(_) | Token::InterpolatedString(_) => self.parse_text_node(),
            // Conditional: if condition { ... }
            Token::If => self.parse_if_node(),
            // Loop: for item in items { ... }
            Token::For => self.parse_for_node(),
            _ => bail!("Unexpected token in view: {:?}", self.current_token()),
        }
    }

    fn parse_element_or_component(&mut self) -> Result<ViewNode> {
        let Token::Ident(name) = self.current_token().clone() else {
            bail!("Expected identifier");
        };
        self.advance();

        // Check if it's a component (starts with uppercase) or element (lowercase)
        let is_component = name.chars().next().unwrap().is_uppercase();

        if is_component {
            self.parse_component_node(name)
        } else {
            self.parse_element_node(name)
        }
    }

    fn parse_element_node(&mut self, tag: String) -> Result<ViewNode> {
        let mut attributes = Vec::new();
        let mut children = Vec::new();

        // Parse attributes: (class: "foo", on_click: handler)
        if self.current_token() == &Token::LParen {
            self.advance();
            while self.current_token() != &Token::RParen {
                attributes.push(self.parse_attribute()?);
                if self.current_token() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect(Token::RParen)?;
        }

        // Parse children: { ... }
        if self.current_token() == &Token::LBrace {
            self.advance();
            while self.current_token() != &Token::RBrace {
                children.push(self.parse_view_node()?);
            }
            self.expect(Token::RBrace)?;
        }

        Ok(ViewNode::Element(ElementNode {
            tag,
            attributes,
            children,
        }))
    }

    fn parse_attribute(&mut self) -> Result<Attribute> {
        let Token::Ident(name) = self.current_token().clone() else {
            bail!("Expected attribute name");
        };
        self.advance();

        self.expect(Token::Colon)?;

        // Check if it's an event handler (starts with "on_")
        if name.starts_with("on_") {
            let handler = self.parse_expression_with_windjammer_parser()?;
            Ok(Attribute::Event { name, handler })
        } else if matches!(self.current_token(), Token::StringLiteral(_)) {
            // Static attribute
            let Token::StringLiteral(value) = self.current_token().clone() else {
                unreachable!();
            };
            self.advance();
            Ok(Attribute::Static { name, value })
        } else {
            // Dynamic attribute
            let value = self.parse_expression_with_windjammer_parser()?;
            Ok(Attribute::Dynamic { name, value })
        }
    }

    fn parse_component_node(&mut self, name: String) -> Result<ViewNode> {
        let mut props = Vec::new();
        let mut children = Vec::new();

        // Parse props: (text: "Click", on_click: handler)
        if self.current_token() == &Token::LParen {
            self.advance();
            while self.current_token() != &Token::RParen {
                let Token::Ident(prop_name) = self.current_token().clone() else {
                    bail!("Expected prop name");
                };
                self.advance();

                self.expect(Token::Colon)?;

                let value = self.parse_expression_with_windjammer_parser()?;
                props.push((prop_name, value));

                if self.current_token() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect(Token::RParen)?;
        }

        // Parse children: { ... }
        if self.current_token() == &Token::LBrace {
            self.advance();
            while self.current_token() != &Token::RBrace {
                children.push(self.parse_view_node()?);
            }
            self.expect(Token::RBrace)?;
        }

        Ok(ViewNode::Component(ComponentNode {
            name,
            props,
            children,
        }))
    }

    fn parse_text_node(&mut self) -> Result<ViewNode> {
        let parts = match self.current_token().clone() {
            Token::StringLiteral(s) => {
                self.advance();
                vec![TextPart::Static(s)]
            }
            Token::InterpolatedString(parts) => {
                self.advance();
                parts
                    .into_iter()
                    .map(|part| match part {
                        crate::lexer::StringPart::Literal(s) => TextPart::Static(s),
                        crate::lexer::StringPart::Expression(expr) => {
                            // Parse the expression
                            let mut lexer = Lexer::new(&expr);
                            let tokens = lexer.tokenize_with_locations();
                            let mut parser = WindjammerParser::new(tokens);
                            let expr = parser
                                .parse_expression_public()
                                .map_err(|e| anyhow::anyhow!(e))
                                .unwrap();
                            TextPart::Dynamic(expr)
                        }
                    })
                    .collect()
            }
            _ => bail!("Expected string literal"),
        };

        Ok(ViewNode::Text(TextNode { parts }))
    }

    fn parse_if_node(&mut self) -> Result<ViewNode> {
        self.expect(Token::If)?;

        let condition = self.parse_expression_with_windjammer_parser()?;

        self.expect(Token::LBrace)?;
        let mut then_branch = Vec::new();
        while self.current_token() != &Token::RBrace {
            then_branch.push(self.parse_view_node()?);
        }
        self.expect(Token::RBrace)?;

        let else_branch = if self.current_token() == &Token::Else {
            self.advance();
            self.expect(Token::LBrace)?;
            let mut else_nodes = Vec::new();
            while self.current_token() != &Token::RBrace {
                else_nodes.push(self.parse_view_node()?);
            }
            self.expect(Token::RBrace)?;
            Some(else_nodes)
        } else {
            None
        };

        Ok(ViewNode::If(IfNode {
            condition,
            then_branch,
            else_branch,
        }))
    }

    fn parse_for_node(&mut self) -> Result<ViewNode> {
        self.expect(Token::For)?;

        let Token::Ident(pattern) = self.current_token().clone() else {
            bail!("Expected identifier in for loop");
        };
        self.advance();

        self.expect(Token::In)?;

        let iterable = self.parse_expression_with_windjammer_parser()?;

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.current_token() != &Token::RBrace {
            body.push(self.parse_view_node()?);
        }
        self.expect(Token::RBrace)?;

        Ok(ViewNode::For(ForNode {
            pattern,
            iterable,
            body,
        }))
    }

    /// Parse advanced syntax component
    fn parse_advanced(&mut self) -> Result<ComponentFile> {
        // Use the main Windjammer parser to parse the struct and impl block
        let program = self.parse_with_windjammer_parser()?;

        // Extract @component struct
        let struct_decl = program
            .items
            .iter()
            .find_map(|item| {
                if let Item::Struct { decl: s, .. } = item {
                    if s.decorators.iter().any(|d| d.name == "component") {
                        return Some(s.clone());
                    }
                }
                None
            })
            .ok_or_else(|| anyhow::anyhow!("No @component struct found"))?;

        // Extract impl block
        let impl_block = program
            .items
            .iter()
            .find_map(|item| {
                if let Item::Impl { block: i, .. } = item {
                    if i.type_name == struct_decl.name {
                        return Some(i.clone());
                    }
                }
                None
            })
            .ok_or_else(|| anyhow::anyhow!("No impl block found for component"))?;

        // Convert to component AST
        let component_struct = ComponentStruct {
            name: struct_decl.name.clone(),
            fields: struct_decl
                .fields
                .iter()
                .map(|f| StructField {
                    name: f.name.clone(),
                    type_: f.field_type.clone(),
                    default: None, // TODO: Parse default values
                })
                .collect(),
            decorators: struct_decl
                .decorators
                .iter()
                .map(|d| d.name.clone())
                .collect(),
        };

        let component_impl = ComponentImpl {
            type_name: impl_block.type_name.clone(),
            methods: impl_block
                .functions
                .iter()
                .map(|f| {
                    let kind = if f.name == "render" {
                        MethodKind::Render
                    } else if f.decorators.iter().any(|d| d.name == "computed") {
                        MethodKind::Computed
                    } else if f.name.starts_with("on_") {
                        if let Ok(lifecycle) = self.parse_lifecycle_kind(&f.name) {
                            MethodKind::Lifecycle(lifecycle)
                        } else {
                            MethodKind::EventHandler
                        }
                    } else {
                        MethodKind::Helper
                    };

                    ComponentMethod {
                        function: f.clone(),
                        kind,
                    }
                })
                .collect(),
        };

        Ok(ComponentFile::advanced(AdvancedComponent {
            struct_decl: component_struct,
            impl_block: component_impl,
        }))
    }

    // Helper methods to use the main Windjammer parser
    fn parse_with_windjammer_parser(&self) -> Result<Program> {
        let mut parser = WindjammerParser::new(self.tokens.clone());
        parser.parse().map_err(|e| anyhow::anyhow!(e))
    }

    fn parse_expression_with_windjammer_parser(&mut self) -> Result<Expression> {
        // Extract tokens from current position to next delimiter
        let start = self.pos;
        let mut depth = 0;
        let mut brace_depth = 0; // Track braces separately for if/match/loop expressions
        while self.pos < self.tokens.len() {
            match self.current_token() {
                Token::LParen | Token::LBracket => {
                    depth += 1;
                    self.advance();
                }
                Token::LBrace => {
                    if depth > 0 || brace_depth > 0 {
                        // Inside parens/brackets or already in a brace block
                        brace_depth += 1;
                        self.advance();
                    } else {
                        // Check if this is part of an if/match/loop expression
                        // by scanning backwards for control flow keywords
                        let mut found_control_flow = false;
                        for i in (start..self.pos).rev() {
                            match &self.tokens[i].token {
                                Token::If
                                | Token::Else
                                | Token::Match
                                | Token::Loop
                                | Token::While
                                | Token::For => {
                                    found_control_flow = true;
                                    break;
                                }
                                // Stop searching if we hit a delimiter
                                Token::Comma | Token::Semicolon => break,
                                _ => continue,
                            }
                        }

                        if found_control_flow {
                            // This brace is part of a control flow expression
                            brace_depth += 1;
                            self.advance();
                        } else {
                            // Stop at opening brace at depth 0 (start of HTML block)
                            break;
                        }
                    }
                }
                Token::RBrace => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                        self.advance();
                    } else if depth == 0 {
                        break;
                    } else {
                        self.advance();
                    }
                }
                Token::RParen | Token::RBracket => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    self.advance();
                }
                Token::Comma | Token::Colon if depth == 0 && brace_depth == 0 => break,
                Token::Eof => break,
                // Stop at decorators, keywords, and other top-level constructs
                Token::Decorator(_) | Token::Fn | Token::Struct | Token::Enum | Token::Impl
                    if depth == 0 && brace_depth == 0 =>
                {
                    break
                }
                Token::Ident(name) if depth == 0 && brace_depth == 0 && name == "view" => break,
                _ => {
                    self.advance();
                }
            }
        }

        let expr_tokens = self.tokens[start..self.pos].to_vec();
        let mut parser = WindjammerParser::new(expr_tokens);
        parser
            .parse_expression_public()
            .map_err(|e| anyhow::anyhow!(e))
    }

    fn parse_type_with_windjammer_parser(&mut self) -> Result<Type> {
        // Extract tokens for type
        let start = self.pos;
        let mut depth = 0;
        while self.pos < self.tokens.len() {
            match self.current_token() {
                Token::Lt => depth += 1,
                Token::Gt => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                Token::Assign | Token::Comma if depth == 0 => break,
                Token::Eof => break,
                _ => {}
            }
            self.advance();
        }

        let type_tokens = self.tokens[start..self.pos].to_vec();
        let mut parser = WindjammerParser::new(type_tokens);
        parser.parse_type_public().map_err(|e| anyhow::anyhow!(e))
    }

    fn parse_function_after_fn_keyword(&mut self) -> Result<FunctionDecl> {
        // Extract tokens for function (fn keyword already consumed)
        // Current token should be the function name
        let start = self.pos;
        let mut depth = 0;

        // Advance through the function tokens
        loop {
            match self.current_token() {
                Token::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Token::RBrace => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                }
                Token::Eof => break,
                _ => {
                    self.advance();
                }
            }
        }

        let func_tokens = self.tokens[start..self.pos].to_vec();
        let mut parser = WindjammerParser::new(func_tokens);
        parser
            .parse_function_public()
            .map_err(|e| anyhow::anyhow!(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_minimal_style() {
        let minimal = r#"
count: int = 0
fn increment() { count = count + 1 }
view { button(on_click: increment) { "Click" } }
"#;
        let result = ComponentParser::parse(minimal);
        if let Err(e) = &result {
            eprintln!("Parse error: {}", e);
        }
        assert!(result.is_ok());
        let component = result.unwrap();
        assert!(matches!(component.style, ComponentStyle::Minimal(_)));
    }

    #[test]
    fn test_detect_advanced_style() {
        let advanced = r#"
@component
struct Counter {
    count: int
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }
}
"#;
        let result = ComponentParser::parse(advanced);
        assert!(result.is_ok());
        let component = result.unwrap();
        assert!(matches!(component.style, ComponentStyle::Advanced(_)));
    }

    #[test]
    fn test_parse_state_declaration() {
        let source = r#"
count: int = 0
view { div { "Hello" } }
"#;
        let result = ComponentParser::parse(source);
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            assert_eq!(minimal.state.len(), 1);
            assert_eq!(minimal.state[0].name, "count");
        } else {
            panic!("Expected minimal style");
        }
    }

    #[test]
    fn test_parse_computed_declaration() {
        let source = r#"
count: int = 0
@computed
doubled: int = count + count
view { div { "Hello" } }
"#;
        let result = ComponentParser::parse(source);
        if let Err(e) = &result {
            eprintln!("Parse error: {}", e);
        }
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            assert_eq!(minimal.computed.len(), 1);
            assert_eq!(minimal.computed[0].name, "doubled");
        } else {
            panic!("Expected minimal style");
        }
    }

    #[test]
    fn test_parse_function() {
        let source = r#"
count: int = 0
fn increment() {
    count = count + 1
}
view { div { "Hello" } }
"#;
        let result = ComponentParser::parse(source);
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            assert_eq!(minimal.functions.len(), 1);
            assert_eq!(minimal.functions[0].name, "increment");
        } else {
            panic!("Expected minimal style");
        }
    }

    #[test]
    fn test_parse_view_with_element() {
        let source = r#"
view {
    div {
        "Hello World"
    }
}
"#;
        let result = ComponentParser::parse(source);
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            assert!(matches!(minimal.view.root, ViewNode::Element(_)));
        } else {
            panic!("Expected minimal style");
        }
    }

    #[test]
    fn test_parse_view_with_attributes() {
        let source = r#"
count: int = 0
view {
    button(class: "btn", on_click: increment) {
        "Click me"
    }
}
"#;
        let result = ComponentParser::parse(source);
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            if let ViewNode::Element(elem) = &minimal.view.root {
                assert_eq!(elem.tag, "button");
                assert_eq!(elem.attributes.len(), 2);
            } else {
                panic!("Expected element node");
            }
        } else {
            panic!("Expected minimal style");
        }
    }

    #[test]
    fn test_parse_view_with_conditional() {
        let source = r#"
show: bool = true
view {
    if show {
        div { "Visible" }
    }
}
"#;
        let result = ComponentParser::parse(source);
        if let Err(e) = &result {
            eprintln!("Parse error: {}", e);
        }
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            assert!(matches!(minimal.view.root, ViewNode::If(_)));
        } else {
            panic!("Expected minimal style");
        }
    }

    #[test]
    fn test_parse_view_with_loop() {
        let source = r#"
items: Vec<int> = vec![1, 2, 3]
view {
    for item in items {
        div { "Item" }
    }
}
"#;
        let result = ComponentParser::parse(source);
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            assert!(matches!(minimal.view.root, ViewNode::For(_)));
        } else {
            panic!("Expected minimal style");
        }
    }

    #[test]
    fn test_complete_counter_component() {
        let source = r#"
count: int = 0

fn increment() {
    count = count + 1
}

fn decrement() {
    count = count - 1
}

view {
    div(class: "counter") {
        button(on_click: decrement) { "-" }
        "Count: {count}"
        button(on_click: increment) { "+" }
    }
}
"#;
        let result = ComponentParser::parse(source);
        assert!(result.is_ok());
        let component = result.unwrap();

        if let ComponentStyle::Minimal(minimal) = component.style {
            assert_eq!(minimal.state.len(), 1);
            assert_eq!(minimal.functions.len(), 2);
            assert!(matches!(minimal.view.root, ViewNode::Element(_)));
        } else {
            panic!("Expected minimal style");
        }
    }
}
