//! OAuth 2.0 Authentication for MCP HTTP Transport
//!
//! Implements OAuth 2.0 authentication flows for the MCP server's HTTP transport.
//! Supports multiple grant types and token validation.
//!
//! Supported flows:
//! - Client Credentials Grant (for service-to-service)
//! - Authorization Code Grant (for user authentication)
//! - Refresh Token Grant
//!
//! Reference: https://datatracker.ietf.org/doc/html/rfc6749

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{McpError, McpResult};

/// OAuth 2.0 token claims (JWT payload)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (client ID)
    pub sub: String,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Scopes granted
    pub scopes: Vec<String>,
    /// Token type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
}

/// OAuth 2.0 access token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// Access token (JWT)
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Expires in seconds
    pub expires_in: i64,
    /// Refresh token (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Granted scopes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// OAuth 2.0 client credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCredentials {
    /// Client ID
    pub client_id: String,
    /// Client secret (hashed)
    pub client_secret_hash: String,
    /// Allowed scopes
    pub allowed_scopes: Vec<String>,
    /// Client name
    pub name: String,
    /// Created at
    pub created_at: DateTime<Utc>,
}

impl ClientCredentials {
    /// Create new client credentials
    pub fn new(client_id: String, client_secret: &str, name: String, scopes: Vec<String>) -> Self {
        Self {
            client_id,
            client_secret_hash: hash_secret(client_secret),
            allowed_scopes: scopes,
            name,
            created_at: Utc::now(),
        }
    }

    /// Verify client secret
    pub fn verify_secret(&self, secret: &str) -> bool {
        self.client_secret_hash == hash_secret(secret)
    }
}

/// Refresh token data
#[derive(Debug, Clone)]
struct RefreshToken {
    token: String,
    client_id: String,
    scopes: Vec<String>,
    expires_at: DateTime<Utc>,
}

/// OAuth 2.0 token manager
pub struct OAuthManager {
    /// JWT encoding key (for signing tokens)
    encoding_key: EncodingKey,
    /// JWT decoding key (for verifying tokens)
    decoding_key: DecodingKey,
    /// Registered clients
    clients: Arc<RwLock<HashMap<String, ClientCredentials>>>,
    /// Active refresh tokens
    refresh_tokens: Arc<RwLock<HashMap<String, RefreshToken>>>,
    /// Issuer name
    issuer: String,
    /// Audience
    audience: String,
    /// Access token TTL (seconds)
    access_token_ttl: i64,
    /// Refresh token TTL (seconds)
    refresh_token_ttl: i64,
}

