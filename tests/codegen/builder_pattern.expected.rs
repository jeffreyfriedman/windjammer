#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub host: String,
    pub port: i64,
    pub timeout: i64,
}

impl Config {
#[inline]
pub fn new() -> Config {
        Config { host: "localhost".to_string(), port: 8080, timeout: 30 }
}
#[inline]
pub fn host(self, host: String) -> Config {
        Config { host, port: self.port, timeout: self.timeout }
}
#[inline]
pub fn port(self, port: i64) -> Config {
        Config { host: self.host, port, timeout: self.timeout }
}
}



















