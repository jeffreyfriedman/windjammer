//! `durable fn` at run time (ADR-0021 section 9, stage E4): a journal of steps.
//!
//! Calling a durable function opens a *run*. Every call in its body that the static pass marked as a step
//! (`ir::effect_facts::durable`, matched by source position) is executed once and its result appended to the run's journal;
//! when the function is called again with the same arguments, steps already in the journal return their recorded result without
//! running, and the first missing step runs live. A run that completed returns its recorded result without running at all.
//!
//! The journal itself (file layout, numbering, replay, divergence) is `durable_core`, the same source the compiled runtime
//! uses; this file adapts it to interpreter values and adds the determinism of a resumed run.
//! Delivery is at-least-once: a crash between performing a step and journaling it re-executes it. After each step the
//! random stream is re-derived from (run seed, step number) and the virtual clock is restored from the journal, so a
//! resumed run draws the same numbers and reads the same times as an uninterrupted one in a deterministic world.

use super::durable_core::{self as jc, Opened, RunJournal, Step};
use super::effect_port::codec;
use super::engine::{FuncDef, Interpreter};
use super::error::{Ctl, ErrorKind, RuntimeError};
use super::value::Value;
use crate::parser::ast::core::Expression;
use crate::parser::ast::types::Type;
use rand::SeedableRng;
use serde_json::Value as Json;
use std::collections::HashMap;
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
        jc::parse_crash(text).map_err(|m| format!("--crash-after: {m}"))
    }
}

struct Run {
    journal: RunJournal,
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

impl Interpreter {
    /// Install the journal for this run. `sites` come from `check::durable_sites::discover`.
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
        let key = format!("{}|{}", f.qual, jc::canon(&Json::Array(arg_json.clone())));
        let occurrence = {
            let n = rt.occurrences.entry(key).or_insert(0);
            *n += 1;
            *n - 1
        };
        let id = jc::run_id(&f.qual, &arg_json, occurrence);
        let seed = mix(rt.base_seed, jc::hash64(id.as_bytes()));
        let journal = match RunJournal::open(rt.cfg.dir.as_deref(), &id, &f.qual, &arg_json).map_err(fail)? {
            Opened::Done(result) => return codec::decode(&result).map_err(fail),
            Opened::Run(j) => *j,
        };
        self.durable.as_mut().expect("durable runtime").runs.push(Run { journal, seed });
        let result = self.call_func_body(f, args, self_val, targs, expected);
        let run = self.durable.as_mut().expect("durable runtime").runs.pop().expect("durable run");
        let v = result?;
        let done = codec::encode(&v).map_err(|what| fail(format!("durable fn `{}` returns {what}, which cannot be journaled (WJ0045)", f.name)))?;
        run.journal.finish(&done).map_err(fail)?;
        Ok(v)
    }

    fn crash_now(&mut self) -> ! {
        self.flush();
        std::process::exit(137)
    }

    /// Run or replay the step `e`.
    pub(crate) fn durable_step(&mut self, e: &'static Expression<'static>) -> R {
        let callee = crate::ir::effect_facts::durable::step_callee(e);
        let rt = self.durable.as_mut().expect("durable runtime");
        let run = rt.runs.last_mut().expect("durable run");
        let seed_base = run.seed;
        let idx = match run.journal.next_step(&callee).map_err(fail)? {
            Step::Replay { result, vt_ns, .. } => {
                self.sched.clock = self.sched.clock.max(vt_ns);
                return codec::decode(&result).map_err(fail);
            }
            Step::Live { idx } => idx,
        };
        // live: the step's random numbers are a function of (run, step), whatever ran before in this process
        self.rng = Some(rand::rngs::StdRng::seed_from_u64(mix(seed_base, idx as u64 + 1)));
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
        rt.runs.last_mut().expect("durable run").journal.record(idx, &callee, result, vt).map_err(fail)?;
        rt.appended += 1;
        if crash_after_now {
            self.crash_now();
        }
        Ok(v)
    }
}
