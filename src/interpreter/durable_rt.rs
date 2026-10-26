//! `durable fn` at run time (ADR-0021 section 9, stage E4): a journal of steps.
//!
//! Calling a durable function opens a *run*. Every call in its body that the static pass marked as a step
//! (`ir::effect_facts::durable`, matched by source position) is executed once and its result appended to the run's journal;
//! when the function is called again with the same arguments, steps already in the journal return their recorded result without
//! running, and the first missing step runs live. A run that completed returns its recorded result without running at all.
//!
//! The journal is JSON Lines, one file per run in `--journal DIR` (or in memory without it):
//! `{"wj_durable":1,"fn":..,"args":..}`, then `{"step":N,"callee":..,"result":..,"vt_ns":..}` per step, then `{"done":..}`.
//! Delivery is at-least-once: a crash between performing a step and journaling it re-executes it. After each step the
//! random stream is re-derived from (run seed, step number) and the virtual clock is restored from the journal, so a
//! resumed run draws the same numbers and reads the same times as an uninterrupted one in a deterministic world.

use super::effect_port::{codec, log};
use super::engine::{FuncDef, Interpreter};
use super::error::{Ctl, ErrorKind, RuntimeError};
use super::value::Value;
use crate::parser::ast::core::Expression;
use crate::parser::ast::types::Type;
use rand::SeedableRng;
use serde_json::{json, Value as Json};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

type R = Result<Value, Ctl>;

/// Where to journal and when to simulate a crash.
#[derive(Clone, Debug, Default)]
pub struct DurableCfg {
    pub dir: Option<PathBuf>,
    /// Exit abruptly (status 137) after this many steps were journaled in the process; with `mid`, after performing the step
    /// but before journaling it (the at-least-once window).
    pub crash_after: Option<usize>,
    pub crash_mid: bool,
}

impl DurableCfg {
    /// `--crash-after N` or `N:mid`.
    pub fn parse_crash(text: &str) -> Result<(usize, bool), String> {
        let (n, mid) = match text.split_once(':') {
            Some((n, "mid")) => (n, true),
            Some(_) => return Err(format!("--crash-after takes N or N:mid, found {text:?}")),
            None => (text, false),
        };
        n.trim().parse::<usize>().map(|n| (n, mid)).map_err(|_| format!("--crash-after takes N or N:mid, found {text:?}"))
    }
}

struct Run {
    path: Option<PathBuf>,
    file: Option<std::fs::File>,
    steps: HashMap<usize, (String, Json, u64)>,
    next: usize,
    seed: u64,
}

#[derive(Default)]
pub struct DurableRt {
    /// Step positions by (line, column); the file is checked against `files`.
    sites: HashMap<(usize, usize), Vec<PathBuf>>,
    cfg: DurableCfg,
    appended: usize,
    runs: Vec<Run>,
    in_step: u32,
    occurrences: HashMap<String, usize>,
    canon: HashMap<PathBuf, PathBuf>,
    base_seed: u64,
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn fail(msg: impl Into<String>) -> Ctl {
    Ctl::Err(Box::new(RuntimeError::new(ErrorKind::Semantic, msg)))
}

fn mix(a: u64, b: u64) -> u64 {
    let mut x = a ^ b.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x.wrapping_mul(0x94D0_49BB_1331_11EB) ^ (x >> 31)
}

fn is_durable(f: &FuncDef) -> bool {
    f.decorators.iter().any(|d| d.name == "durable")
}

/// The positions of the journaled steps of the program rooted at `entry` (empty when it has no `durable fn`).
pub fn discover_sites(entry: &Path) -> Result<Vec<(PathBuf, usize, usize)>, String> {
    use crate::check::effects::{analyze_project, parse_app_inputs};
    let root = super::caps::project_root(entry);
    let target = if root.join("wj.toml").is_file() { root } else { entry.to_path_buf() };
    let files = crate::check::files::collect(&target).map_err(|e| e.to_string())?;
    let mentions = files.iter().any(|f| std::fs::read_to_string(f).is_ok_and(|t| t.contains("durable fn")));
    if !mentions {
        return Ok(Vec::new());
    }
    let inputs = parse_app_inputs(&target).map_err(|e| e.to_string())?;
    let a = analyze_project(&target, inputs, false);
    let d = crate::ir::effect_facts::durable::analyze(&a.resolved, &a.facts);
    Ok(d.step_sites.into_iter().collect())
}

impl Interpreter {
    /// Install the journal for this run. `sites` come from [`discover_sites`].
    pub fn enable_durable(&mut self, sites: Vec<(PathBuf, usize, usize)>, cfg: DurableCfg) {
        let mut rt = DurableRt { cfg, ..DurableRt::default() };
        for (f, l, c) in sites {
            rt.sites.entry((l, c)).or_default().push(canonical(&f));
        }
        self.durable = Some(Box::new(rt));
    }