impl OAuthManager {
    /// Create a new OAuth manager with a secret key
    pub fn new(secret_key: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret_key.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret_key.as_bytes()),
            clients: Arc::new(RwLock::new(HashMap::new())),
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
            issuer,
            audience,
            access_token_ttl: 3600,       // 1 hour
            refresh_token_ttl: 86400 * 7, // 7 days
        }
    }

    /// Register a new OAuth client
    pub async fn register_client(&self, credentials: ClientCredentials) -> McpResult<()> {
        let mut clients = self.clients.write().await;
        clients.insert(credentials.client_id.clone(), credentials);
        Ok(())
    }

    /// Client Credentials Grant
    /// Used for service-to-service authentication
    pub async fn client_credentials_grant(
        &self,
        client_id: &str,
        client_secret: &str,
        requested_scopes: Vec<String>,
    ) -> McpResult<TokenResponse> {
        // Verify client credentials
        let clients = self.clients.read().await;
        let client = clients
            .get(client_id)
            .ok_or_else(|| McpError::AuthenticationError {
                message: "Invalid client credentials".to_string(),
            })?;

        if !client.verify_secret(client_secret) {
            return Err(McpError::AuthenticationError {
                message: "Invalid client credentials".to_string(),
            });
        }

        // Validate requested scopes
        let granted_scopes: Vec<String> = requested_scopes
            .into_iter()
            .filter(|scope| client.allowed_scopes.contains(scope))
            .collect();

        if granted_scopes.is_empty() {
            return Err(McpError::AuthenticationError {
                message: "No valid scopes requested".to_string(),
            });
        }

        drop(clients);

        // Generate access token
        let access_token = self.generate_access_token(client_id, &granted_scopes)?;

        // Generate refresh token
        let refresh_token = self
            .generate_refresh_token(client_id, &granted_scopes)
            .await?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_ttl,
            refresh_token: Some(refresh_token),
            scope: Some(granted_scopes.join(" ")),
        })
    }

    /// Refresh Token Grant
    /// Exchange a refresh token for a new access token
    pub async fn refresh_token_grant(&self, refresh_token: &str) -> McpResult<TokenResponse> {
        let mut tokens = self.refresh_tokens.write().await;

        let token_data = tokens
            .get(refresh_token)
            .ok_or_else(|| McpError::AuthenticationError {
                message: "Invalid refresh token".to_string(),
            })?
            .clone();

        // Check if refresh token is expired
        if Utc::now() > token_data.expires_at {
            tokens.remove(refresh_token);
            return Err(McpError::AuthenticationError {
                message: "Refresh token expired".to_string(),
            });
        }

        drop(tokens);

        // Generate new access token
        let access_token = self.generate_access_token(&token_data.client_id, &token_data.scopes)?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_ttl,
            refresh_token: None, // Don't issue new refresh token on refresh
            scope: Some(token_data.scopes.join(" ")),
        })
    }

    /// Validate an access token (JWT)
    pub fn validate_token(&self, token: &str) -> McpResult<TokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        let token_data =
            decode::<TokenClaims>(token, &self.decoding_key, &validation).map_err(|e| {
                McpError::AuthenticationError {
                    message: format!("Invalid token: {}", e),
                }
            })?;

        Ok(token_data.claims)
    }

    /// Revoke a refresh token
    pub async fn revoke_refresh_token(&self, refresh_token: &str) -> McpResult<()> {
        let mut tokens = self.refresh_tokens.write().await;
        tokens.remove(refresh_token);
        Ok(())
    }

    /// Clean up expired refresh tokens
    pub async fn cleanup_expired_tokens(&self) {
        let mut tokens = self.refresh_tokens.write().await;
        let now = Utc::now();
        tokens.retain(|_, token| token.expires_at > now);
    }

    /// Generate access token (JWT)
    fn generate_access_token(&self, client_id: &str, scopes: &[String]) -> McpResult<String> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(self.access_token_ttl);

        let claims = TokenClaims {
            sub: client_id.to_string(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            scopes: scopes.to_vec(),
            token_type: Some("access".to_string()),
        };

        let header = Header::new(Algorithm::HS256);
        encode(&header, &claims, &self.encoding_key).map_err(|e| McpError::InternalError {
            message: format!("Failed to generate token: {}", e),
        })
    }

    /// Generate refresh token
    async fn generate_refresh_token(
        &self,
        client_id: &str,
        scopes: &[String],
    ) -> McpResult<String> {
        // Generate random token
        let mut rng = rand::thread_rng();
        let token_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        let token = BASE64.encode(&token_bytes);

        let expires_at = Utc::now() + Duration::seconds(self.refresh_token_ttl);

        let refresh_token_data = RefreshToken {
            token: token.clone(),
            client_id: client_id.to_string(),
            scopes: scopes.to_vec(),
            expires_at,
        };

        let mut tokens = self.refresh_tokens.write().await;
        tokens.insert(token.clone(), refresh_token_data);

        Ok(token)
    }

    /// Get client by ID (for testing)
    #[cfg(test)]
    pub async fn get_client(&self, client_id: &str) -> Option<ClientCredentials> {
        let clients = self.clients.read().await;
        clients.get(client_id).cloned()
    }

    /// Get refresh token count (for testing)
    #[cfg(test)]
    pub async fn refresh_token_count(&self) -> usize {
        self.refresh_tokens.read().await.len()
    }
}

