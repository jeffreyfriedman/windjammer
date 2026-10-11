//! `std::yaml`, `std::compress`, `std::jwt`, password hashing, `std::testing` assertions,
//! `std::async` sleeps and `std::sync` shared ints. Same crates and semantics as `windjammer-runtime`.

use super::*;
use crate::interpreter::locks::CellState;

fn s(v: &str) -> Value {
    Value::str(v.to_string())
}

fn opt_str(v: Value) -> Option<String> {
    match v.as_option() {
        Some(Some(Value::Str(t))) => Some((**t).clone()),
        _ => None,
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Claims {
    sub: String,
    tenant_slug: String,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    entity_id: Option<String>,
    #[serde(default)]
    email: Option<String>,
    exp: i64,
}

fn sign(it: &Interpreter, mut claims: Claims, secret: &str, ttl: i64) -> Value {
    if secret.is_empty() {
        return Value::err(s("JWT secret is empty"));
    }
    if ttl <= 0 {
        return Value::err(s("JWT ttl must be positive"));
    }
    claims.exp = (it.wall_ns() / 1_000_000_000) as i64 + ttl;
    match jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    ) {
        Ok(t) => Value::ok(s(&t)),
        Err(e) => Value::err(s(&e.to_string())),
    }
}

pub fn register(r: &mut Registry) {
    use base64::{engine::general_purpose, Engine as _};
    r.func("yaml::parse", |_, a| {
        let text = arg_str(&a, 0, "parse")?;
        if text.trim().is_empty() {
            return Ok(Value::err(s("empty yaml")));
        }
        Ok(match serde_yaml::from_str::<serde_json::Value>(&text) {
            Ok(v) => Value::ok(s(&serde_json::to_string(&v).unwrap_or_default())),
            Err(e) => Value::err(s(&format!("yaml parse: {e}"))),
        })
    });
    r.func("yaml::to_json", |_, a| {
        let text = arg_str(&a, 0, "to_json")?;
        if text.trim().is_empty() {
            return Ok(Value::err(s("empty yaml")));
        }
        Ok(match serde_yaml::from_str::<serde_json::Value>(&text) {
            Ok(v) => Value::ok(s(&serde_json::to_string(&v).unwrap_or_default())),
            Err(e) => Value::err(s(&format!("yaml parse: {e}"))),
        })
    });

    r.func("compress::gzip_encode", |_, a| {
        use std::io::Write;
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let res = enc.write_all(arg_str(&a, 0, "gzip_encode")?.as_bytes()).and_then(|_| enc.finish());
        Ok(match res {
            Ok(b) => Value::ok(s(&general_purpose::STANDARD.encode(b))),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    for n in ["compress::gzip_decode", "compress::gunzip"] {
        r.func(n, |_, a| {
            use std::io::Read;
            let bytes = match general_purpose::STANDARD.decode(arg_str(&a, 0, "gzip_decode")?.as_bytes()) {
                Ok(b) => b,
                Err(e) => return Ok(Value::err(s(&e.to_string()))),
            };
            let mut plain = String::new();
            Ok(match flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut plain) {
                Ok(_) => Value::ok(s(&plain)),
                Err(e) => Value::err(s(&e.to_string())),
            })
        });
    }

    r.func("crypto::hash_password", |_, a| {
        Ok(match bcrypt::hash(arg_str(&a, 0, "hash_password")?.as_str(), bcrypt::DEFAULT_COST) {
            Ok(h) => Value::ok(s(&h)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("crypto::hash_password_with_cost", |_, a| {
        Ok(match bcrypt::hash(arg_str(&a, 0, "hash_password_with_cost")?.as_str(), arg_int(&a, 1, "hash_password_with_cost")? as u32) {
            Ok(h) => Value::ok(s(&h)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    r.func("crypto::verify_password", |_, a| {
        Ok(match bcrypt::verify(arg_str(&a, 0, "verify_password")?.as_str(), arg_str(&a, 1, "verify_password")?.as_str()) {
            Ok(b) => Value::ok(Value::Bool(b)),
            Err(e) => Value::err(s(&e.to_string())),
        })
    });

    r.func("jwt::sign_hs256", |it, a| {
        let claims = Claims {
            sub: arg_str(&a, 0, "sign_hs256")?.to_string(),
            tenant_slug: arg_str(&a, 1, "sign_hs256")?.to_string(),
            tenant_id: None,
            entity_id: None,
            email: None,
            exp: 0,
        };
        Ok(sign(it, claims, &arg_str(&a, 2, "sign_hs256")?, arg_int(&a, 3, "sign_hs256")? as i64))
    });
    r.func("jwt::sign_hs256_scoped", |it, a| {
        let claims = Claims {
            sub: arg_str(&a, 0, "sign_hs256_scoped")?.to_string(),
            tenant_slug: arg_str(&a, 1, "sign_hs256_scoped")?.to_string(),
            tenant_id: Some(arg_str(&a, 2, "sign_hs256_scoped")?.to_string()),
            entity_id: Some(arg_str(&a, 3, "sign_hs256_scoped")?.to_string()),
            email: Some(arg_str(&a, 4, "sign_hs256_scoped")?.to_string()),
            exp: 0,
        };
        Ok(sign(it, claims, &arg_str(&a, 5, "sign_hs256_scoped")?, arg_int(&a, 6, "sign_hs256_scoped")? as i64))
    });
    r.func("jwt::verify_hs256", |it, a| {
        let (token, secret) = (arg_str(&a, 0, "verify_hs256")?, arg_str(&a, 1, "verify_hs256")?);
        if secret.is_empty() {
            return Ok(Value::err(s("JWT secret is empty")));
        }
        if token.trim().is_empty() {
            return Ok(Value::err(s("JWT token is empty")));
        }
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.validate_exp = false; // checked below against the interpreter clock
        let data = jsonwebtoken::decode::<Claims>(token.trim(), &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()), &validation);
        Ok(match data {
            Ok(d) => {
                let now = (it.wall_ns() / 1_000_000_000) as i64;
                if d.claims.exp < now {
                    return Ok(Value::err(s("ExpiredSignature")));
                }
                let c = d.claims;
                let opt = |o: Option<String>| Value::option(o.map(|t| s(&t)));
                Value::ok(Value::Struct(Rc::new(StructVal {
                    name: Rc::from("JwtClaims"),
                    fields: vec![
                        (Rc::from("sub"), s(&c.sub)),
                        (Rc::from("tenant_slug"), s(&c.tenant_slug)),
                        (Rc::from("tenant_id"), opt(c.tenant_id)),
                        (Rc::from("entity_id"), opt(c.entity_id)),
                        (Rc::from("email"), opt(c.email)),
                        (Rc::from("exp"), Value::Int(c.exp as i128, IntTy::I64)),
                    ],
                    copy: false,
                    uid: 0,
                })))
            }
            Err(e) => Value::err(s(&e.to_string())),
        })
    });
    let _ = opt_str;

    // std::testing
    r.func("testing::assert", |_, a| {
        if matches!(arg(&a, 0, "assert")?, Value::Bool(true)) {
            Ok(Value::Unit)
        } else {
            Err(panic_ctl("Assertion failed"))
        }
    });
    r.func("testing::assert_with_message", |it, a| {
        if matches!(arg(&a, 0, "assert_with_message")?, Value::Bool(true)) {
            Ok(Value::Unit)
        } else {
            Err(panic_ctl(it.display_string(&arg(&a, 1, "assert_with_message")?)?))
        }
    });
    r.func("testing::fail", |it, a| Err(panic_ctl(it.display_string(&arg(&a, 0, "fail")?)?)));
    r.func("testing::assert_approx_eq", |it, a| {
        let (l, rr, e) = (arg_f64(&a, 0, "assert_approx_eq")?, arg_f64(&a, 1, "assert_approx_eq")?, arg_f64(&a, 2, "assert_approx_eq")?);
        let d = if l > rr { l - rr } else { rr - l };
        if d > e {
            let (fl, fr, fe) = (
                it.display_string(&Value::float(l))?,
                it.display_string(&Value::float(rr))?,
                it.display_string(&Value::float(e))?,
            );
            return Err(panic_ctl(format!("Assertion failed: {fl} not approximately equal to {fr} (epsilon: {fe})")));
        }
        Ok(Value::Unit)
    });

    // std::async: sleeps run on the virtual clock
    for n in ["async::sleep_ms", "async::sleep_ms_blocking"] {
        r.func(n, |it, a| {
            it.sleep_ns(arg_int(&a, 0, "sleep_ms")?.max(0) as u64 * 1_000_000)?;
            Ok(Value::Unit)
        });
    }

    // std::sync shared ints: the handle is the cell itself
    r.func("sync::shared", |_, a| {
        Ok(Value::Cell(CellKind::Atomic, Rc::new(RefCell::new(arg(&a, 0, "shared")?)), Rc::new(CellState::default())))
    });
    r.func("sync::shared_add", |it, a| {
        it.preempt()?;
        let h = a.first().cloned().ok_or_else(|| semantic("shared_add needs a handle"))?;
        if let Value::Cell(_, slot, _) = &h {
            let d = arg_int(&a, 1, "shared_add")?;
            let cur = slot.borrow().clone();
            if let Value::Int(n, t) = cur {
                *slot.borrow_mut() = Value::Int(t.wrap(n + d), t);
            }
        }
        Ok(h)
    });
    r.func("sync::shared_get", |_, a| match a.first() {
        Some(Value::Cell(_, slot, _)) => Ok(slot.borrow().clone()),
        _ => Err(semantic("shared_get needs a handle")),
    });
}
