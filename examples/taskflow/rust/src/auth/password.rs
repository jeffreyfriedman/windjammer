// Password hashing utilities using bcrypt

use anyhow::{anyhow, Result};
use bcrypt::{hash, verify, DEFAULT_COST};

pub fn hash_password(password: &str) -> Result<String> {
    hash(password, DEFAULT_COST).map_err(|e| anyhow!("Password hashing failed: {}", e))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    verify(password, hash).map_err(|e| anyhow!("Password verification failed: {}", e))
}
