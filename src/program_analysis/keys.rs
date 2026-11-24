//! Keys name how a product was derived; a product is reused only when every input the key names is equal.

use std::sync::Arc;

/// A parsed file: its name and the exact text parsed (the check parses with no renames; a loader that renames would add
/// the rename map here).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileKey {
    name: String,
    text: Arc<str>,
}

impl FileKey {
    pub fn new(name: &str, text: &str) -> Self {
        Self {
            name: name.to_string(),
            text: text.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The signatures an analysis started from, named by derivation, never by content.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Globals {
    /// `SignatureRegistry::new()`.
    Stdlib,
    /// The stdlib signatures with the first-pass registry of each file merged in, in this order.
    Merged(Vec<FileKey>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnalysisKey {
    file: FileKey,
    globals: Globals,
}

impl AnalysisKey {
    pub fn first_pass(file: &FileKey) -> Self {
        Self {
            file: file.clone(),
            globals: Globals::Stdlib,
        }
    }

    pub fn second_pass(file: &FileKey, globals: Globals) -> Self {
        Self {
            file: file.clone(),
            globals,
        }
    }

    pub fn describe(&self) -> String {
        let pass = match self.globals {
            Globals::Stdlib => "A1",
            Globals::Merged(_) => "A2",
        };
        format!("{pass} of {}", self.file.name)
    }
}
