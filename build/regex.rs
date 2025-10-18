struct Regex {
}

struct Match {
    text: String,
    start: i64,
    end: i64,
}

struct Captures {
}

impl Regex {
#[inline]
fn is_match(self, text: &String) -> bool {
        false
}
#[inline]
fn find(self, text: &String) -> Option<Match> {
        None
}
#[inline]
fn find_all(self, text: &String) -> Vec<Match> {
        vec![]
}
#[inline]
fn captures(self, text: &String) -> Option<Captures> {
        None
}
#[inline]
fn captures_all(self, text: &String) -> Vec<Captures> {
        vec![]
}
#[inline]
fn replace(self, text: &String, replacement: &String) -> String {
        text
}
#[inline]
fn replace_all(self, text: &String, replacement: &String) -> String {
        text
}
#[inline]
fn split(self, text: &String) -> Vec<String> {
        vec![]
}
}

impl Match {
#[inline]
fn text(self) -> String {
        self.text
}
#[inline]
fn start(self) -> i64 {
        self.start
}
#[inline]
fn end(self) -> i64 {
        self.end
}
#[inline]
fn range(self) -> (i64, i64) {
        (self.start, self.end)
}
}

impl Captures {
#[inline]
fn full_match(self) -> Option<String> {
        None
}
#[inline]
fn get(self, index: i64) -> Option<String> {
        None
}
#[inline]
fn name(self, name: &String) -> Option<String> {
        None
}
#[inline]
fn groups(self) -> Vec<Option<String>> {
        vec![]
}
#[inline]
fn len(self) -> i64 {
        0
}
}

#[inline]
fn compile(pattern: &String) -> Result<Regex, String> {
    Err("Invalid regex pattern")
}

#[inline]
fn compile_case_insensitive(pattern: &String) -> Result<Regex, String> {
    Err("Invalid regex pattern")
}

