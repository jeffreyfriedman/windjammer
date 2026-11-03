//! Uses after a move, predicted from the emitted Rust: where rustc would report E0382 for a program the
//! interpreter runs (it copies values, so it never sees the move). Read from the generated code, the
//! prediction shares every decision the compiler made (which receivers it takes by value, where it
//! inserted clones, which types are `Copy`) instead of re-deriving them.
//!
//! First class covered: a method that takes `self` by value, called on a local that is used again.

use super::calls::find_callee;
use super::emitted::{EmittedFile, EmittedMode};

/// `var.method(..)` moved `var` (the method takes `self` by value), and `var` is used again.
#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverMove {
    /// The function containing the call (`name` or `Type::name`).
    pub function: String,
    pub var: String,
    /// The callee (`Type::method`).
    pub method: String,
}

pub fn receiver_moves(file: &EmittedFile) -> Vec<ReceiverMove> {
    let mut out = Vec::new();
    let mut names: Vec<&String> = file.fns.keys().collect();
    names.sort();
    for name in names {
        let caller = &file.fns[name];
        for call in caller.calls.iter().filter(|c| c.is_method) {
            let Some(recv) = &call.receiver else { continue };
            let Some(var) = &recv.root else { continue };
            let Some(callee) = find_callee(file, &call.callee, true) else { continue };
            let by_value = callee.params.first().is_some_and(|p| p.name == "self" && p.mode == EmittedMode::Owned);
            let ty = callee.name.rsplit_once("::").map(|(t, _)| t).unwrap_or("");
            // a receiver the caller holds by reference is reborrowed or copied, not moved out of the caller
            let owned = caller.params.iter().find(|p| &p.name == var).map_or(true, |p| p.mode == EmittedMode::Owned);
            if by_value && owned && var != "self" && !file.copy_types.contains(ty) && read_after_move(caller, var, recv.seq, recv.loop_depth) {
                out.push(ReceiverMove { function: name.clone(), var: var.clone(), method: callee.name.clone() });
            }
        }
    }
    out
}

/// `var` is read after the move at `seq` before anything assigns it again. Conservative: any later
/// reassignment of `var` (which may also re-arm it for a loop's next iteration) clears it.
fn read_after_move(f: &super::emitted::EmittedFn, var: &str, seq: usize, loop_depth: usize) -> bool {
    let reassigned = f.assigns.iter().filter(|(s, n)| n == var && *s > seq).map(|(s, _)| *s).min();
    if let Some(r) = reassigned {
        return f.uses.iter().any(|(s, n, _)| n == var && *s > seq && *s < r);
    }
    f.used_after(var, seq, loop_depth)
}
