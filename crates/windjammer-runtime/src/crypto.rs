//! Cryptographic functions
//!
//! Windjammer's `std::crypto` module maps to these functions.

use sha1::{Digest as Sha1Digest, Sha1};
use sha2::{Digest as Sha2Digest, Sha256};

/// SHA-256 hash
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = <Sha256 as Sha2Digest>::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// SHA-256 hash of string (Windjammer `std::crypto::sha256(string)`).
pub fn sha256(data: impl AsRef<str>) -> String {
    sha256_string(data.as_ref())
}

/// SHA-256 hash of string
pub fn sha256_string(s: &str) -> String {
    sha256_bytes(s.as_bytes())
}

/// SHA-256 hex digest of string (Windjammer `std::crypto::sha256_hex`).
pub fn sha256_hex(data: impl AsRef<str>) -> String {
    sha256_string(data.as_ref())
}

/// SHA-1 digest bytes (Windjammer `std::crypto::sha1_bytes`, UUID v5).
pub fn sha1_bytes(data: &[u8]) -> Vec<u8> {
    <Sha1 as Sha1Digest>::digest(data).to_vec()
}

/// Hash password with bcrypt
pub fn hash_password(password: &str) -> Result<String, String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())
}

/// Verify a password against a stored hash. Accepts bcrypt (`$2a$`/`$2b$`/`$2y$`) and Argon2
/// PHC strings (`$argon2id$...`), so an application can migrate hashes on login.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    if hash.starts_with("$argon2") {
        return argon2_verify(password, hash);
    }
    bcrypt::verify(password, hash).map_err(|e| e.to_string())
}

/// Hash a password with Argon2id (OWASP defaults: 19 MiB memory, 2 iterations, 1 lane) and a random
/// salt. Returns a PHC string (`$argon2id$v=19$m=19456,t=2,p=1$...`).
pub fn hash_password_argon2id(password: &str) -> Result<String, String> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let salt = SaltString::generate(&mut OsRng);
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

/// True when `hash` is an Argon2id PHC string. After a successful `verify_password` on a hash for
/// which this is false, rehash with `hash_password_argon2id` to migrate.
pub fn password_hash_is_argon2id(hash: &str) -> bool {
    hash.starts_with("$argon2id$")
}

fn argon2_verify(password: &str, hash: &str) -> Result<bool, String> {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    let parsed = PasswordHash::new(hash).map_err(|e| e.to_string())?;
    match argon2::Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

/// Raw Argon2id tag with explicit parameters (`memory_kib`, `iterations`, `lanes`), optional
/// `secret` and associated data. Used for interoperability checks against the RFC 9106 vectors.
pub fn argon2id_raw(
    password: &[u8],
    salt: &[u8],
    secret: &[u8],
    associated_data: &[u8],
    iterations: u32,
    memory_kib: u32,
    lanes: u32,
    out_len: usize,
) -> Result<Vec<u8>, String> {
    use argon2::{Algorithm, Argon2, AssociatedData, ParamsBuilder, Version};
    let mut b = ParamsBuilder::new();
    b.m_cost(memory_kib).t_cost(iterations).p_cost(lanes).output_len(out_len);
    if !associated_data.is_empty() {
        b.data(AssociatedData::new(associated_data).map_err(|e| e.to_string())?);
    }
    let params = b.build().map_err(|e| e.to_string())?;
    let ctx = Argon2::new_with_secret(secret, Algorithm::Argon2id, Version::V0x13, params)
        .map_err(|e| e.to_string())?;
    let mut out = vec![0u8; out_len];
    ctx.hash_password_into(password, salt, &mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Base64 encode
pub fn base64_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.encode(data)
}

/// Base64 decode
pub fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = sha256("hello");
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex characters
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sha256_bytes() {
        let hash = sha256_bytes(b"hello");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_password_hashing() {
        let password = "secret123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// RFC 9106 section 5.3 (Argon2id test vector).
    #[test]
    fn argon2id_rfc9106_vector() {
        let tag = argon2id_raw(&[0x01; 32], &[0x02; 16], &[0x03; 8], &[0x04; 12], 3, 32, 4, 32).unwrap();
        assert_eq!(hex(&tag), "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659");
    }

    #[test]
    fn argon2id_phc_roundtrip_and_migration_surface() {
        let h = hash_password_argon2id("correct horse").unwrap();
        assert!(h.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{h}");
        assert!(password_hash_is_argon2id(&h));
        assert!(verify_password("correct horse", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
        assert_ne!(h, hash_password_argon2id("correct horse").unwrap(), "random salt");
        let legacy = bcrypt::hash("pw", 4).unwrap();
        assert!(!password_hash_is_argon2id(&legacy));
        assert!(verify_password("pw", &legacy).unwrap(), "verify_password still accepts bcrypt");
    }

    /// Values produced by the reference C implementation (argon2-cffi bindings), Argon2id, v=19.
    #[test]
    fn argon2id_matches_reference_implementation() {
        let phc = "$argon2id$v=19$m=19456,t=2,p=1$MDEyMzQ1Njc4OWFiY2RlZg$nUxirfVK2I/vOT6f2ly2wSgjwZ1oqwTLCmrcpDyjicA";
        assert!(verify_password("hunter2", phc).unwrap());
        assert!(!verify_password("hunter3", phc).unwrap());
        let tag = argon2id_raw(b"password", b"somesalt", &[], &[], 2, 64, 1, 32).unwrap();
        assert_eq!(hex(&tag), "16a1a498734609dd01456da406de9f3d9da93e6c86c300a12fc1465214ce4922");
    }

    #[test]
    fn test_base64() {
        let data = b"hello world";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
