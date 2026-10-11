//! Language intrinsics (ADR-0014 Phase 1): the single registry of built-in
//! "print / assert / panic / list" functions.
//!
//! Windjammer has no macros. `println`, `assert`, `panic`, ... are built-in
//! *functions* (intrinsics). Two spellings exist during migration:
//!
//! * the canonical plain call: `println("x = ${x}")`
//! * the deprecated Rust-style macro: `println!("x = {}", x)`
//!
//! The parser lowers the macro spelling to the canonical call node (see
//! `parser::intrinsic_lowering`), so every later phase (analyzer, IR, each
//! backend, interpreter) only ever sees one representation. All name matching
//! for intrinsics lives here; backends ask this module instead of matching
//! strings ad hoc.
//!
//! This module also owns the pure text helpers shared by the parser, the
//! backends, `wj lint` and `wj fix`:
//! * interpolation specifier splitting (`"${x:.2}"`)
//! * Rust `{}` template -> Windjammer interpolation conversion

pub mod surface_fix;
pub mod template;

pub use template::split_interp_spec;

/// Every built-in intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    Print,
    Println,
    Eprint,
    Eprintln,
    Format,
    Panic,
    Assert,
    AssertEq,
    AssertNe,
    Todo,
    Unreachable,
    /// `debug(x)`: print value + source location to stderr, return `x`.
    Debug,
    /// Growable list literal (`vec![..]` spelling; `[..]` is the canonical form).
    Vec,
}

impl Intrinsic {
    pub const ALL: [Intrinsic; 13] = [
        Intrinsic::Print,
        Intrinsic::Println,
        Intrinsic::Eprint,
        Intrinsic::Eprintln,
        Intrinsic::Format,
        Intrinsic::Panic,
        Intrinsic::Assert,
        Intrinsic::AssertEq,
        Intrinsic::AssertNe,
        Intrinsic::Todo,
        Intrinsic::Unreachable,
        Intrinsic::Debug,
        Intrinsic::Vec,
    ];

    /// Intrinsic for a plain call `name(..)`.
    pub fn from_call_name(name: &str) -> Option<Intrinsic> {
        Some(match name {
            "print" => Intrinsic::Print,
            "println" => Intrinsic::Println,
            "eprint" => Intrinsic::Eprint,
            "eprintln" => Intrinsic::Eprintln,
            "panic" => Intrinsic::Panic,
            "assert" => Intrinsic::Assert,
            "assert_eq" => Intrinsic::AssertEq,
            "assert_ne" => Intrinsic::AssertNe,
            "todo" => Intrinsic::Todo,
            "unreachable" => Intrinsic::Unreachable,
            "debug" => Intrinsic::Debug,
            _ => return None,
        })
    }

    /// Intrinsic for a deprecated macro spelling `name!(..)`.
    pub fn from_macro_name(name: &str) -> Option<Intrinsic> {
        Some(match name {
            "format" => Intrinsic::Format,
            "vec" => Intrinsic::Vec,
            "dbg" => Intrinsic::Debug,
            other => return Self::from_call_name(other).filter(|i| *i != Intrinsic::Debug),
        })
    }

    /// Canonical plain-call name (what the lowered AST node uses).
    pub fn call_name(self) -> &'static str {
        match self {
            Intrinsic::Print => "print",
            Intrinsic::Println => "println",
            Intrinsic::Eprint => "eprint",
            Intrinsic::Eprintln => "eprintln",
            Intrinsic::Format => "format",
            Intrinsic::Panic => "panic",
            Intrinsic::Assert => "assert",
            Intrinsic::AssertEq => "assert_eq",
            Intrinsic::AssertNe => "assert_ne",
            Intrinsic::Todo => "todo",
            Intrinsic::Unreachable => "unreachable",
            Intrinsic::Debug => "debug",
            Intrinsic::Vec => "vec",
        }
    }

    /// Surface spelling `wj fix` rewrites the deprecated macro to.
    pub fn surface_spelling(self) -> &'static str {
        match self {
            Intrinsic::Eprint => "stderr.print",
            Intrinsic::Eprintln => "stderr.println",
            other => other.call_name(),
        }
    }

    /// Writes text to stdout/stderr.
    pub fn is_print(self) -> bool {
        matches!(
            self,
            Intrinsic::Print | Intrinsic::Println | Intrinsic::Eprint | Intrinsic::Eprintln
        )
    }

    pub fn writes_to_stderr(self) -> bool {
        matches!(self, Intrinsic::Eprint | Intrinsic::Eprintln)
    }

    /// Appends a newline after the text.
    pub fn appends_newline(self) -> bool {
        matches!(self, Intrinsic::Println | Intrinsic::Eprintln)
    }

    /// Diverges (never returns normally).
    pub fn diverges(self) -> bool {
        matches!(
            self,
            Intrinsic::Panic | Intrinsic::Todo | Intrinsic::Unreachable
        )
    }

    /// Index of the (optional) message / template argument.
    pub fn message_arg_index(self) -> Option<usize> {
        match self {
            Intrinsic::Print
            | Intrinsic::Println
            | Intrinsic::Eprint
            | Intrinsic::Eprintln
            | Intrinsic::Format
            | Intrinsic::Panic
            | Intrinsic::Todo
            | Intrinsic::Unreachable => Some(0),
            Intrinsic::Assert => Some(1),
            Intrinsic::AssertEq | Intrinsic::AssertNe => Some(2),
            Intrinsic::Debug | Intrinsic::Vec => None,
        }
    }

    /// Message used by `todo()` / `unreachable()` when no message is given.
    pub fn default_panic_message(self) -> Option<&'static str> {
        match self {
            Intrinsic::Todo => Some("not yet implemented"),
            Intrinsic::Unreachable => Some("internal error: entered unreachable code"),
            _ => None,
        }
    }
}