    pub(crate) fn durable_enabled(&self) -> bool {
        self.durable.as_ref().is_some_and(|d| !d.runs.is_empty() && d.in_step == 0)
    }

    /// Is `e` a journaled step of the durable function being run?
    pub(crate) fn is_durable_step(&mut self, e: &'static Expression<'static>) -> bool {
        let Some(rt) = self.durable.as_mut() else { return false };
        let loc = match e {
            Expression::Call { location, .. } | Expression::MethodCall { location, .. } | Expression::MacroInvocation { location, .. } => location,
            _ => return false,
        };
        let Some(loc) = loc else { return false };
        let Some(files) = rt.sites.get(&(loc.line, loc.column)) else { return false };
        let file = rt.canon.entry(loc.file.clone()).or_insert_with(|| canonical(&loc.file)).clone();
        files.contains(&file)
    }

    /// Call a `durable fn`: replay its journal, run the rest live.
    pub(crate) fn call_durable(&mut self, f: &Rc<FuncDef>, args: Vec<Value>, self_val: Option<Value>, targs: &[Type], expected: Option<Type>) -> R {
        let rt = self.durable.get_or_insert_with(Default::default);
        if rt.base_seed == 0 {
            rt.base_seed = if self.sched.deterministic { 0x5EED } else { rand::random::<u64>() | 1 };
        }
        let arg_json = args
            .iter()
            .map(codec::encode)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|what| fail(format!("durable fn `{}`: an argument is {what}, which cannot be journaled (WJ0045)", f.name)))?;
        let key = format!("{}|{}", f.qual, log::canon(&Json::Array(arg_json.clone())));
        let occurrence = {
            let n = rt.occurrences.entry(key.clone()).or_insert(0);
            *n += 1;
            *n - 1
        };
        let id = format!("{}-{}", f.qual.replace("::", "."), &log::digest(format!("{key}#{occurrence}").as_bytes())[..12]);
        let seed = mix(rt.base_seed, u64::from_str_radix(&id[id.len() - 12..], 16).unwrap_or(0));
        let mut run = Run { path: None, file: None, steps: HashMap::new(), next: 0, seed };
        if let Some(dir) = rt.cfg.dir.clone() {
            std::fs::create_dir_all(&dir).map_err(|e| fail(format!("cannot create the journal directory {}: {e}", dir.display())))?;
            let path = dir.join(format!("{id}.wjlog"));
            if let Ok(text) = std::fs::read_to_string(&path) {
                for line in text.lines().skip(1) {
                    let Ok(j) = serde_json::from_str::<Json>(line) else { continue };
                    if let Some(done) = j.get("done") {
                        return codec::decode(done).map_err(fail);
                    }
                    if let Some(n) = j.get("step").and_then(|n| n.as_u64()) {
                        run.steps.insert(n as usize, (j["callee"].as_str().unwrap_or("").to_string(), j["result"].clone(), j["vt_ns"].as_u64().unwrap_or(0)));
                    }
                }
            }
            let fresh = !path.exists();
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| fail(format!("cannot open the journal {}: {e}", path.display())))?;
            if fresh {
                let head = log::canon(&json!({ "wj_durable": 1, "fn": &*f.qual, "args": arg_json }));
                writeln!(file, "{head}").map_err(|e| fail(e.to_string()))?;
            }
            run.path = Some(path);
            run.file = Some(file);
        }
        self.durable.as_mut().expect("durable runtime").runs.push(run);
        let result = self.call_func_walk(f, args, self_val, targs, expected);
        let run = self.durable.as_mut().expect("durable runtime").runs.pop().expect("durable run");
        let v = result?;
        let done = codec::encode(&v).map_err(|what| fail(format!("durable fn `{}` returns {what}, which cannot be journaled (WJ0045)", f.name)))?;
        if let Some(mut file) = run.file {
            let line = log::canon(&json!({ "done": done }));
            writeln!(file, "{line}").and_then(|_| file.flush()).map_err(|e| fail(format!("cannot write the journal: {e}")))?;
        }
        Ok(v)
    }

    fn crash_now(&mut self) -> ! {
        self.flush();
        std::process::exit(137)
    }

    /// Run or replay the step `e`.
    pub(crate) fn durable_step(&mut self, e: &'static Expression<'static>) -> R {
        let callee = match e {
            Expression::Call { function, .. } => match function {
                Expression::Identifier { name, .. } => name.clone(),
                _ => "<expr>".to_string(),
            },
            Expression::MethodCall { method, .. } => method.clone(),
            Expression::MacroInvocation { name, .. } => name.clone(),
            _ => String::new(),
        };
        let rt = self.durable.as_mut().expect("durable runtime");
        let run = rt.runs.last_mut().expect("durable run");
        let idx = run.next;
        run.next += 1;
        let seed = mix(run.seed, idx as u64 + 1);
        if let Some((name, result, vt)) = run.steps.get(&idx).cloned() {
            if name != callee {
                return Err(fail(format!(
                    "WJ0049: durable replay diverged at step {idx}: the journal has `{name}`, the program now calls `{callee}`\n  help: delete the journal to start over, or restore the program that wrote it"
                )));
            }
            self.sched.clock = self.sched.clock.max(vt);
            return codec::decode(&result).map_err(fail);
        }
        // live: the step's random numbers are a function of (run, step), whatever ran before in this process
        self.rng = Some(rand::rngs::StdRng::seed_from_u64(seed));
        self.durable.as_mut().expect("durable runtime").in_step += 1;
        let r = self.eval_keep_ref(e);
        self.durable.as_mut().expect("durable runtime").in_step -= 1;
        let v = super::place::strip_refs(r?)?;
        let result = codec::encode(&v).map_err(|what| fail(format!("step `{callee}` returns {what}, which cannot be journaled (WJ0045)")))?;
        let vt = self.sched.clock;
        let (crash_mid_now, crash_after_now) = {
            let rt = self.durable.as_ref().expect("durable runtime");
            (rt.cfg.crash_mid && rt.cfg.crash_after == Some(rt.appended + 1), !rt.cfg.crash_mid && rt.cfg.crash_after == Some(rt.appended + 1))
        };
        if crash_mid_now {
            self.crash_now();
        }
        let rt = self.durable.as_mut().expect("durable runtime");
        let run = rt.runs.last_mut().expect("durable run");
        if let Some(file) = run.file.as_mut() {
            let line = log::canon(&json!({ "step": idx, "callee": callee, "result": result, "vt_ns": vt }));
            writeln!(file, "{line}").and_then(|_| file.flush()).map_err(|e| fail(format!("cannot write the journal: {e}")))?;
        }
        run.steps.insert(idx, (callee, result, vt));
        rt.appended += 1;
        if crash_after_now {
            self.crash_now();
        }
        Ok(v)
    }
}

/// The sites as a set, for tests.
pub fn site_set(sites: &[(PathBuf, usize, usize)]) -> HashSet<(usize, usize)> {
    sites.iter().map(|(_, l, c)| (*l, *c)).collect()
}
