//! The simulated world (ADR-0021 E3): everything a program can observe that is not a function of the program itself is a
//! function of one seed. Time is the interpreter's virtual clock (it starts at Unix time 1_700_000_000 and moves only
//! by sleeps and timeouts), random numbers come from the seed, the file system is an in-memory tree, the environment is
//! empty, programs do not exist, the network is the mock table or a refusal, and faults are drawn from a second stream
//! of the same seed. The capability gate runs before any of this, so a simulated run is as confined as a live one.

use super::super::builtins::std_sys::{dirent_value, metadata_from_parts};
use super::super::engine::Interpreter;
use super::super::error::{Ctl, ErrorKind, RuntimeError};
use super::super::int::IntTy;
use super::super::value::Value;
use super::sim_fs::{SimFs, ENOENT};
use crate::ir::effect_facts::sinks;
use rand::{Rng, RngCore, SeedableRng};
use std::collections::BTreeMap;
use std::path::Path;

/// `--simulate` configuration.
#[derive(Clone, Debug)]
pub struct SimConfig {
    pub seed: u64,
    /// Probability that an I/O call fails (per call, drawn from the seeded fault stream). 0 = no faults.
    pub faults: f64,
}

pub struct World {
    pub seed: u64,
    faults: f64,
    frng: rand::rngs::StdRng,
    fs: SimFs,
    env: BTreeMap<String, String>,
    tmp: String,
    /// The one salt stream of password hashing, so a hash is a function of the seed.
    salts: rand::rngs::StdRng,
}

impl World {
    /// `cwd` is where relative paths resolve (the project directory, so the capability gate sees the same paths it would live);
    /// `tmp` is what `env::temp_dir()` returns (the run's granted scratch directory).
    pub fn new(cfg: &SimConfig, cwd: &Path, tmp: &Path) -> World {
        World {
            seed: cfg.seed,
            faults: cfg.faults.clamp(0.0, 1.0),
            frng: rand::rngs::StdRng::seed_from_u64(cfg.seed ^ 0xFA17_FA17_FA17_FA17),
            fs: SimFs::new(&cwd.to_string_lossy()),
            env: BTreeMap::new(),
            tmp: tmp.to_string_lossy().to_string(),
            salts: rand::rngs::StdRng::seed_from_u64(cfg.seed ^ 0x5A17_5A17_5A17_5A17),
        }
    }

    /// Put the host directory `fixtures/` of the project into the tree, so a test can read the inputs it ships with.
    pub fn load_fixtures(&mut self, project: &Path) {
        let fx = project.join("fixtures");
        if fx.is_dir() {
            let at = format!("{}/fixtures", project.display());
            self.fs.load_host_dir(&fx, &at);
        }
        let tmp = self.tmp.clone();
        let _ = self.fs.create_dir_all(&tmp);
    }

    fn fault(&mut self) -> bool {
        self.faults > 0.0 && self.frng.gen::<f64>() < self.faults
    }
}

fn err_str(m: &str) -> Value {
    Value::err(Value::str(m))
}

fn unit(r: Result<(), String>) -> Value {
    match r {
        Ok(()) => Value::ok(Value::Unit),
        Err(e) => err_str(&e),
    }
}

fn bytes_value(b: &[u8]) -> Value {
    Value::vec(b.iter().map(|x| Value::Int(*x as i128, IntTy::U8)).collect())
}

