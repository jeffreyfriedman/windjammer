# OAuth 2.0 Authentication for Windjammer MCP Server

## Overview

The Windjammer MCP server supports OAuth 2.0 authentication for the HTTP transport, providing enterprise-grade security for AI assistant integrations.

**Specification**: [RFC 6749 - OAuth 2.0 Authorization Framework](https://datatracker.ietf.org/doc/html/rfc6749)

---

## Features

- ✅ **Client Credentials Grant** - Service-to-service authentication
- ✅ **Refresh Token Grant** - Long-lived sessions without re-authentication
- ✅ **JWT Access Tokens** - Stateless token validation
- ✅ **Scope-based Authorization** - Fine-grained permission control
- ✅ **Token Revocation** - Security-first design
- ✅ **Automatic Cleanup** - Expired token management

---

## Quick Start

### 1. Enable OAuth

Configure your MCP HTTP server with OAuth enabled:

```rust
use windjammer_mcp::http_server::HttpServerConfig;

let config = HttpServerConfig {
    host: "127.0.0.1".to_string(),
    port: 3000,
    session_ttl_seconds: 3600,
    enable_oauth: true,
    oauth_secret_key: Some("your-secret-key-here".to_string()),
    oauth_issuer: Some("windjammer-mcp-server".to_string()),
    oauth_audience: Some("mcp-clients".to_string()),
};
```

⚠️ **Security Note**: Use a strong random secret key in production!

### 2. Register OAuth Clients

```rust
use windjammer_mcp::oauth::ClientCredentials;

let server = McpHttpServer::new(config, mcp_server)?;
let oauth = server.oauth_manager().unwrap();

// Register a client application
let client = ClientCredentials::new(
    "my-ai-app".to_string(),
    "client-secret-here",
    "My AI Application".to_string(),
    vec!["read".to_string(), "write".to_string(), "refactor".to_string()],
);

oauth.register_client(client).await?;
```

### 3. Obtain Access Token (Client Credentials Grant)

**HTTP Request:**
```http
POST /oauth/token HTTP/1.1
Host: localhost:3000
Content-Type: application/json

{
  "grant_type": "client_credentials",
  "client_id": "my-ai-app",
  "client_secret": "client-secret-here",
  "scope": "read write"
}
```

**Example with `curl`:**
```bash
curl -X POST http://localhost:3000/oauth/token \
  -H "Content-Type: application/json" \
  -d '{
    "grant_type": "client_credentials",
    "client_id": "my-ai-app",
    "client_secret": "client-secret-here",
    "scope": "read write"
  }'
```

**Response:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "abc123...",
  "scope": "read write"
}
```

### 4. Make Authenticated Requests

**HTTP Request:**
```http
POST /mcp HTTP/1.1
Host: localhost:3000
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "parse_code",
    "arguments": {
      "code": "fn main() { println!(\"Hello\"); }"
    }
  }
}
```

**Example with `curl`:**
```bash
ACCESS_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."

curl -X POST http://localhost:3000/mcp \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "parse_code",
      "arguments": {
        "code": "fn main() { println!(\"Hello\"); }"
      }
    }
  }'
```

---

## Grant Types

### Client Credentials Grant

**Use Case**: Service-to-service authentication where no user interaction is required.

**Flow:**
1. Client sends `client_id` + `client_secret` + `scope` to `/oauth/token`
2. Server validates credentials
3. Server returns `access_token` + `refresh_token`
4. Client includes `access_token` in `Authorization: Bearer` header

**Example (Rust):**
```rust
let token_response = oauth_manager
    .client_credentials_grant(
        "my-ai-app",
        "client-secret-here",
        vec!["read".to_string(), "write".to_string()],
    )
    .await?;

println!("Access Token: {}", token_response.access_token);
println!("Expires in: {} seconds", token_response.expires_in);
```

### Refresh Token Grant

**Use Case**: Obtain a new access token without re-entering credentials.

**HTTP Request:**
```http
POST /oauth/token HTTP/1.1
Host: localhost:3000
Content-Type: application/json

{
  "grant_type": "refresh_token",
  "refresh_token": "abc123..."
}
```

**Example (Rust):**
```rust
let new_token = oauth_manager
    .refresh_token_grant("abc123...")
    .await?;

