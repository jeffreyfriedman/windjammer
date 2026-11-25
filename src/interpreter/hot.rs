//! Hot reload of function bodies for long-running programs (`wj run --hot`, ADR-0024 spike).
//!
//! A watcher thread polls the program's files; the interpreter applies changes at a safe point (function
//! entry), so no running frame is touched: frames already executing keep the body they started with.
//! A changed function or method whose signature is unchanged gets its new body; anything else (a new or
//! removed item, a changed signature, a struct/enum/trait change) is reported as needing a restart and the
//! old code keeps running. Values, globals and heap state are untouched, so program state survives.

use super::engine::Interpreter;
use crate::parser::{FunctionDecl, Item};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub struct HotState {
    pending: Arc<AtomicBool>,
    changed: Arc<Mutex<Vec<PathBuf>>>,
    /// Item shapes (signatures, type layouts; no bodies) per file, as last loaded.
    shapes: HashMap<PathBuf, Vec<String>>,
}

fn mtime(p: &PathBuf) -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// Text that must stay equal for a body swap to be safe: parameters (names, types, ownership) and return type.
fn sig(name: &str, params: &[crate::parser::Parameter<'_>], ret: Option<&crate::parser::Type>, is_async: bool) -> String {
    let ps: Vec<String> = params.iter().map(|p| format!("{}:{:?}:{:?}", p.name, p.type_, p.ownership)).collect();
    format!("{}({})->{:?} async={}", name, ps.join(","), ret, is_async)
}

fn signature(d: &FunctionDecl<'_>) -> String {
    sig(&d.name, &d.parameters, d.return_type.as_ref(), d.is_async)
}

fn def_signature(f: &super::engine::FuncDef) -> String {
    sig(&f.name, f.params, f.ret, f.is_async)
}

/// Shape of non-function items (a change here needs a restart).
fn shape(items: &[Item<'_>]) -> Vec<String> {
    let mut v = Vec::new();
    for it in items {
        match it {
            Item::Function { decl, .. } => v.push(format!("fn {}", signature(decl))),
            Item::Impl { block, .. } => {
                for f in &block.functions {
                    v.push(format!("impl {} {:?} fn {}", block.type_name, block.trait_name, signature(f)));
                }
            }
            Item::Struct { decl, .. } => v.push(format!("struct {} {:?}", decl.name, decl.fields.iter().map(|f| (&f.name, &f.field_type)).collect::<Vec<_>>())),
            Item::Enum { decl, .. } => v.push(format!("enum {} {}", decl.name, decl.variants.len())),
            other => v.push(format!("{:?}", std::mem::discriminant(other))),
        }
    }
    v.sort();
    v
}

impl Interpreter {
    /// Start watching the loaded program's files (call after loading, before `call_main`).
    pub fn enable_hot_reload(&mut self) {
        let mut files: Vec<PathBuf> = self.modules.iter().map(|m| m.file.clone()).filter(|f| f.is_file()).collect();
        files.sort();
        files.dedup();
        let shapes = files
            .iter()
            .filter_map(|f| {
                let src = std::fs::read_to_string(f).ok()?;
                let p = super::loader::parse_source(&src, &f.to_string_lossy()).ok()?;
                Some((f.clone(), shape(&p.items)))
            })
            .collect();
        let pending = Arc::new(AtomicBool::new(false));
        let changed = Arc::new(Mutex::new(Vec::new()));
        let (p, c) = (pending.clone(), changed.clone());
        std::thread::spawn(move || {
            let mut last: Vec<Option<(SystemTime, u64)>> = files.iter().map(mtime).collect();
            loop {
                std::thread::sleep(Duration::from_millis(20));
                for (i, f) in files.iter().enumerate() {
                    let now = mtime(f);
                    if now != last[i] {
                        last[i] = now;
                        c.lock().unwrap().push(f.clone());
                        p.store(true, Ordering::Release);
                    }
                }
            }
        });
        self.hot = Some(Box::new(HotState { pending, changed, shapes }));
    }

    /// Safe point (function entry): apply pending edits.
    #[inline]
    pub(crate) fn hot_safe_point(&mut self) {
        let Some(h) = &self.hot else { return };
        if !h.pending.swap(false, Ordering::Acquire) {
            return;
        }
        let files: Vec<PathBuf> = std::mem::take(&mut *h.changed.lock().unwrap());
        for f in files {
            self.reload_file(&f);
        }
    }

    fn reload_file(&mut self, file: &PathBuf) {
        let Some(m) = self.modules.iter().position(|x| &x.file == file) else { return };
        let Ok(src) = std::fs::read_to_string(file) else { return };
        let Ok(program) = super::loader::parse_source(&src, &file.to_string_lossy()) else {
            eprintln!("[wj hot] {}: parse error; keeping the running code", file.display());
            return;
        };
        let program = self.retain_program(program);
        let new_shape = shape(&program.items);
        let restart = self.hot.as_mut().map(|h| h.shapes.insert(file.clone(), new_shape.clone()) != Some(new_shape)).unwrap_or(false);
        let mut swapped = 0;
        for it in &program.items {
            match it {
                Item::Function { decl, .. } => {
                    if let Some(f) = self.modules[m].funcs.get(&decl.name) {
                        if def_signature(f) == signature(decl) {
                            let nf = self.func_from_decl(decl, m, None, &[]);
                            self.modules[m].funcs.insert(decl.name.clone(), nf);
                            swapped += 1;
                        }
                    }
                }
                Item::Impl { block, .. } => {
                    let ty = super::register::base_type_name(&block.type_name);
                    for func in &block.functions {
                        let key = match &block.trait_name {
                            Some(tr) => (ty.clone(), format!("{}::{}", super::register::base_type_name(tr), func.name)),
                            None => (ty.clone(), func.name.clone()),
                        };
                        if self.methods.get(&key).is_some_and(|f| def_signature(f) == signature(func)) {
                            let nf = self.func_from_decl(func, m, Some(&ty), &block.type_params);
                            self.methods.insert(key, nf.clone());
                            if block.trait_name.is_some() {
                                self.methods.insert((ty.clone(), func.name.clone()), nf);
                            }
                            swapped += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        if restart {
            eprintln!("[wj hot] {}: signatures or types changed; bodies swapped where shapes match, restart (`wj run --watch`) to apply the rest", file.display());
        }
        eprintln!("[wj hot] reloaded {} function(s) from {}", swapped, file.display());
    }
}

#[cfg(test)]
mod tests {
    //! Hot reload swaps a body that holds closures and islands: the swapped-in body is compiled from ITS OWN text, so the
    //! names its closures capture are the new body's, never a name set scanned from the body it replaced (the scans of
    //! `resolved::island::ast_text` users: `closure::create`/`compile`, `Island::new`/`with_copies`). No watcher thread and no
    //! sleeps: the tests call the reload entry points themselves, which is what the watcher's thread ends up calling.
    use super::*;
    use crate::interpreter::engine::Interpreter;
    use crate::interpreter::value::Value;

    const V1: &str = "fn compute(n: int) -> int {\n    let base = 10\n    let add = |x| x + base\n    add(n)\n}\n\nfn tally(n: int) -> int {\n    let mut total = 0\n    let k = 5\n    for i in 0..n {\n        total += i + k\n    }\n    total\n}\n\nfn main() {\n    println(\"${compute(1)} ${tally(3)}\")\n}\n";
    // `compute` captures other names (two, one of them used twice); `tally` names other locals
    const V2: &str = "fn compute(n: int) -> int {\n    let scale = 7\n    let bias = 3\n    let f = |x| x * scale + bias\n    f(n) + f(1)\n}\n\nfn tally(n: int) -> int {\n    let mut total = 0\n    let step = 2\n    for i in 0..n {\n        total += i * step\n    }\n    total\n}\n\nfn main() {\n    println(\"${compute(1)} ${tally(3)}\")\n}\n";
    // `compute` captures a third name, and the first one again with another meaning
    const V3: &str = "fn compute(n: int) -> int {\n    let offset = 100\n    let base = 1000\n    let g = |x| x - offset + base\n    g(n)\n}\n\nfn tally(n: int) -> int {\n    let mut total = 0\n    let k = 50\n    for i in 0..n {\n        total += i + k\n    }\n    total\n}\n\nfn main() {\n    println(\"${compute(1)} ${tally(3)}\")\n}\n";

    fn int(v: Result<Value, crate::interpreter::error::Ctl>) -> i128 {
        match v {
            Ok(Value::Int(i, _)) => i,
            Ok(other) => panic!("not an int: {other:?}"),
            Err(_) => panic!("the call failed"),
        }
    }

    /// `compute(5)` and `tally(4)` as the interpreter sees the functions NOW (looked up again: a swap replaces the `FuncDef`).
    fn run(it: &mut Interpreter) -> (i128, i128) {
        let mut one = |name: &str, n: i128| {
            let f = it.modules[0].funcs.get(name).cloned().expect("function");
            int(it.call_func(&f, vec![Value::Int(n, crate::interpreter::int::IntTy::I64)], None))
        };
        (one("compute", 5), one("tally", 4))
    }

    fn loaded(dir: &std::path::Path, engine_resolved: bool) -> (Interpreter, PathBuf) {
        let file = dir.join("main.wj");
        std::fs::write(&file, V1).unwrap();
        let mut it = Interpreter::new_capturing();
        it.set_engine_resolved(engine_resolved);
        it.load_program_at(&file).expect("load");
        let file = it.modules[0].file.clone();
        (it, file)
    }

    // what the three bodies compute, worked out by hand: compute(5), tally(4)
    const OLD: (i128, i128) = (15, 26); // 5 + 10; the sum of i + 5 for i in 0..4
    const NEW: (i128, i128) = (48, 12); // f(5) + f(1) = 38 + 10; the sum of i * 2
    const NEWER: (i128, i128) = (905, 206); // 5 - 100 + 1000; the sum of i + 50

    fn in_a_big_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new().stack_size(256 << 20).spawn(f).unwrap().join().unwrap();
    }

    /// The compiled engine (closure `create` scans the body for the names it captures, islands scan for the names they copy):
    /// each swap is followed by a call that must use the new body's names. The reload is `reload_file`, the watcher's.
    #[test]
    fn a_swapped_body_captures_the_names_of_the_new_text() {
        in_a_big_stack(|| {
            let dir = tempfile::tempdir().unwrap();
            let (mut it, file) = loaded(dir.path(), true);
            assert_eq!(run(&mut it), OLD);
            assert_eq!(run(&mut it), OLD, "a second call of the compiled body");
            for (src, want) in [(V2, NEW), (V3, NEWER), (V1, OLD), (V2, NEW)] {
                std::fs::write(&file, src).unwrap();
                it.reload_file(&file);
                assert_eq!(run(&mut it), want);
                assert_eq!(run(&mut it), want, "a second call of the swapped body");
            }
        });
    }

    /// The same through the safe point the running program reaches (`hot_safe_point` at the next call, with the watcher's
    /// flag raised by hand): the call that finds the edit still runs the body it resolved, the next one runs the new one.
    #[test]
    fn the_safe_point_applies_the_edit_between_calls() {
        in_a_big_stack(|| {
            let dir = tempfile::tempdir().unwrap();
            let (mut it, file) = loaded(dir.path(), true);
            it.hot = Some(Box::new(HotState { pending: Arc::new(AtomicBool::new(false)), changed: Arc::new(Mutex::new(Vec::new())), shapes: HashMap::new() }));
            assert_eq!(run(&mut it), OLD);
            std::fs::write(&file, V2).unwrap();
            let h = it.hot.as_ref().unwrap();
            h.changed.lock().unwrap().push(file.clone());
            h.pending.store(true, Ordering::Release);
            let first = run(&mut it);
            // `compute` resolved its body, then the safe point of its own call swapped the module's table: that call is old; `tally`'s is new
            assert_eq!(first, (OLD.0, NEW.1), "the call that meets the edit keeps the body it resolved");
            assert_eq!(run(&mut it), NEW);
        });
    }
}
