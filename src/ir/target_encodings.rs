//! Multi-target safety encodings.
//!
//! Defines how the Safety-Typed IR maps to idiomatic safety constructs in
//! each target language. The IR guarantees safety; each backend encodes it.
//!
//! - Rust: native ownership, lifetimes, newtype wrappers
//! - Go: mutex wrappers for MutRef, struct wrappers for taint
//! - JavaScript/TypeScript: Object.freeze, branded types, Proxy
//! - WASM: linear memory, no GC

use crate::ir::safety_type::{OwnedType, Region};
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
