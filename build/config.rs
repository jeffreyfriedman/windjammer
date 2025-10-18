use std::env::*;


#[derive(Debug, Clone)]
struct Config {
    pub host: String,
    pub port: i64,
    pub database_url: String,
    pub jwt_secret: String,
    pub log_level: String,
}

#[inline]
fn load() -> Config {
    Config { host: env.get_or("HOST", "0.0.0.0"), port: env.get_or_int("PORT", 3000), database_url: env.get_or("DATABASE_URL", "postgres://localhost/taskflow"), jwt_secret: env.get_or("JWT_SECRET", "dev-secret-key"), log_level: env.get_or("LOG_LEVEL", "info") }
}

#[inline]
fn parse_int_or_default(val_opt: &Option<String>, default: i64) -> i64 {
    match val_opt {
        Some(val) => default,
        None => default,
    }
}

