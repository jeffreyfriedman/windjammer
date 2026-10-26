// Parser - Windjammer Language Parser
//
// This file contains the complete parser for Windjammer. It is organized into the following sections:
//
// 1. AST TYPES (lines ~3-340)
//    - Type, TypeParam, Parameter, FunctionDecl, StructDecl, EnumDecl, TraitDecl, ImplBlock
//    - Expression, Statement, Pattern, Item, Program
//
// 2. PARSER CORE (lines ~344-400)
//    - Parser struct
//    - Basic utilities: new(), current_token(), advance(), expect(), peek()
//    - Helper: type_to_string()
//
// 3. TOP-LEVEL PARSING (lines ~400-700)
//    - parse() - main entry point
//    - parse_item() - dispatches to item parsers
//    - parse_const_or_static()
//    - parse_use()
//    - parse_decorator() and parse_decorator_arguments()
//
// 4. ITEM PARSING (lines ~700-1500)
//    - parse_impl() - impl blocks with generics and trait impls
//    - parse_trait() - trait definitions
//    - parse_function() - function declarations
//    - parse_parameters()
//    - parse_struct() - struct definitions
//    - parse_enum() - enum definitions with generics
//    - parse_type_params() - generic type parameters with bounds
//    - parse_where_clause() - where clauses
//
// 5. STATEMENT PARSING (lines ~1500-1900)
//    - parse_block_statements()
//    - parse_statement() - dispatches to statement parsers
//    - parse_const_statement(), parse_static_statement()
//    - parse_let(), parse_return()
//    - parse_if(), parse_match()
//    - parse_for(), parse_loop(), parse_while()
//    - parse_go(), parse_defer()
//
// 6. PATTERN PARSING (lines ~1900-2000)
//    - parse_pattern_with_or() - OR patterns
//    - parse_pattern() - all pattern types including enum variants
//
// 7. EXPRESSION PARSING (lines ~2000-2800)
//    - parse_expression() - entry point
//    - parse_ternary_expression() - ternary operator
//    - parse_match_value() - match value with special handling
//    - parse_binary_expression() - operator precedence climbing
//    - get_binary_op() - operator precedence table
//    - parse_primary_expression() - literals, identifiers, calls, etc.
//    - parse_postfix_expression() - method calls, field access, indexing, turbofish
//    - parse_arguments()
//    - parse_closure()
//
// 8. TYPE PARSING (lines ~2800+)
//    - parse_type() - all type variants
//
// TODO: Split this into modules:
//   - parser/mod.rs - Parser struct and utilities
//   - parser/types.rs - Type parsing
//   - parser/patterns.rs - Pattern parsing
//   - parser/expressions.rs - Expression parsing
//   - parser/statements.rs - Statement parsing
//   - parser/items.rs - Top-level item parsing

use crate::lexer::Token;
use std::sync::Arc;

// Import all AST types from the new parser::ast module
pub use crate::parser::ast::*;

// ============================================================================
// SECTION 2: PARSER CORE
// ============================================================================

/// A structured diagnostic emitted during parsing.
#[derive(Debug, Clone)]
pub struct ParseWarning {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// When true, this diagnostic is an error that should halt compilation.
    pub is_error: bool,
}

