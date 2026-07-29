//! Multi-target safety encodings.
//!
//! Defines how the Safety-Typed IR maps to idiomatic safety constructs in
//! each target language. The IR guarantees safety; each backend encodes it.
//!
//! ## Active targets
//!
//! - **Rust**: native ownership, lifetimes, newtype wrappers
//! - **Go**: mutex wrappers for MutRef, struct wrappers for taint
//! - **JavaScript/TypeScript**: Object.freeze, branded types, Proxy
//! - **WASM**: linear memory, no GC
//!
//! ## Adding a new backend (C++, C#, Java, Python, Ruby, …)
//!
//! 1. Add a `Target` variant here (or a sibling `FutureTarget` until the backend lands).
//! 2. Implement `encode_ownership`, `apply_coercion`, and `encode_taint` arms for that variant.
//! 3. Wire the backend's `ir_lowering` module to call `resolve_call_arg_actual_type` +
//!    `safety_type_from_signature_param` + `encode_call_argument` — **do not** duplicate
//!    ownership heuristics in the backend.
//! 4. Add conformance tests in `standard_equivalence_tests()` so all targets agree on
//!    coercion semantics (borrow vs clone vs identity).
//!
//! Ownership decisions stay in the IR constraint solver; new backends only encode
//! already-resolved `SafetyType` pairs.

use crate::ir::coercion::{compute_coercion, CoercionKind};
use crate::ir::safety_type::{BaseType, OwnedType, Region, SafetyType};
use crate::ir::taint::TaintSourceKind;

/// Target language identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Rust,
    Go,
    JavaScript,
    TypeScript,
    Wasm,
}

/// How a safety type is encoded in a specific target.
#[derive(Debug, Clone)]
pub struct SafetyEncoding {
    pub target: Target,
    pub ownership_encoding: OwnershipEncoding,
    pub taint_encoding: TaintEncoding,
    pub effect_encoding: EffectEncoding,
}