println!("New Access Token: {}", new_token.access_token);
```

---

## Scopes

Scopes control which MCP tools a client can access. You define scopes when registering clients.

**Recommended Scopes:**

| Scope | Description | Example Tools |
|-------|-------------|---------------|
| `read` | Read-only operations | `parse_code`, `analyze_types`, `get_definition` |
| `write` | Code generation | `generate_code`, `suggest_fix` |
| `refactor` | Code refactoring | `extract_function`, `inline_variable`, `rename_symbol` |
| `search` | Workspace search | `search_workspace`, `list_symbols` |
| `admin` | Administrative operations | Client management, token revocation |

**Example - Read-only Client:**
```rust
let readonly_client = ClientCredentials::new(
    "analyzer-bot".to_string(),
    "secret",
    "Code Analyzer Bot".to_string(),
    vec!["read".to_string()], // Only read access
);
```

**Example - Full Access Client:**
```rust
let admin_client = ClientCredentials::new(
    "admin-tool".to_string(),
    "secret",
    "Admin Tool".to_string(),
    vec!["read".to_string(), "write".to_string(), "refactor".to_string(), "admin".to_string()],
);
```

---

## Token Lifecycle

### Access Tokens (JWT)

- **Format**: JSON Web Token (JWT) signed with HS256
- **Default TTL**: 1 hour (3600 seconds)
- **Claims**:
  - `sub`: Client ID
  - `iat`: Issued at (Unix timestamp)
  - `exp`: Expiration time (Unix timestamp)
  - `iss`: Issuer (`oauth_issuer` from config)
  - `aud`: Audience (`oauth_audience` from config)
  - `scopes`: Array of granted scopes

**Inspect Token (JWT Debugger):**
```bash
# Decode JWT (header.payload.signature)
echo "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." | cut -d'.' -f2 | base64 -d | jq
```

### Refresh Tokens

- **Format**: Base64-encoded random bytes (32 bytes)
- **Default TTL**: 7 days (604,800 seconds)
- **Storage**: In-memory (cleared on server restart)
- **Single-use**: No (can be reused until expiration)

### Token Revocation

Revoke a refresh token to invalidate all access derived from it:

```rust
oauth_manager.revoke_refresh_token("abc123...").await?;
```

**Use Cases:**
- User logout
- Security incident response
- Client deregistration

### Automatic Cleanup

The OAuth manager automatically removes expired refresh tokens:

```rust
// Triggered periodically (recommended: every hour)
oauth_manager.cleanup_expired_tokens().await;
```

---

## Security Best Practices

### 1. Secret Key Management

❌ **Don't**:
```rust
oauth_secret_key: Some("secret123".to_string()), // Weak!
```

✅ **Do**:
```rust
use std::env;

oauth_secret_key: Some(
    env::var("MCP_OAUTH_SECRET")
        .expect("MCP_OAUTH_SECRET environment variable required")
),
```

**Generate Strong Keys:**
```bash
# Generate 256-bit random key
openssl rand -base64 32
```

### 2. HTTPS in Production

OAuth 2.0 requires HTTPS to prevent token interception.

```rust
let config = HttpServerConfig {
    host: "0.0.0.0".to_string(),
    port: 443, // HTTPS port
    // ... OAuth config ...
};

// Use reverse proxy (nginx, Caddy) to handle TLS
```

### 3. Scope Principle of Least Privilege

Only grant scopes that clients actually need:

```rust
// ❌ Don't give everything
vec!["read", "write", "refactor", "admin"]

// ✅ Give minimum required
vec!["read"] // For read-only bot
```

### 4. Client Secret Storage

Never commit client secrets to version control:

```bash
# .env file (gitignored)
CLIENT_ID=my-ai-app
CLIENT_SECRET=<strong-random-secret>
```

```rust
use dotenvy::dotenv;

dotenv().ok();
let client_id = env::var("CLIENT_ID")?;
let client_secret = env::var("CLIENT_SECRET")?;
```

### 5. Token Rotation

Regularly rotate access tokens using refresh tokens:

```rust
async fn get_valid_token(
    oauth: &OAuthManager,
    current_token: &str,
    refresh_token: &str,
) -> McpResult<String> {
    // Validate current token
    match oauth.validate_token(current_token) {
        Ok(claims) => {
            // Check if expiring soon (< 5 minutes)
            let now = Utc::now().timestamp();
            if claims.exp - now < 300 {
                // Refresh proactively
                let new_token = oauth.refresh_token_grant(refresh_token).await?;
                Ok(new_token.access_token)
            } else {
                Ok(current_token.to_string())
            }
        }
        Err(_) => {
            // Token invalid/expired, refresh
            let new_token = oauth.refresh_token_grant(refresh_token).await?;
            Ok(new_token.access_token)
        }
    }
}
```

---

## Integration Examples

### Python Client

```python
import requests

# Obtain access token
token_response = requests.post(
    "http://localhost:3000/oauth/token",
    json={
        "grant_type": "client_credentials",
        "client_id": "my-ai-app",
        "client_secret": "client-secret-here",
        "scope": "read write"
    }
)
access_token = token_response.json()["access_token"]

