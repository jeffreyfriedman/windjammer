//! The `const` / `static` declarations numeric inference knows, looked up by declaration: a name resolves to the
//! declaration of the file that uses it, and otherwise only when every declaration of that name agrees. Two modules
//! that declare `TAU` with different types never answer each other's lookups (no bare-name guess).

use crate::parser::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ConstTable {
    by_file: HashMap<(String, String), Type>,
    /// Name -> its type when every declaration agrees; `None` when two declarations disagree.
    global: HashMap<String, Option<Type>>,
}

impl ConstTable {
    pub fn insert(&mut self, file: &str, name: String, ty: Type) {
        self.by_file.insert((file.to_string(), name.clone()), ty.clone());
        match self.global.get(&name) {
            None => {
                self.global.insert(name, Some(ty));
            }
            Some(Some(prev)) if *prev == ty => {}
            Some(_) => {
                self.global.insert(name, None);
            }
        }
    }

    /// The type of `name` as seen from `file`.
    pub fn get(&self, file: &str, name: &str) -> Option<&Type> {
        if let Some((module, item)) = name.rsplit_once("::") {
            return self.get_in_module(module, item);
        }
        self.by_file
            .get(&(file.to_string(), name.to_string()))
            .or_else(|| self.global.get(name).and_then(Option::as_ref))
    }

    /// `module::NAME` (`narrow::TAU`, `crate::narrow::TAU`): the constant the module's own file declares. The module
    /// is the file `<module>.wj` or `<module>/mod.wj` (the file `mod <module>` declares).
    fn get_in_module(&self, module: &str, item: &str) -> Option<&Type> {
        let seg = module.rsplit("::").next().unwrap_or(module);
        let mut found = self.by_file.iter().filter(|((file, name), _)| name == item && file_is_module(file, seg));
        let first = found.next()?;
        if found.next().is_some() {
            return None;
        }
        Some(first.1)
    }

    /// True when some module declares `name` (also when the declarations disagree).
    pub fn is_declared(&self, name: &str) -> bool {
        if let Some((module, item)) = name.rsplit_once("::") {
            return self.get_in_module(module, item).is_some();
        }
        self.global.contains_key(name)
    }

    pub fn extend(&mut self, other: ConstTable) {
        for ((file, name), ty) in other.by_file {
            self.insert(&file, name, ty);
        }
        for (name, ty) in other.global {
            if ty.is_none() {
                self.global.insert(name, None);
            }
        }
    }

    /// Register the constants the modules a program imports declare, read from their cached metadata: `use crate::narrow`
    /// (the module, reachable as `narrow::NAME`) and `use crate::narrow::TAU` (an item of `narrow`). The module is the
    /// metadata file `<path>.wj.meta` or `<path>/mod.wj.meta`; its constants are registered under `<module>.wj`.
    pub fn load_imported(
        &mut self,
        program: &crate::parser::Program,
        source_root: Option<&(std::path::PathBuf, std::path::PathBuf)>,
    ) {
        use crate::metadata::ModuleMetadata;
        use crate::parser::ast::core::Item;
        use std::path::PathBuf;
        let root = source_root.map(|(root, cache)| (root.as_path(), cache.as_path()));
        for item in &program.items {
            let Item::Use { path, .. } = item else { continue };
            let segs: Vec<&str> =
                path.iter().map(String::as_str).skip_while(|s| matches!(*s, "crate" | "self" | "super")).collect();
            for take in [segs.len(), segs.len().saturating_sub(1)] {
                let Some(module) = segs[..take].last().copied() else { continue };
                let mut dir: PathBuf = segs[..take - 1].iter().collect();
                let mut found = None;
                for candidate in [{ dir.push(format!("{module}.wj.meta")); dir.clone() }, {
                    let mut d: PathBuf = segs[..take].iter().collect();
                    d.push("mod.wj.meta");
                    d
                }] {
                    if let Some(meta) = crate::metadata::read_current_cached_meta(root, &candidate) {
                        found = Some(meta);
                        break;
                    }
                }
                let Some(meta) = found else { continue };
                for (name, ty) in &meta.consts {
                    if let Some(ty) = ModuleMetadata::deserialize_type(ty) {
                        self.insert(&format!("{module}.wj"), name.clone(), ty);
                    }
                }
                break;
            }
        }
    }

    /// Names whose declarations agree, with their types (what codegen imports across modules).
    pub fn agreed(&self) -> HashMap<String, Type> {
        self.global.iter().filter_map(|(n, t)| t.clone().map(|t| (n.clone(), t))).collect()
    }
}

fn file_is_module(file: &str, module: &str) -> bool {
    let path = std::path::Path::new(file);
    let stem = path.file_stem().and_then(|s| s.to_str());
    match stem {
        Some("mod") => path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some(module),
        Some(s) => s == module,
        None => false,
    }
}
