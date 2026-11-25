//! Where a declared formal sits in a registry signature (implicit-`self` methods shift every formal by one slot).

use crate::analyzer::FunctionSignature;
use crate::parser::FunctionDecl;

/// Index of declared parameter `param_idx` in a registry signature. An implicit `self` (used, not declared) owns
/// slot 0 of the signature but is not in `func.parameters`, so every declared formal sits one slot later; reading
/// slot `param_idx` handed the first formal the receiver's `&mut` (`Option<T>` param emitted as `&mut Option<T>`).
pub(crate) fn sig_formal_slot(func: &FunctionDecl<'_>, sig: &FunctionSignature, param_idx: usize) -> usize {
    let declares_self = func.parameters.iter().any(|p| p.name == "self");
    param_idx + usize::from(sig.has_self_receiver_slot() && !declares_self)
}
