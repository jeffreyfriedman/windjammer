//! JavaScript lowering of language intrinsics (ADR-0014 Phase 1) beyond print/println:
//! stderr output, `panic`/`todo`/`unreachable`, `assert*`, `debug`.

use crate::intrinsics::Intrinsic;
use crate::parser::Expression;

use super::generator::JavaScriptGenerator;

impl JavaScriptGenerator {
    pub(crate) fn try_generate_intrinsic_call(
        &mut self,
        func_name: &str,
        arguments: &[(Option<String>, &Expression)],
    ) -> Option<String> {
        let intr = Intrinsic::from_call_name(func_name)?;
        let args: Vec<String> = arguments
            .iter()
            .map(|(_, a)| self.generate_expression(a))
            .collect();
        let throw = |msg: String| format!("(() => {{ throw new Error({msg}); }})()");
        match intr {
            Intrinsic::Eprintln => Some(format!("console.error({})", args.join(", "))),
            Intrinsic::Eprint => Some(format!(
                "process.stderr.write(String({}))",
                args.first().cloned().unwrap_or_else(|| "''".to_string())
            )),
            Intrinsic::Panic => Some(throw(
                args.first()
                    .cloned()
                    .unwrap_or_else(|| "'explicit panic'".to_string()),
            )),
            Intrinsic::Todo | Intrinsic::Unreachable => {
                let base = intr.default_panic_message().unwrap_or("panic");
                Some(throw(match args.first() {
                    Some(m) => format!("'{base}: ' + {m}"),
                    None => format!("'{base}'"),
                }))
            }
            Intrinsic::Assert if !args.is_empty() => {
                let msg = args
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| "'assertion failed'".to_string());
                Some(format!(
                    "((c) => {{ if (!c) throw new Error({msg}); }})({})",
                    args[0]
                ))
            }
            Intrinsic::AssertEq | Intrinsic::AssertNe if args.len() >= 2 => {
                let (op, what) = if intr == Intrinsic::AssertEq {
                    ("!==", "==")
                } else {
                    ("===", "!=")
                };
                let user_msg = args
                    .get(2)
                    .map(|m| format!(" + ': ' + {m}"))
                    .unwrap_or_default();
                Some(format!(
                    "((l, r) => {{ if (JSON.stringify(l) {op} JSON.stringify(r)) throw new Error('assertion `left {what} right` failed'{user_msg}); }})({}, {})",
                    args[0], args[1]
                ))
            }
            Intrinsic::Debug if args.len() == 1 => Some(format!(
                "((v) => {{ console.error('[debug] ' + JSON.stringify(v)); return v; }})({})",
                args[0]
            )),
            _ => None,
        }
    }
}
