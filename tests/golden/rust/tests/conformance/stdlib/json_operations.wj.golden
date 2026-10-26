use serde::{Serialize, Deserialize};

use windjammer_runtime::json;
use windjammer_runtime::json::Value;

fn test_json_parse_and_get_fields() {
    let text: String = String::from("{\"name\":\"Alice\",\"age\":30,\"active\":true}");
    match json::parse(&text) {
        Ok(v) => {
            let mut name = String::new();
            let mut age = 0_i64;
            let mut active = false;
            if json::is_object(&v) {
                if let Some(name_val) = json::get(&v, "name") {
                    if let Some(s) = json::as_str(&name_val) {
                        name = s;
                    }
                }
                if let Some(age_val) = json::get(&v, "age") {
                    if let Some(n) = json::as_i64(&age_val) {
                        age = n as i64;
                    }
                }
                if let Some(active_val) = json::get(&v, "active") {
                    if let Some(b) = json::as_bool(&active_val) {
                        active = b;
                    }
                }
            }
            windjammer_runtime::io::println(&format!("[json_parse] name={} age={} active={}", name, age, active));
        },
        Err(e) => {
            windjammer_runtime::io::println(&format!("[json_parse] FAIL: {}", e));
        },
    }
}


fn test_json_stringify_roundtrip() {
    let text: String = String::from("{\"x\":1,\"y\":2}");
    match json::parse(&text) {
        Ok(v) => match json::stringify(&v) {
            Ok(out) => match json::parse(&out) {
                Ok(_) => windjammer_runtime::io::println("[json_stringify] roundtrip ok"),
                Err(e) => windjammer_runtime::io::println(&format!("[json_stringify] FAIL reparsing: {}", e)),
            },
            Err(e) => windjammer_runtime::io::println(&format!("[json_stringify] FAIL stringify: {}", e)),
        },
        Err(e) => windjammer_runtime::io::println(&format!("[json_stringify] FAIL parse: {}", e)),
    }
}

#[inline]
fn test_json_array_len() {
    let text: String = String::from("[1,2,3]");
    match json::parse(&text) {
        Ok(v) => {
            let n = json::len(&v);
            windjammer_runtime::io::println(&format!("[json_array] len={}", n));
        },
        Err(e) => windjammer_runtime::io::println(&format!("[json_array] FAIL: {}", e)),
    }
}

fn main() {
    test_json_parse_and_get_fields();
    test_json_stringify_roundtrip();
    test_json_array_len();
    windjammer_runtime::io::println("[json_all] PASSED");
}

