// JWT token management

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,          // user_id
    pub username: String,
    pub role: String,
    pub exp: i64,          // expiration timestamp
    pub iat: i64,          // issued at timestamp
}

pub fn generate_token(user_id: i32, username: String, role: String, secret: &str) -> Result<String> {
    let now = Utc::now();
    let expiration = now + Duration::hours(24);
    
    let claims = Claims {
        sub: user_id,
        username,
        role,
        exp: expiration.timestamp(),
        iat: now.timestamp(),
    };
    
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("Failed to generate token: {}", e))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| anyhow!("Invalid token: {}", e))?;
    
    Ok(token_data.claims)
}

pub fn extract_user_id_from_token(token: &str, secret: &str) -> Result<i32> {
    let claims = verify_token(token, secret)?;
    Ok(claims.sub)
}
