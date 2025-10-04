use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Int32,
    Uint,
    Float,
    Bool,
    String,
    Custom(String),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Vec(Box<Type>),
    Reference(Box<Type>),
    MutableReference(Box<Type>),
    Tuple(Vec<Type>),  // Tuple type: (T1, T2, T3)
}

#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipHint {
    Owned,
    Ref,
    Mut,
    Inferred, // Let the analyzer decide
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,  // For simple parameters and backward compatibility
    pub pattern: Option<Pattern>,  // For pattern matching parameters
    pub type_: Type,
    pub ownership: OwnershipHint,
}

#[derive(Debug, Clone)]
pub struct Decorator {
    pub name: String,
    pub arguments: Vec<(String, Expression)>, // Named arguments
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub decorators: Vec<Decorator>,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
    pub decorators: Vec<Decorator>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub decorators: Vec<Decorator>,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let {
        name: String,
        mutable: bool,
        type_: Option<Type>,
        value: Expression,
    },
    Const {
        name: String,
        type_: Type,
        value: Expression,
    },
    Static {
        name: String,
        mutable: bool,
        type_: Type,
        value: Expression,
    },
    Assignment {
        target: Expression,
        value: Expression,
    },
    Return(Option<Expression>),
    Expression(Expression),
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    Match {
        value: Expression,
        arms: Vec<MatchArm>,
    },
    For {
        variable: String,
        iterable: Expression,
        body: Vec<Statement>,
    },
    Loop {
        body: Vec<Statement>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    Go {
        body: Vec<Statement>,
    },
    Defer(Box<Statement>),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expression>,  // Optional guard: if condition
    pub body: Expression,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Identifier(String),
    EnumVariant(String, Option<String>), // Enum name, optional binding
    Literal(Literal),
    Tuple(Vec<Pattern>), // Tuple pattern: (a, b, c)
    Or(Vec<Pattern>),    // Or pattern: pattern1 | pattern2 | pattern3
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Ternary {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },
    Call {
        function: Box<Expression>,
        arguments: Vec<(Option<String>, Expression)>,  // (label, expr)
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        arguments: Vec<(Option<String>, Expression)>,  // (label, expr)
    },
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
    },
    Closure {
        parameters: Vec<String>,
        body: Box<Expression>,
    },
    Cast {
        expr: Box<Expression>,
        type_: Type,
    },
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },
    Tuple(Vec<Expression>), // Tuple expression: (a, b, c)
    MacroInvocation {
        name: String,
        args: Vec<Expression>,
        delimiter: MacroDelimiter, // (), [], or {}
    },
    TryOp(Box<Expression>), // The ? operator
    Await(Box<Expression>), // The .await
    ChannelSend {
        channel: Box<Expression>,
        value: Box<Expression>,
    },
    ChannelRecv(Box<Expression>),
    Block(Vec<Statement>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacroDelimiter {
    Parens,   // println!()
    Brackets, // vec![]
    Braces,   // macro_name!{}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
    Ref,   // & operator
    Deref, // * operator (dereference)
}

#[derive(Debug, Clone)]
pub struct TraitDecl {
    pub name: String,
    pub generics: Vec<String>,  // Generic parameters like <T, U>
    pub methods: Vec<TraitMethod>,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub is_async: bool,
    pub body: Option<Vec<Statement>>,  // None for trait definitions, Some for default impls
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub type_name: String,
    pub trait_name: Option<String>,  // None for inherent impl, Some for trait impl
    pub functions: Vec<FunctionDecl>,
    pub decorators: Vec<Decorator>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplBlock),
    Const { name: String, type_: Type, value: Expression },
    Static { name: String, mutable: bool, type_: Type, value: Expression },
    Use(Vec<String>), // use std.fs -> ["std", "fs"]
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }
    
    fn current_token(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::Eof)
    }
    
    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }
    
    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.current_token() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.current_token()))
        }
    }
    
    pub fn parse(&mut self) -> Result<Program, String> {
        let mut items = Vec::new();
        
        while self.current_token() != &Token::Eof {
            items.push(self.parse_item()?);
        }
        
        Ok(Program { items })
    }
    
    fn parse_item(&mut self) -> Result<Item, String> {
        // Check for decorators
        let mut decorators = Vec::new();
        while let Token::Decorator(_) = self.current_token() {
            decorators.push(self.parse_decorator()?);
        }
        
        // Check for pub keyword (for module functions)
        let _is_pub = if self.current_token() == &Token::Pub {
            self.advance();
            true
        } else {
            false
        };
        
        match self.current_token() {
            Token::Fn => {
                self.advance(); // Consume the Fn token
                let mut func = self.parse_function()?;
                func.decorators = decorators;
                Ok(Item::Function(func))
            }
            Token::Async => {
                self.advance();
                self.expect(Token::Fn)?;
                let mut func = self.parse_function()?;
                func.is_async = true;
                func.decorators = decorators;
                Ok(Item::Function(func))
            }
            Token::Struct => {
                self.advance();
                let mut struct_decl = self.parse_struct()?;
                struct_decl.decorators = decorators;
                Ok(Item::Struct(struct_decl))
            }
            Token::Enum => {
                self.advance();
                Ok(Item::Enum(self.parse_enum()?))
            }
            Token::Trait => {
                self.advance();
                Ok(Item::Trait(self.parse_trait()?))
            }
            Token::Impl => {
                self.advance();
                let mut impl_block = self.parse_impl()?;
                impl_block.decorators = decorators;
                Ok(Item::Impl(impl_block))
            }
            Token::Const => {
                self.advance();
                let (name, type_, value) = self.parse_const_or_static()?;
                Ok(Item::Const { name, type_, value })
            }
            Token::Static => {
                self.advance();
                let mutable = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                let (name, type_, value) = self.parse_const_or_static()?;
                Ok(Item::Static { name, mutable, type_, value })
            }
            Token::Use => {
                self.advance(); // consume 'use'
                Ok(Item::Use(self.parse_use()?))
            }
            _ => Err(format!("Unexpected token: {:?}", self.current_token())),
        }
    }
    
    fn parse_const_or_static(&mut self) -> Result<(String, Type, Expression), String> {
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected const/static name".to_string());
        };
        
        self.expect(Token::Colon)?;
        let type_ = self.parse_type()?;
        
        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;
        
        Ok((name, type_, value))
    }
    
    fn parse_impl(&mut self) -> Result<ImplBlock, String> {
        // Parse: impl Type { } or impl Trait for Type { }
        let first_name = if let Token::Ident(name) = self.current_token() {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err("Expected type or trait name after impl".to_string());
        };
        
        // Check if this is "impl Trait for Type" or just "impl Type"
        let (trait_name, type_name) = if self.current_token() == &Token::For {
            self.advance(); // consume "for"
            let type_name = if let Token::Ident(name) = self.current_token() {
                let name = name.clone();
                self.advance();
                name
            } else {
                return Err("Expected type name after 'for'".to_string());
            };
            (Some(first_name), type_name)
        } else {
            (None, first_name)
        };
        
        self.expect(Token::LBrace)?;
        
        let mut functions = Vec::new();
        while self.current_token() != &Token::RBrace {
            // Skip decorators for now (could be added later)
            let mut decorators = Vec::new();
            while let Token::Decorator(_) = self.current_token() {
                decorators.push(self.parse_decorator()?);
            }
            
            // Parse function (pub optional)
            if self.current_token() == &Token::Pub {
                self.advance();
            }
            
            let is_async = if self.current_token() == &Token::Async {
                self.advance();
                true
            } else {
                false
            };
            
            self.expect(Token::Fn)?;
            let mut func = self.parse_function()?;
            func.is_async = is_async;
            func.decorators = decorators;
            functions.push(func);
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(ImplBlock { type_name, trait_name, functions, decorators: Vec::new() })
    }
    
    fn parse_trait(&mut self) -> Result<TraitDecl, String> {
        // Parse: trait Name<T, U> { methods }
        let name = if let Token::Ident(n) = self.current_token() {
            let n = n.clone();
            self.advance();
            n
        } else {
            return Err("Expected trait name".to_string());
        };
        
        // Parse optional generic parameters
        let generics = if self.current_token() == &Token::Lt {
            self.advance();
            let mut params = Vec::new();
            
            while self.current_token() != &Token::Gt {
                if let Token::Ident(param) = self.current_token() {
                    params.push(param.clone());
                    self.advance();
                    
                    if self.current_token() == &Token::Comma {
                        self.advance();
                    }
                } else {
                    return Err("Expected generic parameter name".to_string());
                }
            }
            
            self.expect(Token::Gt)?;
            params
        } else {
            Vec::new()
        };
        
        self.expect(Token::LBrace)?;
        
        let mut methods = Vec::new();
        while self.current_token() != &Token::RBrace {
            // Parse trait method signature
            let is_async = if self.current_token() == &Token::Async {
                self.advance();
                true
            } else {
                false
            };
            
            self.expect(Token::Fn)?;
            
            let method_name = if let Token::Ident(n) = self.current_token() {
                let n = n.clone();
                self.advance();
                n
            } else {
                return Err("Expected method name in trait".to_string());
            };
            
            self.expect(Token::LParen)?;
            let parameters = self.parse_parameters()?;
            self.expect(Token::RParen)?;
            
            let return_type = if self.current_token() == &Token::Arrow {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            
            // Check for default implementation (optional body)
            let body = if self.current_token() == &Token::LBrace {
                self.advance();
                let statements = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                Some(statements)
            } else {
                None
            };
            
            methods.push(TraitMethod {
                name: method_name,
                parameters,
                return_type,
                is_async,
                body,
            });
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(TraitDecl { name, generics, methods })
    }
    
    fn parse_decorator(&mut self) -> Result<Decorator, String> {
        if let Token::Decorator(name) = self.current_token() {
            let name = name.clone();
            self.advance();
            
            // Check for decorator arguments: @route("/path") or @cache(ttl: 60)
            let arguments = if self.current_token() == &Token::LParen {
                self.advance();
                self.parse_decorator_arguments()?
            } else {
                Vec::new()
            };
            
            Ok(Decorator { name, arguments })
        } else {
            Err("Expected decorator".to_string())
        }
    }
    
    fn parse_decorator_arguments(&mut self) -> Result<Vec<(String, Expression)>, String> {
        let mut args = Vec::new();
        
        while self.current_token() != &Token::RParen {
            // Check if it's a named argument (key: value)
            if let Token::Ident(key) = self.current_token() {
                let key = key.clone();
                self.advance();
                
                if self.current_token() == &Token::Colon {
                    self.advance();
                    let value = self.parse_expression()?;
                    args.push((key, value));
                } else {
                    // Positional argument (just a string or expression)
                    // Reparse as expression
                    let expr = Expression::Identifier(key);
                    args.push((String::new(), expr));
                }
            } else {
                // Positional expression argument
                let expr = self.parse_expression()?;
                args.push((String::new(), expr));
            }
            
            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        self.expect(Token::RParen)?;
        Ok(args)
    }
    
    fn parse_use(&mut self) -> Result<Vec<String>, String> {
        // Note: Token::Use already consumed in parse_item
        
        let mut path = Vec::new();
        let mut path_str = String::new();
        
        // Handle relative imports: ./module or ../module
        if self.current_token() == &Token::Dot {
            path_str.push('.');
            self.advance();
            
            // Check for ./ or ../
            if self.current_token() == &Token::Slash {
                path_str.push('/');
                self.advance();
            } else if self.current_token() == &Token::Dot {
                // ../
                path_str.push('.');
                self.advance();
                if self.current_token() == &Token::Slash {
                    path_str.push('/');
                    self.advance();
                }
            }
        }
        
        // Parse the rest of the path (identifiers separated by . or /)
        loop {
            if let Token::Ident(name) = self.current_token() {
                path_str.push_str(name);
                self.advance();
                
                // Check for . or / as separator
                if self.current_token() == &Token::Dot {
                    path_str.push('.');
                    self.advance();
                } else if self.current_token() == &Token::Slash {
                    path_str.push('/');
                    self.advance();
                } else {
                    break;
                }
            } else if path_str.is_empty() {
                return Err(format!("Expected identifier in use statement"));
            } else {
                break;
            }
        }
        
        // For now, return the path as a single-element vector
        // This preserves the relative path structure
        path.push(path_str);
        
        Ok(path)
    }
    
    fn parse_function(&mut self) -> Result<FunctionDecl, String> {
        // Note: Token::Fn already consumed in parse_item
        
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected function name".to_string());
        };
        
        self.expect(Token::LParen)?;
        let parameters = self.parse_parameters()?;
        self.expect(Token::RParen)?;
        
        let return_type = if self.current_token() == &Token::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        
        Ok(FunctionDecl {
            name,
            decorators: Vec::new(), // Set by parse_item
            is_async: false,        // Set by parse_item
            parameters,
            return_type,
            body,
        })
    }
    
    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, String> {
        let mut params = Vec::new();
        
        while self.current_token() != &Token::RParen {
            // Check for self parameters
            if self.current_token() == &Token::Ampersand {
                self.advance();
                if self.current_token() == &Token::Mut {
                    self.advance();
                    self.expect(Token::Self_)?;
                params.push(Parameter {
                    name: "self".to_string(),
                    pattern: None,
                    type_: Type::Custom("Self".to_string()),
                    ownership: OwnershipHint::Mut,
                });
            } else {
                self.expect(Token::Self_)?;
                params.push(Parameter {
                    name: "self".to_string(),
                    pattern: None,
                    type_: Type::Custom("Self".to_string()),
                    ownership: OwnershipHint::Ref,
                });
            }
        } else if self.current_token() == &Token::Self_ {
            self.advance();
            params.push(Parameter {
                name: "self".to_string(),
                pattern: None,
                type_: Type::Custom("Self".to_string()),
                ownership: OwnershipHint::Owned,
            });
            } else {
                // Regular parameter - could be a simple name or a pattern
                let ownership = OwnershipHint::Inferred;
                
                // Check if this is a pattern parameter (starts with '(')
                if self.current_token() == &Token::LParen {
                    // Parse tuple pattern
                    let pattern = self.parse_pattern()?;
                    self.expect(Token::Colon)?;
                    let type_ = self.parse_type()?;
                    
                    // Extract a name from the pattern for backward compatibility
                    let name = Self::pattern_to_name(&pattern);
                    
                    params.push(Parameter { 
                        name, 
                        pattern: Some(pattern), 
                        type_, 
                        ownership 
                    });
                } else {
                    // Simple identifier parameter
            let name = if let Token::Ident(n) = self.current_token() {
                let name = n.clone();
                self.advance();
                name
            } else {
                return Err("Expected parameter name".to_string());
            };
            
            self.expect(Token::Colon)?;
            let type_ = self.parse_type()?;
            
                    params.push(Parameter { name, pattern: None, type_, ownership });
                }
            }
            
            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(params)
    }
    
    fn parse_type(&mut self) -> Result<Type, String> {
        // Handle reference types
        if self.current_token() == &Token::Ampersand {
            self.advance();
            if self.current_token() == &Token::Mut {
                self.advance();
                let inner = Box::new(self.parse_type()?);
                return Ok(Type::MutableReference(inner));
            } else {
                let inner = Box::new(self.parse_type()?);
                return Ok(Type::Reference(inner));
            }
        }
        
        let base_type = match self.current_token() {
            Token::Int => { self.advance(); Type::Int }
            Token::Int32 => { self.advance(); Type::Int32 }
            Token::Uint => { self.advance(); Type::Uint }
            Token::Float => { self.advance(); Type::Float }
            Token::Bool => { self.advance(); Type::Bool }
            Token::String => { self.advance(); Type::String }
            Token::LParen => {
                // Tuple type: (T1, T2, T3)
                self.advance();
                let mut types = Vec::new();
                
                while self.current_token() != &Token::RParen {
                    types.push(self.parse_type()?);
                    
                    if self.current_token() == &Token::Comma {
                        self.advance();
                    } else {
                        break;
                    }
                }
                
                self.expect(Token::RParen)?;
                Type::Tuple(types)
            }
            Token::Ident(name) => {
                let mut type_name = name.clone();
                self.advance();
                
                // Handle qualified type names (module.Type)
                while self.current_token() == &Token::Dot {
                    self.advance();
                    if let Token::Ident(segment) = self.current_token() {
                        type_name.push('.');
                        type_name.push_str(segment);
                        self.advance();
                    } else {
                        return Err("Expected identifier after '.' in type name".to_string());
                    }
                }
                
                // Check for generic parameters
                if self.current_token() == &Token::Lt {
                    self.advance();
                    
                    // Handle Vec<T>, Option<T>, Result<T, E>
                    if type_name == "Vec" {
                        let inner = Box::new(self.parse_type()?);
                        self.expect(Token::Gt)?;
                        Type::Vec(inner)
                    } else if type_name == "Option" {
                        let inner = Box::new(self.parse_type()?);
                        self.expect(Token::Gt)?;
                        Type::Option(inner)
                    } else if type_name == "Result" {
                        let ok_type = Box::new(self.parse_type()?);
                        self.expect(Token::Comma)?;
                        let err_type = Box::new(self.parse_type()?);
                        self.expect(Token::Gt)?;
                        Type::Result(ok_type, err_type)
                    } else {
                        // Generic custom type (not fully supported yet)
                        // Skip the generic params for now
                        let mut depth = 1;
                        while depth > 0 {
                            match self.current_token() {
                                Token::Lt => depth += 1,
                                Token::Gt => depth -= 1,
                                Token::Eof => return Err("Unexpected EOF in generic type".to_string()),
                                _ => {}
                            }
                            self.advance();
                        }
                        Type::Custom(type_name)
                    }
                } else {
                    Type::Custom(type_name)
                }
            }
            Token::LBracket => {
                // Slice type: [T] or array [T; N]
                self.advance();
                let inner = Box::new(self.parse_type()?);
                // For now, treat all as Vec
                self.expect(Token::RBracket)?;
                Type::Vec(inner)
            }
            _ => return Err(format!("Expected type, got {:?}", self.current_token())),
        };
        
        Ok(base_type)
    }
    
    fn parse_struct(&mut self) -> Result<StructDecl, String> {
        // Token::Struct already consumed in parse_item
        
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected struct name".to_string());
        };
        
        self.expect(Token::LBrace)?;
        
        let mut fields = Vec::new();
        while self.current_token() != &Token::RBrace {
            // Parse decorators on fields
            let mut field_decorators = Vec::new();
            while let Token::Decorator(_dec_name) = self.current_token() {
                let decorator = self.parse_decorator()?;
                field_decorators.push(decorator);
            }
            
            let field_name = if let Token::Ident(n) = self.current_token() {
                let name = n.clone();
                self.advance();
                name
            } else {
                return Err("Expected field name".to_string());
            };
            
            self.expect(Token::Colon)?;
            let field_type = self.parse_type()?;
            
            fields.push(StructField {
                name: field_name,
                field_type,
                decorators: field_decorators,
            });
            
            if self.current_token() == &Token::Comma {
                self.advance();
            }
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(StructDecl { name, fields, decorators: Vec::new() })
    }
    
    fn parse_enum(&mut self) -> Result<EnumDecl, String> {
        // Token::Enum already consumed in parse_item
        
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected enum name".to_string());
        };
        
        self.expect(Token::LBrace)?;
        
        let mut variants = Vec::new();
        while self.current_token() != &Token::RBrace {
            let variant_name = if let Token::Ident(n) = self.current_token() {
                let name = n.clone();
                self.advance();
                name
            } else {
                return Err("Expected variant name".to_string());
            };
            
            let data = if self.current_token() == &Token::LParen {
                self.advance();
                let type_ = self.parse_type()?;
                self.expect(Token::RParen)?;
                Some(type_)
            } else {
                None
            };
            
            variants.push(EnumVariant { name: variant_name, data });
            
            if self.current_token() == &Token::Comma {
                self.advance();
            }
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(EnumDecl { name, variants })
    }
    
    fn parse_block_statements(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();
        
        while self.current_token() != &Token::RBrace && self.current_token() != &Token::Eof {
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    fn parse_statement(&mut self) -> Result<Statement, String> {
        match self.current_token() {
            Token::Let => self.parse_let(),
            Token::Const => self.parse_const_statement(),
            Token::Static => self.parse_static_statement(),
            Token::Return => self.parse_return(),
            Token::If => self.parse_if(),
            Token::Match => self.parse_match(),
            Token::For => self.parse_for(),
            Token::Loop => self.parse_loop(),
            Token::While => self.parse_while(),
            Token::Go => self.parse_go(),
            Token::Defer => self.parse_defer(),
            Token::Break => { self.advance(); Ok(Statement::Break) }
            Token::Continue => { self.advance(); Ok(Statement::Continue) }
            _ => {
                // Try to parse as expression first
                let expr = self.parse_expression()?;
                
                // Check if this is an assignment (expr = value) or compound assignment (expr += value)
                match self.current_token() {
                    Token::Assign => {
                        self.advance(); // consume '='
                        let value = self.parse_expression()?;
                        Ok(Statement::Assignment {
                            target: expr,
                            value,
                        })
                    }
                    Token::PlusAssign | Token::MinusAssign | Token::StarAssign | 
                    Token::SlashAssign | Token::PercentAssign => {
                        let op_token = self.current_token().clone();
                        self.advance(); // consume compound operator
                        
                        let rhs = self.parse_expression()?;
                        
                        // Convert x += y to x = x + y
                        let op = match op_token {
                            Token::PlusAssign => BinaryOp::Add,
                            Token::MinusAssign => BinaryOp::Sub,
                            Token::StarAssign => BinaryOp::Mul,
                            Token::SlashAssign => BinaryOp::Div,
                            Token::PercentAssign => BinaryOp::Mod,
                            _ => unreachable!(),
                        };
                        
                        let value = Expression::Binary {
                            left: Box::new(expr.clone()),
                            op,
                            right: Box::new(rhs),
                        };
                        
                        Ok(Statement::Assignment {
                            target: expr,
                            value,
                        })
                    }
                    _ => Ok(Statement::Expression(expr))
                }
            }
        }
    }
    
    fn parse_const_statement(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'const'
        let (name, type_, value) = self.parse_const_or_static()?;
        Ok(Statement::Const { name, type_, value })
    }
    
    fn parse_static_statement(&mut self) -> Result<Statement, String> {
        self.advance(); // consume 'static'
        let mutable = if self.current_token() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };
        let (name, type_, value) = self.parse_const_or_static()?;
        Ok(Statement::Static { name, mutable, type_, value })
    }
    
    fn parse_for(&mut self) -> Result<Statement, String> {
        self.expect(Token::For)?;
        
        let variable = if let Token::Ident(name) = self.current_token() {
            let name = name.clone();
            self.advance();
            name
        } else {
            return Err("Expected variable name in for loop".to_string());
        };
        
        self.expect(Token::In)?;
        let iterable = self.parse_expression()?;
        
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        
        Ok(Statement::For { variable, iterable, body })
    }
    
    fn parse_go(&mut self) -> Result<Statement, String> {
        self.expect(Token::Go)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        
        Ok(Statement::Go { body })
    }
    
    fn parse_defer(&mut self) -> Result<Statement, String> {
        self.expect(Token::Defer)?;
        let stmt = self.parse_statement()?;
        
        Ok(Statement::Defer(Box::new(stmt)))
    }
    
    fn parse_let(&mut self) -> Result<Statement, String> {
        self.expect(Token::Let)?;
        
        let mutable = if self.current_token() == &Token::Mut {
            self.advance();
            true
        } else {
            false
        };
        
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected variable name".to_string());
        };
        
        let type_ = if self.current_token() == &Token::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(Token::Assign)?;
        let value = self.parse_expression()?;
        
        Ok(Statement::Let { name, mutable, type_, value })
    }
    
    fn parse_return(&mut self) -> Result<Statement, String> {
        self.advance();
        
        if matches!(self.current_token(), Token::RBrace | Token::Semicolon) {
            Ok(Statement::Return(None))
        } else {
            Ok(Statement::Return(Some(self.parse_expression()?)))
        }
    }
    
    fn parse_if(&mut self) -> Result<Statement, String> {
        self.expect(Token::If)?;
        
        let condition = self.parse_expression()?;
        
        self.expect(Token::LBrace)?;
        let then_block = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        
        let else_block = if self.current_token() == &Token::Else {
            self.advance();
            self.expect(Token::LBrace)?;
            let block = self.parse_block_statements()?;
            self.expect(Token::RBrace)?;
            Some(block)
        } else {
            None
        };
        
        Ok(Statement::If { condition, then_block, else_block })
    }
    
    fn parse_match(&mut self) -> Result<Statement, String> {
        self.expect(Token::Match)?;
        
        let value = self.parse_match_value()?;
        
        self.expect(Token::LBrace)?;
        
        let mut arms = Vec::new();
        while self.current_token() != &Token::RBrace {
            let pattern = self.parse_pattern_with_or()?;
            
            // Parse optional guard: if condition
            let guard = if self.current_token() == &Token::If {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            self.expect(Token::FatArrow)?;
            let body = self.parse_expression()?;
            
            arms.push(MatchArm { pattern, guard, body });
            
            if self.current_token() == &Token::Comma {
                self.advance();
            }
        }
        
        self.expect(Token::RBrace)?;
        
        Ok(Statement::Match { value, arms })
    }
    
    fn parse_pattern_with_or(&mut self) -> Result<Pattern, String> {
        let first = self.parse_pattern()?;
        
        // Check for OR patterns: pattern1 | pattern2
        if self.current_token() == &Token::Pipe {
            let mut patterns = vec![first];
            
            while self.current_token() == &Token::Pipe {
                self.advance();
                patterns.push(self.parse_pattern()?);
            }
            
            Ok(Pattern::Or(patterns))
        } else {
            Ok(first)
        }
    }
    
    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match self.current_token() {
            Token::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::LParen => {
                // Tuple pattern
                self.advance();
                let mut patterns = Vec::new();
                
                while self.current_token() != &Token::RParen {
                    patterns.push(self.parse_pattern()?);
                    
                    if self.current_token() == &Token::Comma {
                        self.advance();
                    } else {
                        break;  // No comma, must be end of tuple
                    }
                }
                
                self.expect(Token::RParen)?;
                Ok(Pattern::Tuple(patterns))
            }
            Token::BoolLiteral(b) => {
                let b = *b;
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(b)))
            }
            Token::IntLiteral(n) => {
                let n = *n;
                self.advance();
                Ok(Pattern::Literal(Literal::Int(n)))
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            Token::CharLiteral(c) => {
                let c = *c;
                self.advance();
                Ok(Pattern::Literal(Literal::Char(c)))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                
                // Check if it's an enum variant
                if self.current_token() == &Token::Dot {
                    self.advance();
                    if let Token::Ident(variant) = self.current_token() {
                        let variant = variant.clone();
                        self.advance();
                        
                        // Check for binding
                        let binding = if self.current_token() == &Token::LParen {
                            self.advance();
                            if let Token::Ident(b) = self.current_token() {
                                let b = b.clone();
                                self.advance();
                                self.expect(Token::RParen)?;
                                Some(b)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        Ok(Pattern::EnumVariant(format!("{}.{}", name, variant), binding))
                    } else {
                        Err("Expected variant name".to_string())
                    }
                } else {
                    Ok(Pattern::Identifier(name))
                }
            }
            _ => Err(format!("Expected pattern, got {:?}", self.current_token())),
        }
    }
    
    fn parse_loop(&mut self) -> Result<Statement, String> {
        self.expect(Token::Loop)?;
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        
        Ok(Statement::Loop { body })
    }
    
    fn parse_while(&mut self) -> Result<Statement, String> {
        self.expect(Token::While)?;
        let condition = self.parse_expression()?;
        
        self.expect(Token::LBrace)?;
        let body = self.parse_block_statements()?;
        self.expect(Token::RBrace)?;
        
        Ok(Statement::While { condition, body })
    }
    
    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_ternary_expression()
    }
    
    fn parse_ternary_expression(&mut self) -> Result<Expression, String> {
        let condition = self.parse_binary_expression(0)?;
        
        // Check for ternary operator: condition ? true_expr : false_expr
        if self.current_token() == &Token::Question {
            self.advance();
            let true_expr = self.parse_ternary_expression()?;  // Right-associative
            self.expect(Token::Colon)?;
            let false_expr = self.parse_ternary_expression()?;
            
            Ok(Expression::Ternary {
                condition: Box::new(condition),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
            })
        } else {
            Ok(condition)
        }
    }
    
    fn parse_match_value(&mut self) -> Result<Expression, String> {
        // Parse a non-struct-literal expression for match values
        // This is basically parse_binary_expression but without struct literal support
        let mut left = match self.current_token() {
            Token::LParen => {
                self.advance();
                
                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    return Ok(Expression::Tuple(vec![]));
                }
                
                let first_expr = self.parse_expression()?;
                
                // Check if it's a tuple (has comma) or just a parenthesized expression
                if self.current_token() == &Token::Comma {
                    let mut elements = vec![first_expr];
                    
                    while self.current_token() == &Token::Comma {
                        self.advance(); // consume comma
                        
                        // Allow trailing comma
                        if self.current_token() == &Token::RParen {
                            break;
                        }
                        
                        elements.push(self.parse_expression()?);
                    }
                    
                    self.expect(Token::RParen)?;
                    Expression::Tuple(elements)
                } else {
                    // Just a parenthesized expression
                    self.expect(Token::RParen)?;
                    first_expr
                }
            }
            Token::Ampersand => {
                // Handle & and &mut unary operators
                self.advance();
                let _mutable = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                // TODO: Properly handle &mut as separate UnaryOp variant
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Ref,
                    operand: Box::new(inner),
                }
            }
            Token::Star => {
                // Handle * dereference operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Deref,
                    operand: Box::new(inner),
                }
            }
            Token::Minus => {
                // Handle - negation operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(inner),
                }
            }
            Token::Bang => {
                // Handle ! not operator
                self.advance();
                let inner = self.parse_match_value()?;
                Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(inner),
                }
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                // Don't check for { here - just create the identifier
                // and continue to postfix operators
                Expression::Identifier(name)
            }
            _ => return self.parse_primary_expression(),
        };
        
        // Handle postfix operators (., [, etc.) before binary operators
        loop {
            match self.current_token() {
                Token::Dot => {
                    self.advance();
                    let field = if let Token::Ident(name) = self.current_token() {
                        let name = name.clone();
                        self.advance();
                        name
                    } else {
                        return Err("Expected field name after .".to_string());
                    };
                    left = Expression::FieldAccess {
                        object: Box::new(left),
                        field,
                    };
                }
                Token::LBracket => {
                    self.advance();
                    let index = Box::new(self.parse_expression()?);
                    self.expect(Token::RBracket)?;
                    left = Expression::Index {
                        object: Box::new(left),
                        index,
                    };
                }
                Token::LParen => {
                    // Function call
                    self.advance();
                    let mut arguments = Vec::new();
                    while self.current_token() != &Token::RParen {
                        let arg = self.parse_expression()?;
                        arguments.push((None, arg));
                        if self.current_token() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    left = Expression::Call {
                        function: Box::new(left),
                        arguments,
                    };
                }
                _ => break,
            }
        }
        
        // Handle binary operators
        while let Some((op, precedence)) = self.get_binary_op() {
            self.advance();
            let right = self.parse_binary_expression(precedence + 1)?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    fn parse_binary_expression(&mut self, min_precedence: u8) -> Result<Expression, String> {
        let mut left = self.parse_primary_expression()?;
        
        loop {
            // Check for pipe operator: value |> func
            if self.current_token() == &Token::PipeOp {
                self.advance();
                
                // Parse the right side (function to call)
                let func = self.parse_primary_expression()?;
                
                // Transform: left |> func becomes func(left)
                left = Expression::Call {
                    function: Box::new(func),
                    arguments: vec![(None, left)],  // No label for piped argument
                };
                continue;
            }
            
            // Check for channel send: ch <- value
            if self.current_token() == &Token::LeftArrow {
                self.advance();
                let value = self.parse_expression()?;
                left = Expression::ChannelSend {
                    channel: Box::new(left),
                    value: Box::new(value),
                };
                continue;
            }
            
            if let Some((op, precedence)) = self.get_binary_op() {
            if precedence < min_precedence {
                break;
            }
            
            self.advance();
            let right = self.parse_binary_expression(precedence + 1)?;
            
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
            } else {
                break;
            }
        }
        
        Ok(left)
    }
    
    fn get_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.current_token() {
            Token::Or => Some((BinaryOp::Or, 1)),
            Token::And => Some((BinaryOp::And, 2)),
            Token::Eq => Some((BinaryOp::Eq, 3)),
            Token::Ne => Some((BinaryOp::Ne, 3)),
            Token::Lt => Some((BinaryOp::Lt, 4)),
            Token::Le => Some((BinaryOp::Le, 4)),
            Token::Gt => Some((BinaryOp::Gt, 4)),
            Token::Ge => Some((BinaryOp::Ge, 4)),
            Token::Plus => Some((BinaryOp::Add, 5)),
            Token::Minus => Some((BinaryOp::Sub, 5)),
            Token::Star => Some((BinaryOp::Mul, 6)),
            Token::Slash => Some((BinaryOp::Div, 6)),
            Token::Percent => Some((BinaryOp::Mod, 6)),
            _ => None,
        }
    }
    
    fn parse_primary_expression(&mut self) -> Result<Expression, String> {
        let mut expr = match self.current_token() {
            Token::LeftArrow => {
                // Channel receive: <-ch
                self.advance();
                let channel = self.parse_primary_expression()?;
                Expression::ChannelRecv(Box::new(channel))
            }
            Token::Ampersand => {
                // Reference: &expr or &mut expr
                self.advance();
                let _is_mut = if self.current_token() == &Token::Mut {
                    self.advance();
                    true
                } else {
                    false
                };
                // For now, just parse the expression and wrap it
                // In full implementation, we'd track whether it's &mut
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Ref,
                    operand: Box::new(operand),
                }
            }
            Token::Star => {
                // Dereference: *expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Deref,
                    operand: Box::new(operand),
                }
            }
            Token::Minus => {
                // Negation: -expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                }
            }
            Token::Bang => {
                // Logical not: !expr
                self.advance();
                let operand = self.parse_primary_expression()?;
                Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                }
            }
            Token::Self_ => {
                // self keyword used in expressions
                self.advance();
                Expression::Identifier("self".to_string())
            }
            Token::IntLiteral(n) => {
                let n = *n;
                self.advance();
                Expression::Literal(Literal::Int(n))
            }
            Token::FloatLiteral(f) => {
                let f = *f;
                self.advance();
                Expression::Literal(Literal::Float(f))
            }
            Token::StringLiteral(s) => {
                let s = s.clone();
                self.advance();
                Expression::Literal(Literal::String(s))
            }
            Token::CharLiteral(c) => {
                let c = *c;
                self.advance();
                Expression::Literal(Literal::Char(c))
            }
            Token::InterpolatedString(parts) => {
                // Convert interpolated string to format! macro call
                let parts = parts.clone();
                self.advance();
                
                let mut format_string = String::new();
                let mut args = Vec::new();
                
                for part in parts {
                    match part {
                        crate::lexer::StringPart::Literal(lit) => {
                            format_string.push_str(&lit);
                        }
                        crate::lexer::StringPart::Expression(expr_str) => {
                            format_string.push_str("{}");
                            
                            // Parse the expression string
                            let mut expr_lexer = crate::lexer::Lexer::new(&expr_str);
                            let mut expr_tokens = Vec::new();
                            loop {
                                let tok = expr_lexer.next_token();
                                if tok == crate::lexer::Token::Eof {
                                    break;
                                }
                                expr_tokens.push(tok);
                            }
                            
                            // Parse the tokens into an expression
                            let mut expr_parser = Parser::new(expr_tokens);
                            if let Ok(expr) = expr_parser.parse_expression() {
                                args.push(expr);
                            }
                        }
                    }
                }
                
                // Create format! macro invocation
                let mut macro_args = vec![Expression::Literal(Literal::String(format_string))];
                macro_args.extend(args);
                
                Expression::MacroInvocation {
                    name: "format".to_string(),
                    args: macro_args,
                    delimiter: MacroDelimiter::Parens,
                }
            }
            Token::BoolLiteral(b) => {
                let b = *b;
                self.advance();
                Expression::Literal(Literal::Bool(b))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                
                // Check for struct literal
                if self.current_token() == &Token::LBrace {
                    self.advance();
                    let mut fields = Vec::new();
                    
                    while self.current_token() != &Token::RBrace {
                        if let Token::Ident(field_name) = self.current_token() {
                            let field_name = field_name.clone();
                            self.advance();
                            
                            let field_value = if self.current_token() == &Token::Colon {
                                // Regular syntax: field: value
                                self.advance();
                                self.parse_expression()?
                            } else {
                                // Shorthand syntax: field (implicitly field: field)
                                Expression::Identifier(field_name.clone())
                            };
                            
                            fields.push((field_name, field_value));
                            
                            if self.current_token() == &Token::Comma {
                                self.advance();
                            }
                        } else {
                            return Err("Expected field name in struct literal".to_string());
                        }
                    }
                    
                    self.expect(Token::RBrace)?;
                    Expression::StructLiteral { name, fields }
                } else {
                Expression::Identifier(name)
                }
            }
            Token::LParen => {
                self.advance();
                
                // Check for empty tuple ()
                if self.current_token() == &Token::RParen {
                    self.advance();
                    Expression::Tuple(vec![])
                } else {
                
                    let first_expr = self.parse_expression()?;
                    
                    // Check if this is a tuple or just a parenthesized expression
                    if self.current_token() == &Token::Comma {
                        // It's a tuple
                        let mut exprs = vec![first_expr];
                        
                        while self.current_token() == &Token::Comma {
                            self.advance();
                            // Allow trailing comma
                            if self.current_token() == &Token::RParen {
                                break;
                            }
                            exprs.push(self.parse_expression()?);
                        }
                        
                self.expect(Token::RParen)?;
                        Expression::Tuple(exprs)
                    } else {
                        // Just a parenthesized expression
                        self.expect(Token::RParen)?;
                        first_expr
                    }
                }
            }
            Token::Match => {
                // Match expression
                self.advance();
                // Parse the value to match on, but don't allow struct literals here
                // (since we need to see the { for the match arms)
                let value = Box::new(self.parse_match_value()?);
                
                self.expect(Token::LBrace)?;
                
                let mut arms = Vec::new();
                while self.current_token() != &Token::RBrace {
                    let pattern = self.parse_pattern_with_or()?;
                    
                    // Parse optional guard: if condition
                    let guard = if self.current_token() == &Token::If {
                        self.advance();
                        Some(self.parse_expression()?)
                    } else {
                        None
                    };
                    
                    self.expect(Token::FatArrow)?;
                    let body = self.parse_expression()?;
                    
                    arms.push(MatchArm { pattern, guard, body });
                    
                    if self.current_token() == &Token::Comma {
                        self.advance();
                    }
                }
                
                self.expect(Token::RBrace)?;
                
                // Convert match arms into a match expression  
                // For now, wrap in a block expression
                let match_stmt = Statement::Match {
                    value: *value,
                    arms,
                };
                Expression::Block(vec![match_stmt])
            }
            Token::Pipe => {
                // Closure: |params| body
                self.advance();
                let mut parameters = Vec::new();
                
                while self.current_token() != &Token::Pipe {
                    // Handle patterns like &x, &mut x, or just x
                    let param_name = match self.current_token() {
                        Token::Ampersand => {
                            self.advance();
                            // Skip optional 'mut'
                            if self.current_token() == &Token::Mut {
                                self.advance();
                            }
                            // Get the identifier
                            if let Token::Ident(name) = self.current_token() {
                                let n = name.clone();
                                self.advance();
                                n
                            } else {
                                return Err("Expected identifier after & in closure parameter".to_string());
                            }
                        }
                        Token::Ident(name) => {
                            let n = name.clone();
                            self.advance();
                            n
                        }
                        Token::Underscore => {
                            self.advance();
                            "_".to_string()
                        }
                        _ => {
                            return Err("Expected parameter name in closure".to_string());
                        }
                    };
                    
                    parameters.push(param_name);
                    
                    if self.current_token() == &Token::Comma {
                        self.advance();
                    }
                }
                
                self.expect(Token::Pipe)?;
                let body = Box::new(self.parse_expression()?);
                
                Expression::Closure { parameters, body }
            }
            Token::If => {
                // If expression: if cond { ... } else { ... }
                self.advance(); // consume 'if'
                // Use parse_match_value to avoid struct literal ambiguity
                let condition = Box::new(self.parse_match_value()?);
                
                self.expect(Token::LBrace)?;
                let then_block = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                
                let else_block = if self.current_token() == &Token::Else {
                    self.advance();
                    self.expect(Token::LBrace)?;
                    let block = self.parse_block_statements()?;
                    self.expect(Token::RBrace)?;
                    Some(block)
                } else {
                    None
                };
                
                // Convert to expression by wrapping in a block with an if statement
                // that returns the value
                let if_stmt = Statement::If {
                    condition: *condition,
                    then_block,
                    else_block,
                };
                
                Expression::Block(vec![if_stmt])
            }
            Token::Unsafe => {
                // Unsafe block: unsafe { ... }
                self.advance(); // consume 'unsafe'
                self.expect(Token::LBrace)?;
                let body = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                Expression::Block(body)
            }
            Token::LBrace => {
                // Block expression: { ... }
                self.advance(); // consume '{'
                let body = self.parse_block_statements()?;
                self.expect(Token::RBrace)?;
                Expression::Block(body)
            }
            _ => return Err(format!("Unexpected token in expression: {:?}", self.current_token())),
        };
        
        // Handle postfix operators
        loop {
            expr = match self.current_token() {
                Token::Dot => {
                    // Peek ahead to check for .await
                    if self.peek(1) == Some(&Token::Await) {
                        self.advance(); // consume '.'
                        self.advance(); // consume 'await'
                        Expression::Await(Box::new(expr))
                    } else {
                    self.advance();
                    if let Token::Ident(field) = self.current_token() {
                        let field = field.clone();
                        self.advance();
                        
                        if self.current_token() == &Token::LParen {
                            // Method call
                            self.advance();
                            let arguments = self.parse_arguments()?;
                            self.expect(Token::RParen)?;
                            Expression::MethodCall {
                                object: Box::new(expr),
                                method: field,
                                arguments,
                            }
                        } else {
                            // Field access
                            Expression::FieldAccess {
                                object: Box::new(expr),
                                field,
                            }
                        }
                    } else {
                        return Err("Expected field or method name".to_string());
                        }
                    }
                }
                Token::LParen => {
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect(Token::RParen)?;
                    Expression::Call {
                        function: Box::new(expr),
                        arguments,
                    }
                }
                Token::Question => {
                    // Disambiguate between TryOp (?) and ternary (? :)
                    // If next token could start a ternary expression, don't treat this as TryOp
                    if let Some(next_tok) = self.peek(1) {
                        match next_tok {
                            // These tokens can start a ternary true-branch
                            Token::IntLiteral(_) | Token::FloatLiteral(_) | Token::StringLiteral(_) |
                            Token::BoolLiteral(_) | Token::Ident(_) | Token::LParen |
                            Token::Minus | Token::Not | Token::Ampersand | Token::Star => {
                                // Likely ternary, don't consume as TryOp
                                break;
                            }
                            _ => {
                                // Likely TryOp
                    self.advance();
                    Expression::TryOp(Box::new(expr))
                }
                        }
                    } else {
                        // No next token, treat as TryOp
                        self.advance();
                        Expression::TryOp(Box::new(expr))
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(Token::RBracket)?;
                    Expression::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                    }
                }
                Token::DotDot | Token::DotDotEq => {
                    let inclusive = self.current_token() == &Token::DotDotEq;
                    self.advance();
                    let end = self.parse_primary_expression()?;
                    Expression::Range {
                        start: Box::new(expr),
                        end: Box::new(end),
                        inclusive,
                    }
                }
                Token::As => {
                    self.advance();
                    let type_ = self.parse_type()?;
                    Expression::Cast {
                        expr: Box::new(expr),
                        type_,
                    }
                }
                Token::Bang => {
                    // Macro invocation: name!(...) or name![...] or name!{...}
                    if let Expression::Identifier(name) = expr {
                        self.advance(); // consume '!'
                        
                        let (delimiter, end_token) = match self.current_token() {
                            Token::LParen => (MacroDelimiter::Parens, Token::RParen),
                            Token::LBracket => (MacroDelimiter::Brackets, Token::RBracket),
                            Token::LBrace => (MacroDelimiter::Braces, Token::RBrace),
                            _ => return Err("Expected (, [, or { after macro name!".to_string()),
                        };
                        
                        self.advance(); // consume opening delimiter
                        
                        let mut args = Vec::new();
                        while self.current_token() != &end_token {
                            args.push(self.parse_expression()?);
                            
                            if self.current_token() == &Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        
                        self.expect(end_token)?;
                        
                        Expression::MacroInvocation {
                            name,
                            args,
                            delimiter,
                        }
                    } else {
                        // Not a macro invocation, break out of postfix loop
                        break;
                    }
                }
                _ => break,
            };
        }
        
        Ok(expr)
    }
    
    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset)
    }
    
    fn parse_arguments(&mut self) -> Result<Vec<(Option<String>, Expression)>, String> {
        let mut args = Vec::new();
        
        while self.current_token() != &Token::RParen {
            // Check for labeled argument: name: expr
            let label = if let Token::Ident(name) = self.current_token() {
                if self.peek(1) == Some(&Token::Colon) {
                    let label = name.clone();
                    self.advance(); // consume identifier
                    self.advance(); // consume colon
                    Some(label)
                } else {
                    None
                }
            } else {
                None
            };
            
            let expr = self.parse_expression()?;
            args.push((label, expr));
            
            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        Ok(args)
    }
    
    // Helper: Extract a name from a pattern for backward compatibility
    fn pattern_to_name(pattern: &Pattern) -> String {
        match pattern {
            Pattern::Identifier(name) => name.clone(),
            Pattern::Tuple(patterns) => {
                // For tuple patterns, generate a name like "_tuple_param"
                format!("_tuple_{}", patterns.len())
            }
            Pattern::EnumVariant(name, _) => name.clone(),
            Pattern::Wildcard => "_".to_string(),
            Pattern::Literal(_) => "_lit".to_string(),
            Pattern::Or(patterns) => {
                // Use the first pattern's name
                if let Some(first) = patterns.first() {
                    Self::pattern_to_name(first)
                } else {
                    "_or_pattern".to_string()
                }
            }
        }
    }
}