/// How ownership is encoded per target.
#[derive(Debug, Clone, PartialEq)]
pub enum OwnershipEncoding {
    /// Rust: native &T, &mut T, T, clone
    RustNative { emit: String },
    /// Go: value type, *T with sync.RWMutex wrapper
    GoMutex {
        needs_lock: bool,
        lock_type: GoLockType,
        emit: String,
    },
    /// JS/TS: Object.freeze for Ref, Readonly<T> for TS
    JsFrozen {
        freeze_in_dev: bool,
        readonly_type: bool,
        emit: String,
    },
    /// WASM: linear memory pointer with no ownership transfer
    WasmLinear { offset: u32, emit: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GoLockType {
    RWMutex,
    Mutex,
    None,
}

/// How taint is encoded per target.
#[derive(Debug, Clone, PartialEq)]
pub enum TaintEncoding {
    /// Rust: Tainted<T> newtype wrapper
    RustNewtype { wrapper: String },
    /// Go: struct wrapper with accessor methods
    GoStructWrapper { type_name: String },
    /// TypeScript: branded type `T & { __taint: true }`
    TsBrandedType { brand: String },
    /// JS: Proxy that throws on unguarded access
    JsProxy,
    /// No taint encoding (target doesn't support it)
    None,
}

/// How effects are encoded per target.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectEncoding {
    /// Rust: trait bounds or module-level attributes
    RustTraitBounds { bounds: Vec<String> },
    /// Go: build tags enforced by go vet
    GoBuildTags { tags: Vec<String> },
    /// TypeScript: interface segregation (no I/O types in pure functions)
    TsInterfaceSegregation { restricted_types: Vec<String> },
    /// No effect encoding
    None,
}

/// Encode an ownership type for a specific target.
pub fn encode_ownership(ownership: &OwnedType, target: Target) -> OwnershipEncoding {
    match target {
        Target::Rust => match ownership {
            OwnedType::Owned => OwnershipEncoding::RustNative { emit: "T".into() },
            OwnedType::Ref(Region(r)) => OwnershipEncoding::RustNative {
                emit: format!("&'r{} T", r),
            },
            OwnedType::MutRef(Region(r)) => OwnershipEncoding::RustNative {
                emit: format!("&'r{} mut T", r),
            },
            OwnedType::Copy => OwnershipEncoding::RustNative { emit: "T".into() },
            OwnedType::Inferred => OwnershipEncoding::RustNative {
                emit: "T /* inferred */".into(),
            },
        },

        Target::Go => match ownership {
            OwnedType::Owned => OwnershipEncoding::GoMutex {
                needs_lock: false,
                lock_type: GoLockType::None,
                emit: "T".into(),
            },
            OwnedType::Ref(_) => OwnershipEncoding::GoMutex {
                needs_lock: true,
                lock_type: GoLockType::RWMutex,
                emit: "*T /* RLock */".into(),
            },
            OwnedType::MutRef(_) => OwnershipEncoding::GoMutex {
                needs_lock: true,
                lock_type: GoLockType::Mutex,
                emit: "*T /* Lock */".into(),
            },
            OwnedType::Copy => OwnershipEncoding::GoMutex {
                needs_lock: false,
                lock_type: GoLockType::None,
                emit: "T".into(),
            },
            OwnedType::Inferred => OwnershipEncoding::GoMutex {
                needs_lock: false,
                lock_type: GoLockType::None,
                emit: "T".into(),
            },
        },

        Target::JavaScript | Target::TypeScript => match ownership {
            OwnedType::Owned => OwnershipEncoding::JsFrozen {
                freeze_in_dev: false,
                readonly_type: false,
                emit: "T".into(),
            },
            OwnedType::Ref(_) => OwnershipEncoding::JsFrozen {
                freeze_in_dev: true,
                readonly_type: target == Target::TypeScript,
                emit: if target == Target::TypeScript {
                    "Readonly<T>".into()
                } else {
                    "Object.freeze(T)".into()
                },
            },
            OwnedType::MutRef(_) => OwnershipEncoding::JsFrozen {
                freeze_in_dev: false,
                readonly_type: false,
                emit: "T /* mut */".into(),
            },
            OwnedType::Copy => OwnershipEncoding::JsFrozen {
                freeze_in_dev: false,
                readonly_type: false,
                emit: "T".into(),
            },
            OwnedType::Inferred => OwnershipEncoding::JsFrozen {
                freeze_in_dev: false,
                readonly_type: false,
                emit: "T".into(),
            },
        },

        Target::Wasm => match ownership {
            OwnedType::Owned => OwnershipEncoding::WasmLinear {
                offset: 0,
                emit: "i32 /* ptr, owned */".into(),
            },
            OwnedType::Ref(_) => OwnershipEncoding::WasmLinear {
                offset: 0,
                emit: "i32 /* ptr, borrowed */".into(),
            },
            OwnedType::MutRef(_) => OwnershipEncoding::WasmLinear {
                offset: 0,
                emit: "i32 /* ptr, mut borrowed */".into(),
            },
            OwnedType::Copy => OwnershipEncoding::WasmLinear {
                offset: 0,
                emit: "T /* value */".into(),
            },
            OwnedType::Inferred => OwnershipEncoding::WasmLinear {
                offset: 0,
                emit: "i32 /* ptr */".into(),
            },
        },
    }
}

/// Wrap `expr` in a shared borrow, parenthesizing when needed for precedence.
pub(crate) fn rust_shared_borrow(expr: &str) -> String {
    if expr.starts_with('&') && !expr.starts_with("&mut ") {
        return expr.to_string();
    }
    // Rust string literals are already `&str`; `&"…"` is `&&str`.
    if crate::codegen::rust::expression_utilities::is_rust_string_literal_text(expr) {
        return expr.to_string();
    }
    if needs_borrow_parentheses(expr) {
        format!("&({expr})")
    } else {
        format!("&{expr}")
    }
}

/// Wrap `expr` in a mutable borrow, parenthesizing when needed for precedence.
fn rust_mut_borrow(expr: &str) -> String {
    if expr.starts_with("&mut ") {
        return expr.to_string();
    }
    let base = crate::codegen::rust::expression_utilities::borrow_base_expr(expr);
    if needs_borrow_parentheses(base) {
        format!("&mut ({base})")
    } else {
        format!("&mut {base}")
    }
}

fn needs_borrow_parentheses(expr: &str) -> bool {
    expr.contains(" as ")
        || expr.contains(" + ")
        || expr.contains(" - ")
        || expr.contains(" * ")
        || expr.contains(" / ")
        || expr.contains(" % ")
        || expr.contains(" << ")
        || expr.contains(" >> ")
        || expr.contains(" && ")
        || expr.contains(" || ")
}

/// Apply a target-agnostic coercion to a generated expression string.
pub fn apply_coercion(kind: &CoercionKind, expr: &str, target: Target) -> String {
    match (target, kind) {
        (_, CoercionKind::Identity) => expr.to_string(),
        (Target::Rust, CoercionKind::Borrow) => rust_shared_borrow(expr),
        (Target::Rust, CoercionKind::MutBorrow) => rust_mut_borrow(expr),
        (Target::Rust, CoercionKind::Clone) => {
            if expr.ends_with(".clone()")
                || expr.ends_with(".to_string()")
                || expr.ends_with(".to_owned()")
            {
                expr.to_string()
            } else {
                let core = if let Some(rest) = expr.strip_prefix("&mut ") {
                    rest
                } else if let Some(rest) = expr.strip_prefix('&') {
                    rest
                } else {
                    expr
                };
                let core = core
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or(core);
                format!("{core}.clone()")
            }
        }
        (Target::Rust, CoercionKind::Deref) => {
            if expr.starts_with('*') {
                expr.to_string()
            } else if expr.trim().starts_with('&')
                || (expr.trim().starts_with('(') && expr.contains('&'))
            {
                // `&x` / `(&x)` → owned Copy: strip borrow (auto-copy), do not `*x`.
                let core = expr
                    .trim()
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or(expr)
                    .trim()
                    .trim_start_matches("&mut ")
                    .trim_start_matches('&');
                core.to_string()
            } else {
                // Bare `&T` binding name → `*name`.
                let core = expr
                    .trim()
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or(expr)
                    .trim();
                format!("*{core}")
            }
        }
        (Target::Rust, CoercionKind::ToOwnedString) => {
            let core = if let Some(rest) = expr.strip_prefix("&mut ") {
                rest
            } else if let Some(rest) = expr.strip_prefix('&') {
                rest
            } else {
                expr
            };
            if core.ends_with(".to_string()") || core.ends_with(".to_owned()") {
                expr.to_string()
            } else {
                format!("{}.to_string()", core)
            }
        }
        (Target::Rust, CoercionKind::StripBorrow) => {
            let trimmed = expr.trim();
            if let Some(rest) = trimmed.strip_prefix("&mut ") {
                rest.to_string()
            } else if let Some(rest) = trimmed.strip_prefix('&') {
                rest.to_string()
            } else if trimmed.starts_with('(') && trimmed.ends_with(')') {
                let inner = trimmed[1..trimmed.len() - 1].trim();
                if let Some(rest) = inner.strip_prefix("&mut ") {
                    rest.to_string()
                } else if let Some(rest) = inner.strip_prefix('&') {
                    rest.to_string()
                } else {
                    inner.to_string()
                }
            } else {
                expr.to_string()
            }
        }
        (Target::Rust, CoercionKind::NumericCast(base)) => {
            format!("{} as {}", expr, base_type_rust_cast(base))
        }
        // Go/JS/WASM: pass-through for now; IR encodings expand in phase 3.
        (_, CoercionKind::Borrow | CoercionKind::MutBorrow | CoercionKind::StripBorrow) => {
            expr.to_string()
        }
        (_, CoercionKind::Clone) => expr.to_string(),
        (_, CoercionKind::Deref) => expr.to_string(),
        (_, CoercionKind::ToOwnedString) => expr.to_string(),
        (_, CoercionKind::NumericCast(base)) => {
            format!("{} /* cast {:?} */", expr, base)
        }
    }
}

/// Encode a call-site argument: compute coercion from SafetyTypes and apply for target.
pub fn encode_call_argument(
    actual: &SafetyType,
    expected: &SafetyType,
    target: Target,
    expr: &str,
) -> String {
    let kind = compute_coercion(actual, expected);
    apply_coercion(&kind, expr, target)
}

fn base_type_rust_cast(base: &BaseType) -> &'static str {
    match base {
        BaseType::F32 => "f32",
        BaseType::F64 => "f64",
        BaseType::I8 => "i8",
        BaseType::I16 => "i16",
        BaseType::I32 => "i32",
        BaseType::I64 => "i64",
        BaseType::I128 => "i128",
        BaseType::U8 => "u8",
        BaseType::U16 => "u16",
        BaseType::U32 => "u32",
        BaseType::U64 => "u64",
        BaseType::U128 => "u128",
        _ => "/* unknown cast */",
    }
}

