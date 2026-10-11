// Go lowering of language intrinsics (ADR-0014 Phase 1) beyond print/println.
// `include!`d into `generator/mod.rs` (shares its imports and `GoGenerator`).

impl GoGenerator {
    /// Go message expression for a panic/assert message argument.
    fn go_message_expr(&mut self, expr: &Expression) -> String {
        let generated = self.generate_expression(expr);
        match expr {
            Expression::Literal {
                value: Literal::String(_),
                ..
            } => generated,
            Expression::MacroInvocation { name, .. } if name == "format" => generated,
            _ => {
                self.needs_fmt_import = true;
                format!("fmt.Sprint({})", generated)
            }
        }
    }

    fn try_generate_intrinsic_call(
        &mut self,
        name: &str,
        arguments: &[(Option<String>, &Expression)],
    ) -> Option<String> {
        use crate::intrinsics::Intrinsic;
        let intr = Intrinsic::from_call_name(name)?;
        match intr {
            // Go's builtin print/println write to stderr.
            Intrinsic::Eprintln | Intrinsic::Eprint => {
                let builtin = if intr == Intrinsic::Eprintln {
                    "println"
                } else {
                    "print"
                };
                let msg = match arguments.first() {
                    Some((_, e)) => self.go_message_expr(e),
                    None => String::new(),
                };
                Some(format!("{builtin}({msg})"))
            }
            Intrinsic::Panic => {
                let msg = match arguments.first() {
                    Some((_, e)) => self.go_message_expr(e),
                    None => "\"explicit panic\"".to_string(),
                };
                Some(format!("panic({msg})"))
            }
            Intrinsic::Todo | Intrinsic::Unreachable => {
                let base = intr.default_panic_message().unwrap_or("panic");
                Some(match arguments.first() {
                    Some((_, e)) => {
                        let msg = self.go_message_expr(e);
                        format!("panic(\"{base}: \" + {msg})")
                    }
                    None => format!("panic(\"{base}\")"),
                })
            }
            Intrinsic::Assert if !arguments.is_empty() => {
                let cond = self.generate_expression(arguments[0].1);
                let msg = match arguments.get(1) {
                    Some((_, e)) => self.go_message_expr(e),
                    None => "\"assertion failed\"".to_string(),
                };
                Some(format!("func() {{ if !({cond}) {{ panic({msg}) }} }}()"))
            }
            Intrinsic::AssertEq | Intrinsic::AssertNe if arguments.len() >= 2 => {
                self.needs_fmt_import = true;
                let a = self.generate_expression(arguments[0].1);
                let b = self.generate_expression(arguments[1].1);
                let (cmp, what) = if intr == Intrinsic::AssertEq {
                    ("!=", "==")
                } else {
                    ("==", "!=")
                };
                let user = match arguments.get(2) {
                    Some((_, e)) => format!(" + \": \" + {}", self.go_message_expr(e)),
                    None => String::new(),
                };
                Some(format!(
                    "func(l, r interface{{}}) {{ if fmt.Sprint(l) {cmp} fmt.Sprint(r) {{ panic(fmt.Sprintf(\"assertion `left {what} right` failed\\n  left: %v\\n right: %v\", l, r){user}) }} }}({a}, {b})"
                ))
            }
            Intrinsic::Debug if arguments.len() == 1 => {
                self.needs_fmt_import = true;
                self.needs_debug_helper = true;
                let v = self.generate_expression(arguments[0].1);
                Some(format!("wjDebug({v})"))
            }
            _ => None,
        }
    }
}

/// Package-level helper emitted when `debug(x)` is used: print to stderr, return the value.
const GO_DEBUG_HELPER: &str = "func wjDebug[T any](v T) T {\n\tprintln(\"[debug] \" + fmt.Sprintf(\"%+v\", v))\n\treturn v\n}\n";
