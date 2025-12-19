// Ownership - Ownership and mutability hints for Windjammer

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OwnershipHint {
    Owned,
    Ref,
    Mut,
    Inferred, // Let the analyzer decide
}


