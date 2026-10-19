#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Config {
    host: String,
    port: i64,
    debug: bool,
}

impl Config {
#[inline]
fn default() -> Config {
        Config { host: String::from("localhost"), port: 8080_i64, debug: false }
}
#[inline]
fn with_host(self, h: String) -> Config {
        Config { host: h, port: self.port, debug: self.debug }
}
#[inline]
fn with_port(self, p: i64) -> Config {
        Config { host: self.host, port: p, debug: self.debug }
}
#[inline]
fn with_debug(self, d: bool) -> Config {
        Config { host: self.host, port: self.port, debug: d }
}
#[inline]
fn show(&self) -> String {
        format!("{}:{} debug={}", self.host, self.port, self.debug)
}
}

fn main() {
    println!("{}", Config::default().show());
    let c = Config::default().with_host("example.org".to_string()).with_port(443_i64).with_debug(true);
    println!("{}", c.show());
}