fn text(args: &[Value], i: usize) -> String {
    match args.get(i) {
        Some(Value::Str(s)) => (**s).clone(),
        Some(Value::Ref(p)) => match p.read() {
            Ok(Value::Str(s)) => (*s).clone(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

fn payload(it: &mut Interpreter, v: Option<&Value>) -> Result<Vec<u8>, Ctl> {
    Ok(match v {
        Some(Value::Str(s)) => s.as_bytes().to_vec(),
        Some(other) => it.to_items(other.clone())?.into_iter().map(|x| if let Value::Int(n, _) = x { n as u8 } else { 0 }).collect(),
        None => Vec::new(),
    })
}

/// The table name with the aliases the runtime gives (`path::exists` is `fs::exists`).
fn canonical_op(path: &str) -> String {
    let k = sinks::canonical(path);
    match k.as_str() {
        "path::exists" => "fs::exists",
        "path::is_file" => "fs::is_file",
        "path::is_dir" => "fs::is_dir",
        "path::canonicalize" => "fs::canonicalize",
        "fs::read_bytes" => "fs::read",
        "fs::write_string" | "fs::write_bytes" => "fs::write",
        "env::set_var" => "env::set",
        "env::remove_var" => "env::remove",
        other => return other.to_string(),
    }
    .to_string()
}

const EIO: &str = "Input/output error (os error 5)";
const ENOSPC: &str = "No space left on device (os error 28)";
const ECONNRESET: &str = "Connection reset by peer (os error 104)";

fn unsupported(op: &str) -> Ctl {
    Ctl::Err(Box::new(RuntimeError::new(ErrorKind::Unsupported, format!("`{op}` is not available in the simulated world"))))
}

/// Answer one sink call from the world.
pub fn answer(it: &mut Interpreter, path: &str, args: Vec<Value>) -> Result<Value, Ctl> {
    let op = canonical_op(path);
    let a0 = text(&args, 0);
    let a1 = text(&args, 1);
    let data = if matches!(op.as_str(), "fs::write" | "fs::append") { Some(payload(it, args.get(1))?) } else { None };
    if op.starts_with("http::") {
        return net(it, path, args);
    }
    if op == "mcp::call" {
        // the simulated world talks to canned tool results only
        let server = a0.clone();
        if !it.mcp_is_mock(&server) {
            return Ok(err_str(&format!("the simulated world has no mock MCP server `{server}` (bind one with --bind {server}=mock:FILE)")));
        }
        return it.call_builtin_live(path, args);
    }
    let port = it.port.as_ref().expect("simulation without a port");
    let mut p = port.borrow_mut();
    let w = p.world().expect("simulation without a world");
    Ok(match op.as_str() {
        "fs::read_to_string" | "fs::read" => {
            if w.fault() {
                return Ok(err_str(EIO));
            }
            match w.fs.read(&a0) {
                Err(e) => err_str(&e),
                Ok(b) if op == "fs::read" => Value::ok(bytes_value(&b)),
                Ok(b) => match String::from_utf8(b) {
                    Ok(s) => Value::ok(Value::str(s)),
                    Err(_) => err_str("stream did not contain valid UTF-8"),
                },
            }
        }
        "fs::write" | "fs::append" => {
            let data = data.unwrap_or_default();
            if w.fault() {
                // a short write: the first half reached the disk, then the device filled up
                let half = data[..data.len() / 2].to_vec();
                let _ = if op == "fs::write" { w.fs.write(&a0, half) } else { w.fs.append(&a0, &half) };
                return Ok(err_str(ENOSPC));
            }
            unit(if op == "fs::write" { w.fs.write(&a0, data) } else { w.fs.append(&a0, &data) })
        }
        "fs::copy" | "fs::rename" | "fs::create_dir" | "fs::create_dir_all" | "fs::remove_file" | "fs::remove_dir" | "fs::remove_dir_all" => {
            if w.fault() {
                return Ok(err_str(ENOSPC));
            }
            unit(match op.as_str() {
                "fs::copy" => w.fs.copy(&a0, &a1),
                "fs::rename" => w.fs.rename(&a0, &a1),
                "fs::create_dir" => w.fs.create_dir(&a0),
                "fs::create_dir_all" => w.fs.create_dir_all(&a0),
                "fs::remove_file" => w.fs.remove_file(&a0),
                "fs::remove_dir" => w.fs.remove_dir(&a0),
                _ => w.fs.remove_dir_all(&a0),
            })
        }
        "fs::exists" => Value::Bool(w.fs.exists(&a0)),
        "fs::is_file" => Value::Bool(w.fs.is_file(&a0)),
        "fs::is_dir" => Value::Bool(w.fs.is_dir(&a0)),
        "fs::read_dir" => {
            if w.fault() {
                return Ok(err_str(EIO));
            }
            match w.fs.read_dir(&a0) {
                Ok(names) => {
                    let base = a0.trim_end_matches('/').to_string();
                    Value::ok(Value::vec(names.into_iter().map(|n| dirent_value(format!("{base}/{n}"))).collect()))
                }
                Err(e) => err_str(&e),
            }
        }
        "fs::metadata" => {
            if w.fault() {
                return Ok(err_str(EIO));
            }
            if w.fs.is_file(&a0) {
                Value::ok(metadata_from_parts(w.fs.len(&a0), true, false, false))
            } else if w.fs.is_dir(&a0) {
                Value::ok(metadata_from_parts(4096, false, true, false))
            } else {
                err_str(ENOENT)
            }
        }
        "fs::canonicalize" => {
            if w.fs.exists(&a0) {
                Value::ok(Value::str(w.fs.abs(&a0)))
            } else {
                err_str(ENOENT)
            }
        }
        "fs::set_current_dir" => unit(w.fs.set_cwd(&a0)),
        "fs::current_dir" | "path::current_dir" => Value::ok(Value::str(w.fs.cwd().to_string())),
        "env::current_dir" => Value::str(w.fs.cwd().to_string()),
        "env::temp_dir" => Value::str(w.tmp.clone()),
        "process::pid" => Value::Int(4242, IntTy::I64),
        "env::var" => match w.env.get(&a0) {
            Some(v) => Value::ok(Value::str(v.clone())),
            None => err_str("environment variable not found"),
        },
        "env::get" => Value::option(w.env.get(&a0).map(|v| Value::str(v.clone()))),
        "env::get_or" => Value::str(w.env.get(&a0).cloned().unwrap_or(a1)),
        "env::set" => {
            w.env.insert(a0, a1);
            Value::Unit
        }
        "env::remove" => {
            w.env.remove(&a0);
            Value::Unit
        }
        "env::home_dir" => Value::option(w.env.get("HOME").map(|v| Value::str(v.clone()))),
        "process::run" | "process::run_with_args" | "process::run_with_output" => err_str(ENOENT),
        "io::read_line" | "io::read_to_string" => Value::ok(Value::str("")),
        "log::init" | "log::init_with_level" => Value::Unit,
        "crypto::hash_password" | "crypto::hash_password_with_cost" => {
            let mut salt = [0u8; 16];
            w.salts.fill_bytes(&mut salt);
            let cost = if op == "crypto::hash_password" { bcrypt::DEFAULT_COST } else { args.get(1).and_then(|c| if let Value::Int(n, _) = c { Some(*n as u32) } else { None }).unwrap_or(bcrypt::DEFAULT_COST) };
            match bcrypt::hash_with_salt(a0.as_str(), cost, salt) {
                Ok(h) => Value::ok(Value::str(h.to_string())),
                Err(e) => err_str(&e.to_string()),
            }
        }
        "crypto::hash_password_argon2id" => {
            use argon2::password_hash::{PasswordHasher, SaltString};
            let mut salt = [0u8; 16];
            w.salts.fill_bytes(&mut salt);
            match SaltString::encode_b64(&salt).map_err(|e| e.to_string()).and_then(|s| argon2::Argon2::default().hash_password(a0.as_bytes(), &s).map(|h| h.to_string()).map_err(|e| e.to_string())) {
                Ok(h) => Value::ok(Value::str(h)),
                Err(e) => err_str(&e),
            }
        }
        _ => return Err(unsupported(&op)),
    })
}

/// Network: the mock table when there is one (or the refusal), with faults in front of it.
fn net(it: &mut Interpreter, path: &str, args: Vec<Value>) -> Result<Value, Ctl> {
    let fault = {
        let mut p = it.port.as_ref().unwrap().borrow_mut();
        let w = p.world().unwrap();
        if w.fault() {
            Some(w.frng.gen::<bool>())
        } else {
            None
        }
    };
    match fault {
        Some(true) => Ok(err_str(ECONNRESET)),
        Some(false) => {
            // a response that never comes: the caller's timeout fires after 30 virtual seconds
            it.sched.clock = it.sched.clock.saturating_add(30_000_000_000);
            Ok(err_str("operation timed out"))
        }
        None => it.call_builtin_live(path, args),
    }
}
