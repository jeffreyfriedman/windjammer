//! The simulated world (filled in below).
use super::super::engine::Interpreter;
use super::super::error::Ctl;
use super::super::value::Value;

pub struct World;

pub fn answer(_it: &mut Interpreter, path: &str, _args: Vec<Value>) -> Result<Value, Ctl> {
    Err(Ctl::Err(Box::new(super::super::error::RuntimeError::new(super::super::error::ErrorKind::Unsupported, format!("`{path}` is not simulated yet")))))
}
