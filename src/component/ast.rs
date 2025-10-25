//! Abstract Syntax Tree for `.wj` component files
//!
//! Supports two syntax styles:
//! 1. Minimal: Top-level declarations (no struct wrapper)
//! 2. Advanced: Struct-based with @component decorator

use crate::parser::{Expression, FunctionDecl, Statement, Type};

/// A parsed component file
#[derive(Debug, Clone)]
pub struct ComponentFile {
    pub style: ComponentStyle,
    pub name: Option<String>, // Inferred from filename for minimal style
}

/// Component style (minimal vs. advanced)
#[derive(Debug, Clone)]
pub enum ComponentStyle {
    /// Minimal syntax: top-level declarations
    Minimal(MinimalComponent),
    /// Advanced syntax: struct-based
    Advanced(AdvancedComponent),
}

/// Minimal component (no struct wrapper)
#[derive(Debug, Clone)]
pub struct MinimalComponent {
    /// Reactive state variables (count: int = 0)
    pub state: Vec<StateDecl>,
    /// Computed values (@computed doubled: int = count * 2)
    pub computed: Vec<ComputedDecl>,
    /// Functions (fn increment() { ... })
    pub functions: Vec<FunctionDecl>,
    /// View block (view { ... })
    pub view: ViewBlock,
    /// Lifecycle hooks (optional)
    pub lifecycle: Vec<LifecycleHook>,
}

/// Advanced component (struct-based)
#[derive(Debug, Clone)]
pub struct AdvancedComponent {
    pub struct_decl: ComponentStruct,
    pub impl_block: ComponentImpl,
}

/// State declaration in minimal syntax
#[derive(Debug, Clone)]
pub struct StateDecl {
    pub name: String,
    pub type_: Type,
    pub initial_value: Expression,
    pub mutable: bool,
}

/// Computed declaration
#[derive(Debug, Clone)]
pub struct ComputedDecl {
    pub name: String,
    pub type_: Option<Type>,
    pub expression: Expression,
}

/// View block (view { ... })
#[derive(Debug, Clone)]
pub struct ViewBlock {
    pub root: ViewNode,
}

/// A node in the view tree
#[derive(Debug, Clone)]
pub enum ViewNode {
    /// Element: div { ... }
    Element(ElementNode),
    /// Text with interpolation: "Hello {name}"
    Text(TextNode),
    /// Conditional: if condition { ... }
    If(IfNode),
    /// Loop: for item in items { ... }
    For(ForNode),
    /// Component: Button(text: "Click", on_click: handler)
    Component(ComponentNode),
}

/// Element node
#[derive(Debug, Clone)]
pub struct ElementNode {
    pub tag: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<ViewNode>,
}

/// Element attribute
#[derive(Debug, Clone)]
pub enum Attribute {
    /// Static: class: "foo"
    Static { name: String, value: String },
    /// Dynamic: class: className
    Dynamic { name: String, value: Expression },
    /// Event: on_click: handler
    Event { name: String, handler: Expression },
}

/// Text node with interpolation
#[derive(Debug, Clone)]
pub struct TextNode {
    pub parts: Vec<TextPart>,
}

#[derive(Debug, Clone)]
pub enum TextPart {
    /// Static text: "Hello "
    Static(String),
    /// Interpolated: {name}
    Dynamic(Expression),
}

/// Conditional node
#[derive(Debug, Clone)]
pub struct IfNode {
    pub condition: Expression,
    pub then_branch: Vec<ViewNode>,
    pub else_branch: Option<Vec<ViewNode>>,
}

/// Loop node
#[derive(Debug, Clone)]
pub struct ForNode {
    pub pattern: String, // item name
    pub iterable: Expression,
    pub body: Vec<ViewNode>,
}

/// Component instance node
#[derive(Debug, Clone)]
pub struct ComponentNode {
    pub name: String,
    pub props: Vec<(String, Expression)>,
    pub children: Vec<ViewNode>,
}

/// Lifecycle hook
#[derive(Debug, Clone)]
pub struct LifecycleHook {
    pub kind: LifecycleKind,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleKind {
    OnMount,
    OnDestroy,
    OnUpdate,
}

/// Advanced component struct
#[derive(Debug, Clone)]
pub struct ComponentStruct {
    pub name: String,
    pub fields: Vec<StructField>,
    pub decorators: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_: Type,
    pub default: Option<Expression>,
}

/// Advanced component impl block
#[derive(Debug, Clone)]
pub struct ComponentImpl {
    pub type_name: String,
    pub methods: Vec<ComponentMethod>,
}

#[derive(Debug, Clone)]
pub struct ComponentMethod {
    pub function: FunctionDecl,
    pub kind: MethodKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodKind {
    Computed,
    EventHandler,
    Render,
    Lifecycle(LifecycleKind),
    Helper,
}

impl MinimalComponent {
    pub fn new() -> Self {
        Self {
            state: Vec::new(),
            computed: Vec::new(),
            functions: Vec::new(),
            view: ViewBlock::empty(),
            lifecycle: Vec::new(),
        }
    }
}

impl Default for MinimalComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewBlock {
    pub fn empty() -> Self {
        Self {
            root: ViewNode::Element(ElementNode {
                tag: "div".to_string(),
                attributes: Vec::new(),
                children: Vec::new(),
            }),
        }
    }
}

impl ComponentFile {
    pub fn minimal(component: MinimalComponent, name: Option<String>) -> Self {
        Self {
            style: ComponentStyle::Minimal(component),
            name,
        }
    }

    pub fn advanced(component: AdvancedComponent) -> Self {
        let name = Some(component.struct_decl.name.clone());
        Self {
            style: ComponentStyle::Advanced(component),
            name,
        }
    }
}