pub struct Parser {
    pub(crate) tokens: Vec<crate::lexer::TokenWithLocation>,
    pub(crate) position: usize,
    pub(crate) filename: String,
    #[allow(dead_code)]
    pub(crate) source: String,
    pub(crate) warnings: Vec<ParseWarning>,
    /// True when parsing inside an `extern fn` declaration (FFI boundary).
    /// Suppresses W0010 warnings since FFI signatures must match Rust types exactly.
    pub(crate) in_extern_fn: bool,
    /// True while parsing a `|>` stage: `_` is then a placeholder expression.
    pub(crate) in_pipe_stage: bool,
    /// Number of `_` placeholders produced in the current pipe stage.
    pub(crate) pipe_placeholders: Vec<usize>,
    /// `_` shorthand-closure frames, one per call argument being parsed (ADR-0018, `parser/shorthand_closure.rs`).
    pub(crate) shorthand_frames: Vec<crate::parser::shorthand_closure::ShorthandFrame>,
    /// Explicit closures being parsed (a `_` inside one, in the same argument, is an error).
    pub(crate) closure_depth: usize,
    /// Named-argument lowering table (second parse pass only; `parser/named_args.rs`).
    pub(crate) call_table: Option<crate::parser::named_args::CallTable>,
    /// A labelled call argument (`f(x: 1)`) was parsed.
    pub(crate) saw_label: bool,
    // Arena allocators for AST nodes (no recursive Drop: Windows stack-overflow fix).
    //
    // SOUNDNESS: nodes are handed out as `'static` references, so the type system cannot tie them to this
    // arena bundle. Instead, `parse()` *seals* the bundle: it moves the `Arc` into the returned `Program`
    // (as its `AstOwner`) and installs a fresh bundle here. The arenas live exactly as long as the last
    // program (or merged program) that borrows them; the parser can be dropped at any time.
    pub(crate) arenas: Arc<AstArenas>,
    /// `AstArenas` is `Sync` only once sealed; a parser allocates through `&self`, so it must not be shared.
    _not_sync: std::marker::PhantomData<std::cell::Cell<()>>,
    /// Errors already reported and recovered from (statement-level recovery); `parse()` fails
    /// with all of them plus the final one.
    pub(crate) recovered_errors: Vec<String>,
    /// Statement-level error recovery is only on inside `parse()`, which reports the errors.
    pub(crate) recovering: bool,
    /// True while parsing the operand of a prefix operator (stops before `as`).
    pub(crate) in_unary_operand: bool,
    /// Node spans (off unless `enable_span_log()` was called).
    pub(crate) span_log: std::cell::RefCell<Option<crate::parser::span_log::SpanLog>>,
}

impl Parser {
    /// Index of the next unconsumed token (tests use it to check a parse consumed its input).
    pub fn token_position(&self) -> usize {
        self.position
    }

    /// 1-based (line, column) of the token the parser stopped at (for error reporting).
    pub fn error_position(&self) -> Option<(usize, usize)> {
        self.tokens
            .get(self.position)
            .or_else(|| self.tokens.last())
            .map(|t| (t.src_line, t.src_column))
    }

    /// Check if there was a newline before the current token (for ASI - Automatic Semicolon Insertion)
    pub(crate) fn had_newline_before_current(&self) -> bool {
        if self.position == 0 {
            return false; // No previous token
        }

        let prev_token = self.tokens.get(self.position - 1);
        let curr_token = self.tokens.get(self.position);

        match (prev_token, curr_token) {
            (Some(prev), Some(curr)) => {
                // If the line number changed, there was a newline
                curr.line > prev.line
            }
            _ => false,
        }
    }
    pub fn new(tokens: Vec<crate::lexer::TokenWithLocation>) -> Self {
        Parser {
            tokens,
            position: 0,
            filename: String::new(),
            source: String::new(),
            warnings: Vec::new(),
            in_extern_fn: false,
            in_pipe_stage: false,
            pipe_placeholders: Vec::new(),
            shorthand_frames: Vec::new(),
            closure_depth: 0,
            call_table: None,
            saw_label: false,
            arenas: Arc::new(AstArenas::new()),
            _not_sync: std::marker::PhantomData,
            recovered_errors: Vec::new(),
            recovering: false,
            in_unary_operand: false,
            span_log: std::cell::RefCell::new(None),
        }
    }

    pub fn new_with_source(
        tokens: Vec<crate::lexer::TokenWithLocation>,
        filename: String,
        source: String,
    ) -> Self {
        Parser {
            tokens,
            position: 0,
            filename,
            source,
            warnings: Vec::new(),
            in_extern_fn: false,
            in_pipe_stage: false,
            pipe_placeholders: Vec::new(),
            shorthand_frames: Vec::new(),
            closure_depth: 0,
            call_table: None,
            saw_label: false,
            arenas: Arc::new(AstArenas::new()),
            _not_sync: std::marker::PhantomData,
            recovered_errors: Vec::new(),
            recovering: false,
            in_unary_operand: false,
            span_log: std::cell::RefCell::new(None),
        }
    }

