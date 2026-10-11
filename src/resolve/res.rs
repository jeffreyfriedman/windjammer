//! Resolution results: what a call expression refers to, and the shadow-mode report.

use super::def::DefId;
use crate::source_map::Location;
use std::collections::BTreeMap;

/// Why a name could not be tied to a program definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// Provided outside the program (std, runtime, other crates). Full path attached.
    External(String),
    /// No such name in scope (a real diagnostic).
    NotFound(String),
    /// Type resolved but has no such associated item.
    NoAssocItem(String),
    /// `T::f` where `T` is a generic parameter; dispatch is decided at monomorphization.
    GenericDispatch(String),
    /// Callee is a computed value (`self.cb(x)`, `(make())(x)`).
    Dynamic,
    /// Method call with no user-defined method of that name and arity.
    NoMethodCandidate(String),
}

/// Resolution of one name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Res {
    Def(DefId),
    Local(String),
    /// Language/runtime prelude item (`Some`, `Vec::new`, `i32::from`).
    Prim(String),
    Unresolved(Reason),
    /// Several distinct defs match; an error, never silently picked.
    Ambiguous(Vec<DefId>),
    /// Method call with unknown receiver type: all same-name, same-arity methods.
    MethodCandidates(Vec<DefId>),
}

/// Syntactic shape of the callee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    /// `f(x)`
    Free,
    /// `A::b(x)`
    Path,
    /// `recv.m(x)`
    Method,
    /// Callee is any other expression.
    Dynamic,
}

/// Identity of an expression node (address in the parser arena; stable while the AST lives).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprKey(pub usize);

/// One resolved call site.
#[derive(Debug, Clone)]
pub struct CallResolution {
    pub key: ExprKey,
    pub kind: CallKind,
    /// `f`, `A::b`, or the method name for method calls.
    pub callee: String,
    pub arg_count: usize,
    /// Function/method whose body contains the call (None for module-level initializers).
    pub enclosing: Option<DefId>,
    pub module: DefId,
    pub location: Option<Location>,
    pub res: Res,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    UnresolvedCall,
    AmbiguousCall,
    BadImport,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub location: Option<Location>,
}

/// Counts and diagnostics from one resolver run.
#[derive(Debug, Clone, Default)]
pub struct ResolveReport {
    pub total_calls: usize,
    /// `Def` + `Local` + `Prim`.
    pub resolved: usize,
    pub method_candidates: usize,
    pub unresolved: usize,
    pub ambiguous: usize,
    /// Unresolved breakdown by reason kind (`External`, `NotFound`, ...).
    pub unresolved_by_reason: BTreeMap<&'static str, usize>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Reason {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Reason::External(_) => "External",
            Reason::NotFound(_) => "NotFound",
            Reason::NoAssocItem(_) => "NoAssocItem",
            Reason::GenericDispatch(_) => "GenericDispatch",
            Reason::Dynamic => "Dynamic",
            Reason::NoMethodCandidate(_) => "NoMethodCandidate",
        }
    }

    /// True for reasons that mean "not our program's problem" (std/runtime/dynamic).
    pub fn is_benign(&self) -> bool {
        matches!(
            self,
            Reason::External(_)
                | Reason::GenericDispatch(_)
                | Reason::Dynamic
                | Reason::NoMethodCandidate(_)
        )
    }
}

impl ResolveReport {
    pub(crate) fn from_calls(calls: &[CallResolution], mut diagnostics: Vec<Diagnostic>) -> Self {
        let mut r = ResolveReport {
            total_calls: calls.len(),
            ..Default::default()
        };
        for c in calls {
            match &c.res {
                Res::Def(_) | Res::Local(_) | Res::Prim(_) => r.resolved += 1,
                Res::MethodCandidates(_) => r.method_candidates += 1,
                Res::Ambiguous(_) => {
                    r.ambiguous += 1;
                    diagnostics.push(call_diag(DiagnosticKind::AmbiguousCall, c, "ambiguous"));
                }
                Res::Unresolved(reason) => {
                    r.unresolved += 1;
                    *r.unresolved_by_reason
                        .entry(reason.kind_name())
                        .or_default() += 1;
                    if !reason.is_benign() {
                        diagnostics.push(call_diag(
                            DiagnosticKind::UnresolvedCall,
                            c,
                            "unresolved",
                        ));
                    }
                }
            }
        }
        r.diagnostics = diagnostics;
        r
    }
}

fn call_diag(kind: DiagnosticKind, c: &CallResolution, what: &str) -> Diagnostic {
    Diagnostic {
        kind,
        message: format!("{} call to `{}`: {:?}", what, c.callee, c.res),
        location: c.location.clone(),
    }
}
