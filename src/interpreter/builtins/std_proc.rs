//! `std::subprocess` (line-oriented pipes to a child process) and `std::cli` (declarative argument
//! parsing). Same behaviour as `crates/windjammer-runtime` (`subprocess.rs`) and clap.
//!
//! Reads block the interpreter (and every interpreted task) until the child writes a line, as a
//! blocking read does in the compiled program's thread.

use super::*;
use crate::interpreter::native::{downcast, Native};
use std::any::Any;
use std::io::{BufRead, BufReader, Write};

/// One running child: stdin for `write_line`, a buffered stdout for `read_line`.
pub(crate) struct Session {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

fn handle(id: i128) -> Value {
    Value::Struct(Rc::new(StructVal {
        name: Rc::from("SubprocessHandle"),
        fields: vec![(Rc::from("id"), Value::Int(id, IntTy::U64))],
        copy: true,
        uid: 0,
    }))
}

fn handle_id(v: &Value) -> u64 {
    match v {
        Value::Struct(s) => match s.fields.iter().find(|(n, _)| &**n == "id") {
            Some((_, Value::Int(n, _))) => *n as u64,
            _ => 0,
        },
        _ => 0,
    }
}

fn ok_unit(r: Result<(), String>) -> Value {
    match r {
        Ok(()) => Value::ok(Value::Unit),
        Err(e) => Value::err(Value::str(e)),
    }
}

impl Interpreter {
    fn with_session<T>(&mut self, h: &Value, f: impl FnOnce(&mut Session) -> Result<T, String>) -> Result<T, String> {
        let id = handle_id(h);
        if id == 0 {
            return Err("invalid subprocess handle".to_string());
        }
        let s = self.subprocesses.get_mut(&id).ok_or_else(|| "subprocess not found".to_string())?;
        f(s)
    }
}

/// The runtime's line protocol: trailing whitespace trimmed; an empty line or EOF is "stdout closed".
fn read_line(s: &mut Session) -> Result<String, String> {
    let mut buf = String::new();
    match s.stdout.read_line(&mut buf) {
        Ok(0) => Err("stdout closed".to_string()),
        Ok(_) => {
            let line = buf.trim_end().to_string();
            if line.is_empty() {
                Err("stdout closed".to_string())
            } else {
                Ok(line)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

const AGENT_OBS_PREFIX: &str = "@AGENT OBS ";

pub fn register(r: &mut Registry) {
    r.type_name("SubprocessHandle");
    r.func("subprocess::invalid_handle", |_, _| Ok(handle(0)));
    r.func("subprocess::is_valid", |_, a| Ok(Value::Bool(handle_id(&arg(&a, 0, "is_valid")?) != 0)));
    r.func("subprocess::spawn", |it, a| {
        let program = arg_str(&a, 0, "spawn")?;
        let args: Vec<String> = it
            .to_items(arg(&a, 1, "spawn")?)?
            .iter()
            .map(|v| if let Value::Str(s) = v { (**s).clone() } else { String::new() })
            .collect();
        let spawned = std::process::Command::new(program.as_str())
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn();
        let mut child = match spawned {
            Ok(c) => c,
            Err(e) => return Ok(Value::err(Value::str(format!("spawn {}: {}", program, e)))),
        };
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Ok(Value::err(Value::str("stdin unavailable")));
        };
        it.next_subprocess += 1;
        let id = it.next_subprocess;
        it.subprocesses.insert(id, Session { child, stdin, stdout: BufReader::new(stdout) });
        Ok(Value::ok(handle(id as i128)))
    });
    r.func("subprocess::write_line", |it, a| {
        let line = arg_str(&a, 1, "write_line")?;
        Ok(ok_unit(it.with_session(&arg(&a, 0, "write_line")?, |s| {
            writeln!(s.stdin, "{}", line).map_err(|e| e.to_string())?;
            s.stdin.flush().map_err(|e| e.to_string())
        })))
    });
    r.func("subprocess::read_line", |it, a| {
        Ok(match it.with_session(&arg(&a, 0, "read_line")?, read_line) {
            Ok(l) => Value::ok(Value::str(l)),
            Err(e) => Value::err(Value::str(e)),
        })
    });
    r.func("subprocess::read_line_until_prefix", |it, a| {
        let prefix = arg_str(&a, 1, "read_line_until_prefix")?;
        let h = arg(&a, 0, "read_line_until_prefix")?;
        Ok(match it.with_session(&h, |s| loop {
            let line = read_line(s)?;
            if line.starts_with(prefix.as_str()) {
                return Ok(line);
            }
        }) {
            Ok(l) => Value::ok(Value::str(l)),
            Err(e) => Value::err(Value::str(e)),
        })
    });
    r.func("subprocess::frames_in_agent_command", |_, a| {
        let cmd = arg_str(&a, 0, "frames_in_agent_command")?;
        for part in cmd.split(';') {
            if let Some(v) = part.trim().strip_prefix("frames=") {
                if let Ok(n) = v.parse::<i64>() {
                    return Ok(Value::Int(n.max(1) as i128, IntTy::I64));
                }
            }
        }
        Ok(Value::Int(1, IntTy::I64))
    });
    r.func("subprocess::read_agent_obs_n", |it, a| {
        let n = arg_int(&a, 1, "read_agent_obs_n")?.max(1);
        let h = arg(&a, 0, "read_agent_obs_n")?;
        Ok(match it.with_session(&h, |s| {
            let mut last = String::new();
            for _ in 0..n {
                let line = loop {
                    let l = read_line(s)?;
                    if l.starts_with(AGENT_OBS_PREFIX) {
                        break l;
                    }
                };
                last = line.strip_prefix(AGENT_OBS_PREFIX).unwrap_or(&line).trim().to_string();
            }
            Ok(last)
        }) {
            Ok(l) => Value::ok(Value::str(l)),
            Err(e) => Value::err(Value::str(e)),
        })
    });
    r.func("subprocess::kill", |it, a| Ok(ok_unit(it.with_session(&arg(&a, 0, "kill")?, |s| s.child.kill().map_err(|e| e.to_string())))));
    r.func("subprocess::wait", |it, a| {
        Ok(match it.with_session(&arg(&a, 0, "wait")?, |s| s.child.wait().map_err(|e| e.to_string())) {
            Ok(st) => Value::ok(Value::Int(st.code().unwrap_or(-1) as i128, IntTy::I32)),
            Err(e) => Value::err(Value::str(e)),
        })
    });
    r.func("subprocess::close", |it, a| {
        let id = handle_id(&arg(&a, 0, "close")?);
        if id != 0 {
            it.subprocesses.remove(&id);
        }
        Ok(Value::ok(Value::Unit))
    });
    register_cli(r);
}

// ======================================================================= cli

#[derive(Clone)]
struct ArgSpec {
    name: String,
    short: Option<char>,
    long: Option<String>,
    help: String,
    required: bool,
    flag: bool,
}

struct CliApp {
    name: String,
    version: String,
    about: String,
    args: Vec<ArgSpec>,
}

struct CliMatches(clap::ArgMatches, Vec<ArgSpec>);

impl Native for CliApp {
    fn type_name(&self) -> &'static str {
        "CliApp"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl Native for CliMatches {
    fn type_name(&self) -> &'static str {
        "CliMatches"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn app_of(v: &Value) -> R<&CliApp> {
    downcast::<CliApp>(v).ok_or_else(|| semantic("expected a CliApp"))
}

fn opt_char(v: Value) -> Option<char> {
    match v.as_option() {
        Some(Some(Value::Char(c))) => Some(*c),
        _ => None,
    }
}

fn opt_string(v: Value) -> Option<String> {
    match v.as_option() {
        Some(Some(Value::Str(s))) => Some((**s).clone()),
        _ => None,
    }
}

fn with_arg(v: &Value, spec: ArgSpec) -> R {
    let a = app_of(v)?;
    let mut args = a.args.clone();
    args.push(spec);
    Ok(Value::Native(Rc::new(CliApp { name: a.name.clone(), version: a.version.clone(), about: a.about.clone(), args })))
}

fn register_cli(r: &mut Registry) {
    for t in ["CliApp", "CliMatches"] {
        r.type_name(t);
    }
    r.func("cli::app", |_, a| {
        Ok(Value::Native(Rc::new(CliApp {
            name: arg_str(&a, 0, "app")?.to_string(),
            version: arg_str(&a, 1, "app")?.to_string(),
            about: arg_str(&a, 2, "app")?.to_string(),
            args: Vec::new(),
        })))
    });
    r.native("CliApp", "arg", |_, v, a| {
        with_arg(v, ArgSpec {
            name: arg_str(&a, 0, "arg")?.to_string(),
            short: opt_char(arg(&a, 1, "arg")?),
            long: opt_string(arg(&a, 2, "arg")?),
            help: arg_str(&a, 3, "arg")?.to_string(),
            required: matches!(arg(&a, 4, "arg")?, Value::Bool(true)),
            flag: false,
        })
    });
    r.native("CliApp", "flag", |_, v, a| {
        with_arg(v, ArgSpec {
            name: arg_str(&a, 0, "flag")?.to_string(),
            short: opt_char(arg(&a, 1, "flag")?),
            long: opt_string(arg(&a, 2, "flag")?),
            help: arg_str(&a, 3, "flag")?.to_string(),
            required: false,
            flag: true,
        })
    });
    r.native("CliApp", "parse", |it, v, _| {
        let app = app_of(v)?;
        // clap (without its `string` feature) takes `&'static str`, as in the runtime: leak the specs
        let leak = |s: &String| -> &'static str { Box::leak(s.clone().into_boxed_str()) };
        let mut cmd = clap::Command::new(leak(&app.name)).version(leak(&app.version)).about(leak(&app.about));
        for s in &app.args {
            let mut arg = clap::Arg::new(leak(&s.name)).help(leak(&s.help));
            if let Some(c) = s.short {
                arg = arg.short(c);
            }
            if let Some(l) = &s.long {
                arg = arg.long(leak(l));
            }
            arg = if s.flag { arg.action(clap::ArgAction::SetTrue) } else { arg.required(s.required) };
            cmd = cmd.arg(arg);
        }
        let mut argv = vec![app.name.clone()];
        argv.extend(it.args.iter().cloned());
        match cmd.try_get_matches_from(argv) {
            Ok(m) => Ok(Value::Native(Rc::new(CliMatches(m, app.args.clone())))),
            Err(e) => {
                // clap's own exit behaviour: help/version on stdout with status 0, errors on stderr with 2
                let text = e.render().to_string();
                let code = e.exit_code();
                if code == 0 {
                    it.write_out(&text);
                } else {
                    it.write_err(&text);
                }
                Err(crate::interpreter::error::RuntimeError::new(crate::interpreter::error::ErrorKind::Exit(code), "cli exit").into())
            }
        }
    });
    fn matches(v: &Value) -> R<&CliMatches> {
        downcast::<CliMatches>(v).ok_or_else(|| semantic("expected CliMatches"))
    }
    r.native("CliMatches", "get", |_, v, a| {
        let m = matches(v)?;
        let name = arg_str(&a, 0, "get")?;
        if m.1.iter().any(|s| s.flag && s.name == *name) {
            return Ok(Value::none());
        }
        Ok(Value::option(m.0.try_get_one::<String>(name.as_str()).ok().flatten().map(|s| Value::str(s.clone()))))
    });
    r.native("CliMatches", "is_present", |_, v, a| {
        let m = matches(v)?;
        let name = arg_str(&a, 0, "is_present")?;
        if m.1.iter().any(|s| s.flag && s.name == *name) {
            return Ok(Value::Bool(m.0.get_flag(name.as_str())));
        }
        Ok(Value::Bool(m.0.try_contains_id(name.as_str()).unwrap_or(false) && m.0.value_source(name.as_str()).is_some()))
    });
    r.native("CliMatches", "get_many", |_, v, a| {
        let m = matches(v)?;
        let name = arg_str(&a, 0, "get_many")?;
        Ok(Value::vec(
            m.0.try_get_many::<String>(name.as_str())
                .ok()
                .flatten()
                .map(|vals| vals.map(|s| Value::str(s.clone())).collect())
                .unwrap_or_default(),
        ))
    });
}