    /// `line L, column C` of the current token (the last token when past the end).
    pub(crate) fn pos_desc(&self) -> String {
        match self.tokens.get(self.position).or_else(|| self.tokens.last()) {
            Some(t) => format!("line {}, column {}", t.src_line, t.src_column),
            None => "end of input".to_string(),
        }
    }

    /// Guarantee `line L, column C` in a parse error (many call sites build bare messages).
    /// The position is that of the token the parser stopped at.
    pub(crate) fn with_position(&self, message: String) -> String {
        if message.contains("line ") && message.contains(", column ") {
            message
        } else {
            format!("{} (at {})", message, self.pos_desc())
        }
    }

    /// Position + diagnostics watermark for speculative parsing; pair with `rewind`.
    pub(crate) fn mark(&self) -> (usize, usize) {
        (self.position, self.recovered_errors.len())
    }

    /// Undo everything since `mark` (tokens consumed and errors recovered inside a speculation).
    pub(crate) fn rewind(&mut self, mark: (usize, usize)) {
        self.position = mark.0;
        self.recovered_errors.truncate(mark.1);
    }

    /// One message per `Token::Invalid` in the stream, each with line and column.
    pub fn lexical_errors(&self) -> Vec<String> {
        self.tokens
            .iter()
            .filter_map(|t| match &t.token {
                Token::Invalid(m) => Some(format!(
                    "Lexical error: {} (at line {}, column {})",
                    m, t.line, t.column
                )),
                _ => None,
            })
            .collect()
    }

    /// Skip to a safe resume point after a failed statement that began at `start`: the next
    /// token that starts a new line with all brackets opened since `start` closed, or a `;`
    /// (consumed), or the `}` that ends the enclosing block (not consumed). Always makes progress.
    pub(crate) fn recover_to_statement_boundary(&mut self, start: usize) {
        let mut depth: i32 = 0;
        let fail_at = self.position.max(start);
        let mut i = start;
        while i < self.tokens.len() {
            let t = &self.tokens[i].token;
            match t {
                Token::Eof => break,
                Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
                Token::RParen | Token::RBracket | Token::RBrace => {
                    if depth == 0 && i >= fail_at && *t == Token::RBrace {
                        break; // end of the enclosing block
                    }
                    depth -= 1;
                }
                Token::Semicolon if depth <= 0 && i >= fail_at => {
                    i += 1;
                    break;
                }
                _ => {}
            }
            let next_starts_line = self
                .tokens
                .get(i + 1)
                .is_some_and(|n| n.line > self.tokens[i].line);
            if i >= fail_at && depth <= 0 && next_starts_line && i + 1 > start {
                i += 1;
                break;
            }
            i += 1;
        }
        self.position = i.max(start + 1).min(self.tokens.len().saturating_sub(1));
    }

    /// Skip to the next top-level item after a failed item that began at `start`.
    fn recover_to_item_boundary(&mut self, start: usize) {
        let fail_at = self.position.max(start);
        let mut depth: i32 = 0;
        let mut i = start;
        while i < self.tokens.len() {
            let t = &self.tokens[i].token;
            match t {
                Token::Eof => break,
                Token::LBrace => depth += 1,
                Token::RBrace => {
                    depth -= 1;
                    if depth <= 0 && i >= fail_at {
                        i += 1;
                        break;
                    }
                }
                Token::Fn
                | Token::Struct
                | Token::Enum
                | Token::Trait
                | Token::Impl
                | Token::Use
                | Token::Mod
                | Token::Pub
                | Token::Const
                | Token::Static
                | Token::Extern
                | Token::Decorator(_)
                    if depth <= 0
                        && i > start
                        && i >= fail_at
                        && self.tokens[i].line > self.tokens[i - 1].line =>
                {
                    break;
                }
                _ => {}
            }
            i += 1;
        }
        self.position = i.max(start + 1).min(self.tokens.len().saturating_sub(1));
    }