/// Hash a secret using SHA-256
fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    BASE64.encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_registration() {
        let manager = OAuthManager::new(
            "test-secret-key",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let credentials = ClientCredentials::new(
            "test-client".to_string(),
            "test-secret",
            "Test Client".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        manager.register_client(credentials.clone()).await.unwrap();

        let stored_client = manager.get_client("test-client").await.unwrap();
        assert_eq!(stored_client.client_id, "test-client");
        assert!(stored_client.verify_secret("test-secret"));
        assert!(!stored_client.verify_secret("wrong-secret"));
    }

    #[tokio::test]
    async fn test_client_credentials_grant() {
        let manager = OAuthManager::new(
            "test-secret-key",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let credentials = ClientCredentials::new(
            "test-client".to_string(),
            "test-secret",
            "Test Client".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        manager.register_client(credentials).await.unwrap();

        // Valid credentials
        let response = manager
            .client_credentials_grant("test-client", "test-secret", vec!["read".to_string()])
            .await
            .unwrap();

        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
        assert!(response.refresh_token.is_some());
        assert_eq!(response.scope, Some("read".to_string()));

        // Validate the access token
        let claims = manager.validate_token(&response.access_token).unwrap();
        assert_eq!(claims.sub, "test-client");
        assert_eq!(claims.scopes, vec!["read"]);
    }

    #[tokio::test]
    async fn test_invalid_credentials() {
        let manager = OAuthManager::new(
            "test-secret-key",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let credentials = ClientCredentials::new(
            "test-client".to_string(),
            "test-secret",
            "Test Client".to_string(),
            vec!["read".to_string()],
        );

        manager.register_client(credentials).await.unwrap();

        // Invalid client ID
        let result = manager
            .client_credentials_grant("wrong-client", "test-secret", vec!["read".to_string()])
            .await;
        assert!(result.is_err());

        // Invalid client secret
        let result = manager
            .client_credentials_grant("test-client", "wrong-secret", vec!["read".to_string()])
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refresh_token_grant() {
        let manager = OAuthManager::new(
            "test-secret-key",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let credentials = ClientCredentials::new(
            "test-client".to_string(),
            "test-secret",
            "Test Client".to_string(),
            vec!["read".to_string()],
        );

        manager.register_client(credentials).await.unwrap();

        // Get initial tokens
        let initial_response = manager
            .client_credentials_grant("test-client", "test-secret", vec!["read".to_string()])
            .await
            .unwrap();

        let refresh_token = initial_response.refresh_token.unwrap();

        // Use refresh token to get new access token
        let refreshed_response = manager.refresh_token_grant(&refresh_token).await.unwrap();

        assert_eq!(refreshed_response.token_type, "Bearer");
        assert!(refreshed_response.refresh_token.is_none()); // No new refresh token

        // Validate new access token
        let claims = manager
            .validate_token(&refreshed_response.access_token)
            .unwrap();
        assert_eq!(claims.sub, "test-client");
        assert_eq!(claims.scopes, vec!["read"]);
    }

    #[tokio::test]
    async fn test_token_revocation() {
        let manager = OAuthManager::new(
            "test-secret-key",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let credentials = ClientCredentials::new(
            "test-client".to_string(),
            "test-secret",
            "Test Client".to_string(),
            vec!["read".to_string()],
        );

        manager.register_client(credentials).await.unwrap();

        let response = manager
            .client_credentials_grant("test-client", "test-secret", vec!["read".to_string()])
            .await
            .unwrap();

        let refresh_token = response.refresh_token.unwrap();

        // Revoke refresh token
        manager.revoke_refresh_token(&refresh_token).await.unwrap();

        // Try to use revoked token
        let result = manager.refresh_token_grant(&refresh_token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scope_filtering() {
        let manager = OAuthManager::new(
            "test-secret-key",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        let credentials = ClientCredentials::new(
            "test-client".to_string(),
            "test-secret",
            "Test Client".to_string(),
            vec!["read".to_string()], // Only "read" allowed
        );

        manager.register_client(credentials).await.unwrap();

        // Request both "read" and "write", should only get "read"
        let response = manager
            .client_credentials_grant(
                "test-client",
                "test-secret",
                vec!["read".to_string(), "write".to_string()],
            )
            .await
            .unwrap();

        assert_eq!(response.scope, Some("read".to_string()));

        let claims = manager.validate_token(&response.access_token).unwrap();
        assert_eq!(claims.scopes, vec!["read"]);
        assert!(!claims.scopes.contains(&"write".to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens() {
        let mut manager = OAuthManager::new(
            "test-secret-key",
            "test-issuer".to_string(),
            "test-audience".to_string(),
        );

        // Set very short TTL for testing
        manager.refresh_token_ttl = 1; // 1 second

        let credentials = ClientCredentials::new(
            "test-client".to_string(),
            "test-secret",
            "Test Client".to_string(),
            vec!["read".to_string()],
        );

        manager.register_client(credentials).await.unwrap();

        let response = manager
            .client_credentials_grant("test-client", "test-secret", vec!["read".to_string()])
            .await
            .unwrap();

        assert_eq!(manager.refresh_token_count().await, 1);

        // Wait for token to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Clean up expired tokens
        manager.cleanup_expired_tokens().await;

        assert_eq!(manager.refresh_token_count().await, 0);
    }
}