/// `stderr.println(..)` / `stdout.println(..)` (+ `print`) receiver sugar.
/// Returns the canonical intrinsic for `<stream>.<method>`.
pub fn stream_method_intrinsic(stream: &str, method: &str) -> Option<Intrinsic> {
    match (stream, method) {
        ("stdout", "println") => Some(Intrinsic::Println),
        ("stdout", "print") => Some(Intrinsic::Print),
        ("stderr", "println") => Some(Intrinsic::Eprintln),
        ("stderr", "print") => Some(Intrinsic::Eprint),
        _ => None,
    }
}

/// Register the surface-syntax deprecation diagnostics in the error catalog.
pub fn register_catalog(catalog: &mut crate::error_catalog::ErrorCatalog) {
    use crate::error_catalog::{ErrorEntry, ErrorExample};
    catalog.add_error(ErrorEntry {
        code: "W0007".to_string(),
        title: "Deprecated macro spelling (`println!`, `vec![]`, ...)".to_string(),
        explanation: "Windjammer has no macros. print/println/panic/assert/assert_eq/assert_ne/todo/unreachable \
            are built-in functions, `dbg!` is `debug(x)`, `eprintln!` is `stderr.println(..)`, `format!` is an \
            interpolated string and `vec![..]` is the list literal `[..]`. The `!` forms still compile but will \
            become errors after one release (ADR-0014)."
            .to_string(),
        causes: vec!["Rust-style macro syntax".to_string()],
        solutions: vec![
            "Run `wj fix <path>` to rewrite automatically".to_string(),
            "Replace `name!(..)` with `name(..)`".to_string(),
        ],
        examples: vec![ErrorExample {
            title: "println! macro".to_string(),
            bad_code: "println!(\"x = {}\", x)".to_string(),
            good_code: "println(\"x = ${x}\")".to_string(),
            explanation: "Plain call with string interpolation".to_string(),
        }],
        related: vec!["W0008".to_string()],
        severity: "warning".to_string(),
        category: "Style".to_string(),
    });
    catalog.add_error(ErrorEntry {
        code: "W0008".to_string(),
        title: "`{}`-style format string".to_string(),
        explanation: "Windjammer uses string interpolation (`\"${x}\"`, `\"${x:.2}\"`, `\"${x:?}\"`) instead of \
            Rust `{}` / `{:?}` format strings with trailing arguments."
            .to_string(),
        causes: vec!["Rust-style format string with positional arguments".to_string()],
        solutions: vec!["Run `wj fix <path>` or inline the arguments with `${..}`".to_string()],
        examples: vec![ErrorExample {
            title: "format string".to_string(),
            bad_code: "println(\"a {} b {:?}\", x, y)".to_string(),
            good_code: "println(\"a ${x} b ${y:?}\")".to_string(),
            explanation: "Arguments move into the string".to_string(),
        }],
        related: vec!["W0007".to_string()],
        severity: "warning".to_string(),
        category: "Style".to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_round_trips() {
        for i in Intrinsic::ALL {
            if i != Intrinsic::Format && i != Intrinsic::Vec {
                assert_eq!(Intrinsic::from_call_name(i.call_name()), Some(i));
            }
        }
        assert_eq!(Intrinsic::from_macro_name("dbg"), Some(Intrinsic::Debug));
        assert_eq!(Intrinsic::from_macro_name("debug"), None);
        assert_eq!(Intrinsic::from_macro_name("vec"), Some(Intrinsic::Vec));
        assert_eq!(Intrinsic::from_macro_name("nope"), None);
    }

    #[test]
    fn stream_sugar() {
        assert_eq!(
            stream_method_intrinsic("stderr", "println"),
            Some(Intrinsic::Eprintln)
        );
        assert_eq!(
            stream_method_intrinsic("stdout", "print"),
            Some(Intrinsic::Print)
        );
        assert_eq!(stream_method_intrinsic("stdin", "println"), None);
    }
}