    pub fn warnings(&self) -> &[ParseWarning] {
        &self.warnings
    }

    pub(crate) fn emit_warning(
        &mut self,
        message: String,
        file: Option<String>,
        line: Option<usize>,
        column: Option<usize>,
    ) {
        if let (Some(l), Some(c)) = (line, column) {
            crate::error::span_check::assert_location("parser diagnostic", l, c);
        }
        self.warnings.push(ParseWarning {
            message,
            file,
            line,
            column,
            is_error: false,
        });
    }

    pub(crate) fn emit_error_diagnostic(
        &mut self,
        message: String,
        file: Option<String>,
        line: Option<usize>,
        column: Option<usize>,
    ) {
        if let (Some(l), Some(c)) = (line, column) {
            crate::error::span_check::assert_location("parser diagnostic", l, c);
        }
        self.warnings.push(ParseWarning {
            message,
            file,
            line,
            column,
            is_error: true,
        });
    }

    /// A sub-parser (string interpolation) that allocates into `parent`'s arena bundle, so its nodes are owned
    /// by the `Program` the parent's `parse()` returns. Only used synchronously on the parent's thread.
    pub(crate) fn new_sharing_arenas(tokens: Vec<crate::lexer::TokenWithLocation>, parent: &Parser) -> Self {
        let mut p = Parser::new(tokens);
        p.arenas = parent.arenas.clone();
        p
    }

    /// Ledger of the arena bundle currently being filled (the one the next `parse()` will seal).
    pub fn arena_ledger(&self) -> ArenaLedger {
        self.arenas.ledger()
    }

    /// Hand the filled arena bundle to the caller and start a fresh one.
    fn seal_arenas(&mut self) -> AstOwner {
        AstOwner::of(std::mem::replace(&mut self.arenas, Arc::new(AstArenas::new())))
    }

    /// Mutation-test hook: re-introduce the old behaviour (arenas are never freed) for `program`'s storage.
    #[doc(hidden)]
    pub fn leak_arenas_for_mutation_test(&self, program: &Program<'static>) {
        std::mem::forget(program.owner().clone());
    }

