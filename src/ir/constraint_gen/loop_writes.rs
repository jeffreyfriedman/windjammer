//! OWN-LOOP-WRITE: a `for` binding that is assigned, or whose field or element is assigned
//! (`for mut q in qs { q.xs[0] = 1 }`, `for mut row in grid { row[0] = 7 }`), writes through to the element, so
//! the loop iterates the collection mutably. The assignment's root is resolved to the binding it names (lexical
//! scoping), never by name, so a body `let q = ..` that shadows the loop variable does not count.

use super::*;

/// Open loops (by the `for` statement's source position) with the variables their pattern binds.
#[derive(Debug, Default)]
pub(super) struct LoopWrites {
    open: Vec<((usize, usize), Vec<ConstraintVar>)>,
    pub(super) written: std::collections::BTreeSet<(usize, usize)>,
}

impl LoopWrites {
    /// A write through `root` marks every open loop that bound it.
    fn write(&mut self, root: ConstraintVar) {
        for (at, vars) in &self.open {
            if vars.contains(&root) {
                self.written.insert(*at);
            }
        }
    }
}

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// Called after the loop pattern is bound in its own scope (`None` closes the innermost loop).
    pub(super) fn loop_writes_scope(&mut self, location: Option<&crate::parser::SourceLocation>) {
        let Some(location) = location else {
            self.loop_writes.open.pop();
            return;
        };
        let vars = self.scopes.last().map(|s| s.values().copied().collect()).unwrap_or_default();
        let at = location.as_ref().map(|l| (l.line, l.column)).unwrap_or((0, 0));
        self.loop_writes.open.push((at, vars));
    }

    /// The root binding of an assignment target, if it is a loop binding still in scope.
    pub(super) fn loop_writes_target(&mut self, target: &Expression<'ast>) {
        let Some(name) = self.root_identifier(target) else { return };
        if let Some(&v) = self.scopes.iter().rev().find_map(|s| s.get(&name)) {
            self.loop_writes.write(v);
        }
    }
}
