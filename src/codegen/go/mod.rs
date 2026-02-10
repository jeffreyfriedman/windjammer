//! Go code generation backend
//!
//! This module generates Go source code from the Windjammer AST.
//! Safety is enforced by the Windjammer analyzer at compile time —
//! the Go backend only needs to produce semantically-equivalent Go code.
//!
//! ## Design Principles
//!
//! 1. **Idiomatic Go**: Generate code that looks like hand-written Go
//! 2. **Semantic equivalence**: Same observable behavior as Rust backend
//! 3. **Compilation speed**: Leverage Go's fast compilation
//!
//! ## Type Mapping
//!
//! | Windjammer | Go |
//! |------------|------|
//! | `int` | `int64` |
//! | `float` | `float64` |
//! | `bool` | `bool` |
//! | `string` | `string` |
//! | `Vec<T>` | `[]T` (slice) |
//! | `HashMap<K,V>` | `map[K]V` |
//! | `Option<T>` | `*T` (pointer, nil for None) |
//! | Struct | struct |
//! | Enum | interface + variant structs |

mod generator;

pub use generator::GoBackend;