# Make authenticated request
response = requests.post(
    "http://localhost:3000/mcp",
    headers={"Authorization": f"Bearer {access_token}"},
    json={
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "parse_code",
            "arguments": {"code": "fn main() {}"}
        }
    }
)

print(response.json())
```

### Node.js/TypeScript Client

```typescript
import axios from 'axios';

// Obtain access token
const tokenResponse = await axios.post('http://localhost:3000/oauth/token', {
  grant_type: 'client_credentials',
  client_id: 'my-ai-app',
  client_secret: 'client-secret-here',
  scope: 'read write'
});

const accessToken = tokenResponse.data.access_token;

// Make authenticated request
const response = await axios.post(
  'http://localhost:3000/mcp',
  {
    jsonrpc: '2.0',
    id: 1,
    method: 'tools/call',
    params: {
      name: 'parse_code',
      arguments: { code: 'fn main() {}' }
    }
  },
  {
    headers: { Authorization: `Bearer ${accessToken}` }
  }
);

console.log(response.data);
```

---

## Troubleshooting

### "Invalid client credentials"

**Cause**: Client ID or secret is wrong.

**Solution**:
- Verify client is registered: `oauth.get_client("my-ai-app").await`
- Check secret hash matches: `client.verify_secret("your-secret")`

### "Missing Authorization header"

**Cause**: OAuth is enabled but request has no `Authorization` header.

**Solution**: Include `Authorization: Bearer <token>` in all requests.

### "Invalid token"

**Possible Causes**:
1. Token expired (default: 1 hour)
2. Wrong issuer/audience
3. Token signature invalid

**Solutions**:
- Check token expiration: Decode JWT and inspect `exp` claim
- Verify `oauth_issuer` and `oauth_audience` match
- Use refresh token to get new access token

### "No valid scopes requested"

**Cause**: Client requested scopes it doesn't have permission for.

**Solution**: Only request scopes granted to the client during registration.

---

## API Reference

### `OAuthManager::new()`

```rust
pub fn new(secret_key: &str, issuer: String, audience: String) -> Self
```

Create a new OAuth manager.

**Parameters:**
- `secret_key`: JWT signing secret (use strong random value!)
- `issuer`: Token issuer (e.g., "windjammer-mcp-server")
- `audience`: Token audience (e.g., "mcp-clients")

### `register_client()`

```rust
pub async fn register_client(&self, credentials: ClientCredentials) -> McpResult<()>
```

Register a new OAuth client.

### `client_credentials_grant()`

```rust
pub async fn client_credentials_grant(
    &self,
    client_id: &str,
    client_secret: &str,
    requested_scopes: Vec<String>,
) -> McpResult<TokenResponse>
```

Authenticate a client and issue tokens.

### `refresh_token_grant()`

```rust
pub async fn refresh_token_grant(&self, refresh_token: &str) -> McpResult<TokenResponse>
```

Exchange a refresh token for a new access token.

### `validate_token()`

```rust
pub fn validate_token(&self, token: &str) -> McpResult<TokenClaims>
```

Validate and decode an access token (JWT).

### `revoke_refresh_token()`

```rust
pub async fn revoke_refresh_token(&self, refresh_token: &str) -> McpResult<()>
```

Revoke a refresh token.

### `cleanup_expired_tokens()`

```rust
pub async fn cleanup_expired_tokens(&self)
```

Remove expired refresh tokens from storage.

---

## FAQ

**Q: Is OAuth required?**  
A: No. OAuth is optional and disabled by default. Set `enable_oauth: true` to enable it.

**Q: Can I use OAuth with stdio transport?**  
A: No. OAuth is only for HTTP transport. stdio uses process-level security.

**Q: What happens if the server restarts?**  
A: Refresh tokens are stored in memory and will be lost. Access tokens (JWT) remain valid until expiration.

**Q: Can I customize token TTLs?**  
A: Yes. `OAuthManager` has `access_token_ttl` and `refresh_token_ttl` fields (currently private, but can be exposed).

**Q: Does this support OAuth 2.1?**  
A: Partially. Implements core OAuth 2.0 (RFC 6749). OAuth 2.1 (PKCE, etc.) coming in future versions.

---

## Further Reading

- [RFC 6749 - OAuth 2.0 Authorization Framework](https://datatracker.ietf.org/doc/html/rfc6749)
- [RFC 6750 - OAuth 2.0 Bearer Token Usage](https://datatracker.ietf.org/doc/html/rfc6750)
- [RFC 7519 - JSON Web Token (JWT)](https://datatracker.ietf.org/doc/html/rfc7519)
- [OAuth 2.0 Security Best Practices](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-security-topics)

---

**Security Disclosure**: Report security issues to security@windjammer.dev (or GitHub Security Advisories)

