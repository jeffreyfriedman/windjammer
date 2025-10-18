struct Value {
}

struct Object {
}

struct Array {
}

impl Value {
#[inline]
fn is_object(self) -> bool {
        false
}
#[inline]
fn is_array(self) -> bool {
        false
}
#[inline]
fn is_string(self) -> bool {
        false
}
#[inline]
fn is_number(self) -> bool {
        false
}
#[inline]
fn is_bool(self) -> bool {
        false
}
#[inline]
fn is_null(self) -> bool {
        false
}
#[inline]
fn as_object(self) -> Option<Object> {
        None
}
#[inline]
fn as_array(self) -> Option<Array> {
        None
}
#[inline]
fn as_string(self) -> Option<String> {
        None
}
#[inline]
fn as_i64(self) -> Option<i64> {
        None
}
#[inline]
fn as_bool(self) -> Option<bool> {
        None
}
#[inline]
fn get(self, key: &String) -> Option<Value> {
        None
}
}

impl Object {
#[inline]
fn get(self, key: &String) -> Option<Value> {
        None
}
#[inline]
fn keys(self) -> Vec<String> {
        vec![]
}
#[inline]
fn len(self) -> i64 {
        0
}
}

impl Array {
#[inline]
fn get(self, key: &String) -> Option<Value> {
        None
}
#[inline]
fn len(self) -> i64 {
        0
}
}

#[inline]
fn parse(s: &String) -> Result<Value, String> {
    Err("JSON parsing requires serde_json (auto-added)")
}

#[inline]
fn stringify<T>(value: &T) -> Result<String, String> {
    Err("JSON stringify requires serde_json (auto-added)")
}

#[inline]
fn pretty<T>(value: &T) -> Result<String, String> {
    Err("JSON pretty print requires serde_json (auto-added)")
}