    /// Allocate an expression in the arena.
    /// SAFETY of the lifetime extension: the reference points into `self.arenas`, which `parse()` moves into
    /// the resulting `Program`'s `AstOwner`; the nodes are only observable through that program (or a program
    /// built with `Program::from_items_of`), which keeps the arena alive. Boxed arena chunks never move.
    pub(crate) fn alloc_expr<'ast>(&self, expr: Expression<'static>) -> &'ast Expression<'ast> {
        unsafe {
            let ptr = self.arenas.expr.alloc(expr);
            self.record_span(ptr as *const Expression<'static> as usize);
            std::mem::transmute(ptr)
        }
    }

    /// Allocate a statement in the arena (same argument as `alloc_expr`).
    pub(crate) fn alloc_stmt<'ast>(&self, stmt: Statement<'static>) -> &'ast Statement<'ast> {
        unsafe {
            let ptr = self.arenas.stmt.alloc(stmt);
            self.record_span(ptr as *const Statement<'static> as usize);
            std::mem::transmute(ptr)
        }
    }

    /// Allocate a pattern in the arena (same argument as `alloc_expr`).
    pub(crate) fn alloc_pattern<'ast>(&self, pattern: Pattern<'static>) -> &'ast Pattern<'ast> {
        unsafe {
            let ptr = self.arenas.pattern.alloc(pattern);
            std::mem::transmute(ptr)
        }
    }

    pub(crate) fn current_token(&self) -> &Token {
        self.tokens
            .get(self.position)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    /// Get the current token's location for source mapping
    pub(crate) fn current_location(&self) -> Option<crate::source_map::Location> {
        self.tokens
            .get(self.position)
            .map(|t| crate::source_map::Location {
                file: std::path::PathBuf::from(&self.filename),
                line: t.line,
                column: t.column,
            })
    }

    pub(crate) fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    pub(crate) fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.current_token() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, got {:?} (at {})",
                expected,
                self.current_token(),
                self.pos_desc()
            ))
        }
    }

    /// Handle closing of nested generic types where `>>` should be treated as two `>` tokens
    pub(crate) fn expect_gt_or_split_shr(&mut self) -> Result<bool, String> {
        match self.current_token() {
            Token::Gt => {
                self.advance();
                Ok(false) // Not a split, normal >
            }
            Token::Shr => {
                // Split >> into two > tokens
                // We consume one > and insert another > after it
                // The two halves partition the `>>` span: `>` at [s, s+1) and `>` at [s+1, s+2).
                let shr = self.tokens[self.position].clone();
                let mut first = shr.clone();
                first.token = Token::Gt;
                first.byte_end = shr.byte_start + 1;
                first.end_line = shr.src_line;
                first.end_column = shr.src_column + 1;
                let mut second = shr;
                second.token = Token::Gt;
                second.byte_start += 1;
                second.column += 1;
                second.src_column += 1;

                self.tokens[self.position] = first;
                self.tokens.insert(self.position + 1, second);

                // Advance to consume the first >
                self.advance();

                Ok(true) // Was a split >>
            }
            _ => Err(format!(
                "Expected '>' or '>>', got {:?} (at {})",
                self.current_token(),
                self.pos_desc()
            )),
        }
    }

    // ========================================================================
    // SECTION 3: TOP-LEVEL PARSING
    // ========================================================================

    pub fn parse(&mut self) -> Result<Program<'static>, String> {
        let mut items = Vec::new();

        // Lexical errors first, each with line:column. Parsing a stream with invalid tokens
        // would only add cascading noise.
        let lexical = self.lexical_errors();
        if !lexical.is_empty() {
            return Err(lexical.join("\n"));
        }

        self.recovering = true;
        const MAX_ERRORS: usize = 25;
        let mut errors: Vec<String> = Vec::new();
        while self.current_token() != &Token::Eof {
            let start = self.position;
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    errors.append(&mut self.recovered_errors);
                    errors.push(self.with_position(e));
                    if errors.len() >= MAX_ERRORS {
                        break;
                    }
                    self.recover_to_item_boundary(start);
                }
            }
        }
        self.recovering = false;
        errors.append(&mut self.recovered_errors);
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }

        let program = Program::new(items);
        // ADR-0018 named arguments / defaults: parse again with the file's callee table (parser/named_args.rs).
        if self.call_table.is_none() {
            if let Some(table) = crate::parser::named_args::call_table(&program, self.saw_label) {
                self.call_table = Some(table);
                self.position = 0;
                self.warnings.clear();
                return self.parse();
            }
        }
        self.emit_surface_syntax_warnings();
        Ok(Program::with_owner(program.items, self.seal_arenas()))
    }

    pub(crate) fn parse_item(&mut self) -> Result<Item<'static>, String> {
        // Skip leading blank lines so indented r#"\\n    @derive(...)"# still attaches decorators.
        while matches!(self.current_token(), Token::Newline) {
            self.advance();
        }

        // Collect doc comments (/// lines) that appear before the item
        let mut doc_lines = Vec::new();
        while let Token::DocComment(content) = self.current_token() {
            doc_lines.push(content.clone());
            self.advance();
        }
        let mut doc_comment = if doc_lines.is_empty() {
            None
        } else {
            Some(doc_lines.join("\n"))
        };

        while matches!(self.current_token(), Token::Newline) {
            self.advance();
        }

        // Check for decorators
        let mut decorators = Vec::new();
        while let Token::Decorator(_) = self.current_token() {
            decorators.push(self.parse_decorator()?);
        }

        while matches!(self.current_token(), Token::Newline) {
            self.advance();
        }

        // Doc comments may appear after @derive and before the item (e.g. @derive(Clone)\n/// doc\npub struct S)
        let mut doc_lines_after = Vec::new();
        while let Token::DocComment(content) = self.current_token() {
            doc_lines_after.push(content.clone());
            self.advance();
        }
        if !doc_lines_after.is_empty() {
            doc_comment = Some(doc_lines_after.join("\n"));
        }

        while matches!(self.current_token(), Token::Newline) {
            self.advance();
        }

        // Check for pub keyword (for module functions)
        let is_pub = if self.current_token() == &Token::Pub {
            self.advance();
            true
        } else {
            false
        };

        match self.current_token() {
            Token::Fn => {
                self.advance(); // Consume the Fn token
                let mut func = self.parse_function()?;
                func.decorators = decorators.clone();
                func.is_pub = is_pub;
                func.doc_comment = doc_comment;
                // Check if @async decorator is present
                if decorators.iter().any(|d| d.name == "async") {
                    func.is_async = true;
                }
                Ok(Item::Function {
                    decl: func,
                    location: self.current_location(),
                })
            }
            Token::Async => {
                self.advance();
                self.expect(Token::Fn)?;
                let mut func = self.parse_function()?;
                func.is_async = true;
                func.is_pub = is_pub;
                func.decorators = decorators;
                func.doc_comment = doc_comment;
                Ok(Item::Function {
                    decl: func,
                    location: self.current_location(),
                })
            }
            Token::Struct => {
                self.advance();
                let mut struct_decl = self.parse_struct(false)?;
                struct_decl.decorators = decorators;
                struct_decl.is_pub = is_pub;
                struct_decl.doc_comment = doc_comment;
                Ok(Item::Struct {
                    decl: struct_decl,
                    location: self.current_location(),
                })
            }
            Token::Enum => {
                self.advance();
                let mut enum_decl = self.parse_enum()?;
                enum_decl.is_pub = is_pub;
                enum_decl.doc_comment = doc_comment;
                Ok(Item::Enum {
                    decl: enum_decl,
                    location: self.current_location(),
                })
            }
            Token::Trait => {
                self.advance();
                let mut trait_decl = self.parse_trait()?;
                trait_decl.doc_comment = doc_comment;
                Ok(Item::Trait {
                    decl: trait_decl,
                    location: self.current_location(),
                })
            }
            Token::Impl => {
                self.advance();
                let mut impl_block = self.parse_impl(false)?;
                impl_block.decorators = decorators;
                Ok(Item::Impl {
                    block: impl_block,
                    location: self.current_location(),
                })
            }
            Token::Const => {
                self.advance();
                let (name, type_, value) = self.parse_const_or_static()?;
                Ok(Item::Const {
                    name,
                    is_pub,
                    type_,
                    value,
                    location: self.current_location(),
                })
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
                Ok(Item::Static {
                    name,
                    mutable,
                    type_,
                    value,
                    location: self.current_location(),
                })
            }
            Token::Extern => {
                // `extern let` (GPU) | `extern struct` / `extern impl` (FFI types) | `extern fn` (FFI)
                if self.peek(1) == Some(&Token::Let) {
                    self.advance(); // consume extern
                    self.advance(); // consume let

                    let name = if let Token::Ident(n) = self.current_token() {
                        let n = n.clone();
                        self.advance();
                        n
                    } else {
                        return Err("Expected variable name after extern let".to_string());
                    };

                    self.expect(Token::Colon)?;
                    let type_ = self.parse_type()?;

                    // Semicolon optional (ASI)
                    if self.current_token() == &Token::Semicolon {
                        self.advance();
                    }

                    Ok(Item::ExternLet {
                        name,
                        type_,
                        decorators,
                        is_pub,
                        location: self.current_location(),
                    })
                } else {
                    self.advance(); // consume Extern
                    match self.current_token() {
                        Token::Struct => {
                            self.advance();
                            let mut struct_decl = self.parse_struct(true)?;
                            struct_decl.decorators = decorators;
                            struct_decl.is_pub = is_pub;
                            struct_decl.doc_comment = doc_comment;
                            Ok(Item::Struct {
                                decl: struct_decl,
                                location: self.current_location(),
                            })
                        }
                        Token::Impl => {
                            self.advance();
                            let mut impl_block = self.parse_impl(true)?;
                            impl_block.decorators = decorators;
                            Ok(Item::Impl {
                                block: impl_block,
                                location: self.current_location(),
                            })
                        }
                        Token::Fn => {
                            self.expect(Token::Fn)?; // Expect fn after extern
                            self.in_extern_fn = true;
                            let mut func = self.parse_function()?;
                            self.in_extern_fn = false;
                            func.is_extern = true; // Mark as extern function
                            func.is_pub = is_pub;
                            func.decorators = decorators;
                            func.doc_comment = doc_comment;
                            Ok(Item::Function {
                                decl: func,
                                location: self.current_location(),
                            })
                        }
                        _ => {
                            Err("expected `let`, `struct`, `impl`, or `fn` after `extern`"
                                .to_string())
                        }
                    }
                }
            }
            Token::Use => {
                self.advance(); // consume 'use'
                let (path, alias) = self.parse_use()?;
                // Consume optional semicolon (ASI - automatic semicolon insertion)
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                }
                Ok(Item::Use {
                    path,
                    alias,
                    is_pub, // THE WINDJAMMER WAY: Track pub use for re-exports
                    location: self.current_location(),
                })
            }
            Token::Ident(name)
                if name == "bound"
                    && matches!(self.peek(1), Some(Token::Ident(_)))
                    && self.peek(2) == Some(&Token::Assign) =>
            {
                self.advance(); // consume soft-keyword `bound`
                self.parse_bound_alias()
            }
            Token::Mod => {
                self.advance(); // consume 'mod'
                let (name, items, _) = self.parse_mod()?;
                Ok(Item::Mod {
                    name,
                    items,
                    is_public: is_pub,
                    location: self.current_location(),
                })
            }
            Token::Type => {
                self.advance(); // consume 'type'
                let name = if let Token::Ident(n) = self.current_token() {
                    let name = n.clone();
                    self.advance();
                    name
                } else {
                    return Err("Expected type alias name".to_string());
                };
                let type_params = self.parse_type_params()?;
                self.expect(Token::Assign)?;
                let target = self.parse_type()?;
                // Semicolon optional (ASI)
                if self.current_token() == &Token::Semicolon {
                    self.advance();
                }
                Ok(Item::TypeAlias {
                    name,
                    type_params,
                    target,
                    is_pub,
                    location: self.current_location(),
                })
            }
            Token::Ident(_) if self.peek(1) == Some(&Token::Bang) => {
                let expr = self.parse_expression()?;
                Ok(Item::Macro {
                    doc_comment,
                    expr,
                    location: self.current_location(),
                })
            }
            _ => Err(format!(
                "Unexpected token: {:?} (at {}); expected an item: one of `fn`, `struct`, `enum`, `trait`, `impl`, `use`, `mod`, `const`, `static`, `type`, `extern`, `pub`, `async`, a decorator such as `@derive(..)`, or a doc comment",
                self.current_token(),
                self.pos_desc()
            )),
        }
    }

    fn parse_bound_alias(&mut self) -> Result<Item<'static>, String> {
        // bound Name = Trait + Trait + ...
        let name = if let Token::Ident(n) = self.current_token() {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected bound alias name".to_string());
        };

        self.expect(Token::Assign)?;

        // Parse trait list: Trait + Trait + ...
        let mut traits = Vec::new();
        loop {
            if let Token::Ident(trait_name) = self.current_token() {
                traits.push(trait_name.clone());
                self.advance();
            } else {
                return Err("Expected trait name in bound alias".to_string());
            }

            if self.current_token() == &Token::Plus {
                self.advance(); // consume +
            } else {
                break;
            }
        }

        Ok(Item::BoundAlias {
            name,
            traits,
            location: self.current_location(),
        })
    }

    pub(crate) fn parse_const_or_static(
        &mut self,
    ) -> Result<(String, Type, &'static Expression<'static>), String> {
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

    // Helper: Extract a name from a pattern for backward compatibility

}