/// Encode taint for a specific target.
pub fn encode_taint(source: &TaintSourceKind, target: Target) -> TaintEncoding {
    match target {
        Target::Rust => TaintEncoding::RustNewtype {
            wrapper: format!("Tainted<T, {:?}>", source),
        },
        Target::Go => TaintEncoding::GoStructWrapper {
            type_name: format!("Tainted{:?}", source),
        },
        Target::TypeScript => TaintEncoding::TsBrandedType {
            brand: format!("T & {{ __taint_{:?}: true }}", source),
        },
        Target::JavaScript => TaintEncoding::JsProxy,
        Target::Wasm => TaintEncoding::None,
    }
}

/// Cross-target semantic equivalence test specification.
/// Used to verify that the same Windjammer program produces semantically
/// equivalent behavior across all target languages.
#[derive(Debug, Clone)]
pub struct SemanticEquivalenceTest {
    pub name: String,
    pub wj_source: String,
    pub expected_behavior: ExpectedBehavior,
}

#[derive(Debug, Clone)]
pub enum ExpectedBehavior {
    /// Function returns this value in all targets.
    Returns(String),
    /// Compile error with this message pattern.
    CompileError(String),
    /// Runtime panic/error (taint violation in dev mode).
    RuntimeError(String),
}

/// Standard cross-target equivalence tests.
pub fn standard_equivalence_tests() -> Vec<SemanticEquivalenceTest> {
    vec![
        SemanticEquivalenceTest {
            name: "ownership_move_prevents_use_after".into(),
            wj_source: r#"
                fn consume(s: String) -> i32 { s.len() }
                fn main() -> i32 {
                    let s = "hello".to_string()
                    consume(s)
                    // s is moved — cannot use here
                }
            "#
            .into(),
            expected_behavior: ExpectedBehavior::Returns("5".into()),
        },
        SemanticEquivalenceTest {
            name: "taint_blocks_unsafe_sink".into(),
            wj_source: r#"
                fn handler(body: Tainted<String>) {
                    db.query(body)  // ERROR: tainted
                }
            "#
            .into(),
            expected_behavior: ExpectedBehavior::CompileError("tainted data reaches sink".into()),
        },
        SemanticEquivalenceTest {
            name: "effect_blocks_unauthorized_io".into(),
            wj_source: r#"
                // manifest: effects = ["logic_only"]
                fn process() {
                    std::fs::read("secret.txt")  // ERROR: fs_read not allowed
                }
            "#
            .into(),
            expected_behavior: ExpectedBehavior::CompileError("effect not in manifest".into()),
        },
        SemanticEquivalenceTest {
            name: "spawn_produces_join_handle".into(),
            wj_source: r#"
                fn compute() -> i32 { 42 }
                fn main() -> i32 {
                    let handle = spawn compute()
                    handle.join()
                }
            "#
            .into(),
            expected_behavior: ExpectedBehavior::Returns("42".into()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_owned_encoding() {
        let enc = encode_ownership(&OwnedType::Owned, Target::Rust);
        assert!(matches!(enc, OwnershipEncoding::RustNative { .. }));
    }

    #[test]
    fn test_go_mutref_uses_mutex() {
        let enc = encode_ownership(&OwnedType::MutRef(Region(1)), Target::Go);
        match enc {
            OwnershipEncoding::GoMutex {
                needs_lock,
                lock_type,
                ..
            } => {
                assert!(needs_lock);
                assert_eq!(lock_type, GoLockType::Mutex);
            }
            _ => panic!("expected GoMutex"),
        }
    }

    #[test]
    fn test_go_ref_uses_rwmutex() {
        let enc = encode_ownership(&OwnedType::Ref(Region(1)), Target::Go);
        match enc {
            OwnershipEncoding::GoMutex {
                needs_lock,
                lock_type,
                ..
            } => {
                assert!(needs_lock);
                assert_eq!(lock_type, GoLockType::RWMutex);
            }
            _ => panic!("expected GoMutex"),
        }
    }

    #[test]
    fn test_typescript_ref_is_readonly() {
        let enc = encode_ownership(&OwnedType::Ref(Region(1)), Target::TypeScript);
        match enc {
            OwnershipEncoding::JsFrozen {
                readonly_type,
                emit,
                ..
            } => {
                assert!(readonly_type);
                assert!(emit.contains("Readonly"));
            }
            _ => panic!("expected JsFrozen"),
        }
    }

    #[test]
    fn test_js_ref_uses_freeze() {
        let enc = encode_ownership(&OwnedType::Ref(Region(1)), Target::JavaScript);
        match enc {
            OwnershipEncoding::JsFrozen {
                freeze_in_dev,
                emit,
                ..
            } => {
                assert!(freeze_in_dev);
                assert!(emit.contains("Object.freeze"));
            }
            _ => panic!("expected JsFrozen"),
        }
    }

    #[test]
    fn test_rust_taint_newtype() {
        let enc = encode_taint(&TaintSourceKind::HttpRequestBody, Target::Rust);
        match enc {
            TaintEncoding::RustNewtype { wrapper } => {
                assert!(wrapper.contains("Tainted"));
            }
            _ => panic!("expected RustNewtype"),
        }
    }

    #[test]
    fn test_typescript_taint_branded() {
        let enc = encode_taint(&TaintSourceKind::HttpRequestBody, Target::TypeScript);
        match enc {
            TaintEncoding::TsBrandedType { brand } => {
                assert!(brand.contains("__taint"));
            }
            _ => panic!("expected TsBrandedType"),
        }
    }

    #[test]
    fn test_equivalence_tests_exist() {
        let tests = standard_equivalence_tests();
        assert!(tests.len() >= 4);
    }

    #[test]
    fn test_encode_call_argument_borrow_for_rust() {
        let actual = SafetyType::owned(BaseType::String);
        let expected = SafetyType::borrowed(BaseType::String, Region::fresh(0));
        let encoded = encode_call_argument(&actual, &expected, Target::Rust, "key");
        assert_eq!(encoded, "&key");
    }

    #[test]
    fn test_rust_shared_borrow_skips_string_literals() {
        assert_eq!(rust_shared_borrow(r#""</div>""#), r#""</div>""#);
        assert_eq!(rust_shared_borrow("key"), "&key");
        // Owned String → &str still borrows non-literals.
        let actual = SafetyType::owned(BaseType::String);
        let expected = SafetyType::borrowed(BaseType::String, Region::fresh(0));
        let encoded = encode_call_argument(&actual, &expected, Target::Rust, r#""</div>""#);
        assert_eq!(encoded, r#""</div>""#);
    }

    #[test]
    fn test_all_targets_encode_owned() {
        for target in [
            Target::Rust,
            Target::Go,
            Target::JavaScript,
            Target::TypeScript,
            Target::Wasm,
        ] {
            let enc = encode_ownership(&OwnedType::Owned, target);
            // All targets should produce some encoding for owned
            match enc {
                OwnershipEncoding::RustNative { .. }
                | OwnershipEncoding::GoMutex { .. }
                | OwnershipEncoding::JsFrozen { .. }
                | OwnershipEncoding::WasmLinear { .. } => {}
            }
        }
    }
}
