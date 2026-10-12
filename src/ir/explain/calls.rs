//! Call-site report: how each argument reaches a callee defined in the same file, and why.
//! Derived from the emitted Rust (the program that actually runs) plus callee formals.

use super::emitted::{ArgForm, EmittedFile, EmittedFn, EmittedMode};
use super::report::{CallArgReport, CallReport, Finding};

/// Find the emitted callee for a call: free function / `Type::method` path by name, or a
/// method call by its (unique) method name.
fn find_callee<'a>(file: &'a EmittedFile, callee: &str, is_method: bool) -> Option<&'a EmittedFn> {
    if !is_method {
        return file
            .fns
            .get(callee)
            .or_else(|| file.fns.get(callee.strip_prefix("Self::").unwrap_or(callee)));
    }
    let mut hits = file
        .fns
        .iter()
        .filter(|(k, _)| k.contains("::") && k.rsplit("::").next() == Some(callee));
    let first = hits.next()?;
    if hits.next().is_some() {
        return None;
    }
    Some(first.1)
}

fn caller_ref(caller: &EmittedFn, var: &str) -> bool {
    caller
        .params
        .iter()
        .any(|p| p.name == var && p.mode != EmittedMode::Owned)
}

fn is_owned_var(caller: &EmittedFn, var: &str) -> Option<bool> {
    if let Some(p) = caller.params.iter().find(|p| p.name == var) {
        return Some(p.mode == EmittedMode::Owned);
    }
    caller
        .lets
        .iter()
        .rev()
        .find(|l| l.name == var)
        .map(|l| l.owned_init)
}

pub fn call_reports(caller: &EmittedFn, file: &EmittedFile) -> Vec<CallReport> {
    let mut out = Vec::new();
    for call in &caller.calls {
        let Some(callee) = find_callee(file, &call.callee, call.is_method) else { continue };
        let skip_self = call.is_method && callee.params.first().is_some_and(|p| p.name == "self");
        let formals: Vec<_> = callee.params.iter().skip(usize::from(skip_self)).collect();
        let mut args = Vec::new();
        for (i, a) in call.args.iter().enumerate() {
            let formal = formals.get(i).map(|p| p.mode);
            let used_later = a
                .root
                .as_ref()
                .map(|v| caller.used_after(v, a.seq, a.loop_depth));
            let mut findings = Vec::new();
            let rule = match (formal, a.form) {
                (Some(EmittedMode::Ref), ArgForm::Borrow) => Some("CALL-BORROW"),
                (Some(EmittedMode::MutRef), ArgForm::BorrowMut) => Some("CALL-BORROW-MUT"),
                (Some(EmittedMode::Owned), ArgForm::Clone) => {
                    let owned = a.root.as_ref().and_then(|v| is_owned_var(caller, v));
                    if used_later == Some(false) && owned == Some(true) {
                        findings.push(Finding {
                            id: "D-CLONE-LASTUSE".into(),
                            message: format!(
                                "`{}` is cloned for an owned formal but is not used afterwards; the last use should move",
                                a.root.clone().unwrap_or_default()
                            ),
                        });
                    }
                    Some("CALL-CLONE")
                }
                (Some(EmittedMode::Owned), ArgForm::Bare) => Some("CALL-MOVE"),
                // A reference parameter handed straight on is a reborrow, not a move.
                (Some(EmittedMode::Ref), ArgForm::Bare)
                    if a.root.as_ref().is_some_and(|v| caller_ref(caller, v)) =>
                {
                    Some("CALL-BORROW")
                }
                (Some(EmittedMode::MutRef), ArgForm::Bare)
                    if a.root.as_ref().is_some_and(|v| caller_ref(caller, v)) =>
                {
                    Some("CALL-BORROW-MUT")
                }
                _ => None,
            };
            args.push(CallArgReport {
                index: i,
                form: a.form.label().to_string(),
                var: a.root.clone(),
                formal: formal.map(|m| m.label().to_string()),
                used_later,
                rule: rule.map(String::from),
                findings,
            });
        }
        out.push(CallReport {
            callee: call.callee.clone(),
            args,
        });
    }
    out
}
