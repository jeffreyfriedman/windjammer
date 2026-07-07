//! JWT HS256 verify/sign — Windjammer `std::jwt` contract.

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub tenant_slug: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    pub exp: i64,
}

pub fn verify_hs256(token: impl AsRef<str>, secret: impl AsRef<str>) -> Result<JwtClaims, String> {
    let token = token.as_ref().trim();
    let secret = secret.as_ref();
    if secret.is_empty() {
        return Err("JWT secret is empty".into());
    }
    if token.is_empty() {
        return Err("JWT token is empty".into());
    }

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;

    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| e.to_string())
}

pub fn sign_hs256(
    sub: impl AsRef<str>,
    tenant_slug: impl AsRef<str>,
    secret: impl AsRef<str>,
    ttl_secs: i64,
) -> Result<String, String> {
    let secret = secret.as_ref();
    if secret.is_empty() {
        return Err("JWT secret is empty".into());
    }
    if ttl_secs <= 0 {
        return Err("JWT ttl must be positive".into());
    }

    let now = chrono::Utc::now().timestamp();
    let claims = JwtClaims {
        sub: sub.as_ref().to_string(),
        tenant_slug: tenant_slug.as_ref().to_string(),
        tenant_id: None,
        entity_id: None,
        email: None,
        exp: now + ttl_secs,
    };

    encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

/// Mint HS256 JWT with full tenant scope claims (Phase 0 platform auth).
pub fn sign_hs256_scoped(
    sub: impl AsRef<str>,
    tenant_slug: impl AsRef<str>,
    tenant_id: impl AsRef<str>,
    entity_id: impl AsRef<str>,
    email: impl AsRef<str>,
    secret: impl AsRef<str>,
    ttl_secs: i64,
) -> Result<String, String> {
    let secret = secret.as_ref();
    if secret.is_empty() {
        return Err("JWT secret is empty".into());
    }
    if ttl_secs <= 0 {
        return Err("JWT ttl must be positive".into());
    }

    let now = chrono::Utc::now().timestamp();
    let claims = JwtClaims {
        sub: sub.as_ref().to_string(),
        tenant_slug: tenant_slug.as_ref().to_string(),
        tenant_id: Some(tenant_id.as_ref().to_string()),
        entity_id: Some(entity_id.as_ref().to_string()),
        email: Some(email.as_ref().to_string()),
        exp: now + ttl_secs,
    };

    encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify_round_trip() {
        let secret = "test-secret";
        let token = sign_hs256("user-1", "demo", secret, 3600).expect("sign");
        let claims = verify_hs256(&token, secret).expect("verify");
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.tenant_slug, "demo");
        assert!(claims.exp > chrono::Utc::now().timestamp());
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let token = sign_hs256("user-1", "demo", "secret-a", 3600).expect("sign");
        assert!(verify_hs256(&token, "secret-b").is_err());
    }

    #[test]
    fn verify_rejects_tampered_token() {
        let token = sign_hs256("user-1", "demo", "secret", 3600).expect("sign");
        assert!(verify_hs256(&format!("{token}x"), "secret").is_err());
    }

    #[test]
    fn sign_verify_scoped_claims_round_trip() {
        let secret = "test-secret";
        let token = sign_hs256_scoped(
            "user-1",
            "demo",
            "550e8400-e29b-41d4-a716-446655440000",
            "660e8400-e29b-41d4-a716-446655440001",
            "owner@demo.local",
            secret,
            3600,
        )
        .expect("sign");
        let claims = verify_hs256(&token, secret).expect("verify");
        assert_eq!(
            claims.tenant_id.as_deref(),
            Some("550e8400-e29b-41d4-a716-446655440000")
        );
        assert_eq!(
            claims.entity_id.as_deref(),
            Some("660e8400-e29b-41d4-a716-446655440001")
        );
        assert_eq!(claims.email.as_deref(), Some("owner@demo.local"));
    }

    #[test]
    fn legacy_token_without_tenant_id_still_verifies() {
        let secret = "test-secret";
        let token = sign_hs256("user-1", "demo", secret, 3600).expect("sign");
        let claims = verify_hs256(&token, secret).expect("verify");
        assert!(claims.tenant_id.is_none());
    }
}
