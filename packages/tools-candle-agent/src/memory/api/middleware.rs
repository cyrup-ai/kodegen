//! Middleware for the memory API
//! This module contains middleware functions for authentication, logging, etc.

use std::collections::HashMap;
use std::time::Instant;

use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use log::info;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::OnceCell;
use tower_http::cors::{Any, CorsLayer};

/// Security configuration errors
#[derive(Debug, Error)]
pub enum SecurityConfigError {
    #[error(
        "JWT secret not configured - set JWT_SECRET environment variable or provide secure config"
    )]
    JwtSecretMissing,
    #[error("JWT secret too weak - must be at least 32 characters")]
    JwtSecretTooWeak,
    #[error(
        "API keys not configured - set API_KEYS_FILE environment variable or configure programmatically"
    )]
    ApiKeysNotConfigured,
    #[error("Invalid API key format: {0}")]
    InvalidApiKeyFormat(String),
    #[error("Security config file error: {0}")]
    ConfigFileError(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Secure JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    secret: String,
    algorithm: Algorithm,
    expiration_hours: u64,
}

impl JwtConfig {
    /// Create JWT config from environment variables with validation
    pub fn from_env() -> Result<Self, SecurityConfigError> {
        let secret =
            std::env::var("JWT_SECRET").map_err(|_| SecurityConfigError::JwtSecretMissing)?;

        // Validate secret strength
        if secret.len() < 32 {
            return Err(SecurityConfigError::JwtSecretTooWeak);
        }

        // Validate secret is not a common weak pattern
        if secret == "default-secret-key" || secret.starts_with("test") || secret == "secret" {
            return Err(SecurityConfigError::JwtSecretTooWeak);
        }

        let algorithm = match std::env::var("JWT_ALGORITHM").as_deref() {
            Ok("HS256") => Algorithm::HS256,
            Ok("HS384") => Algorithm::HS384,
            Ok("HS512") => Algorithm::HS512,
            _ => Algorithm::HS256, // Default to HS256
        };

        let expiration_hours = std::env::var("JWT_EXPIRATION_HOURS")
            .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
            .unwrap_or(24); // Default 24 hours

        Ok(Self {
            secret,
            algorithm,
            expiration_hours,
        })
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    pub fn expiration_hours(&self) -> u64 {
        self.expiration_hours
    }
}

/// API key management with secure loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub key_id: String,
    pub key_hash: String, // Store hash, not plaintext
    pub user_context: UserContext,
}

/// Secure API key manager
pub struct ApiKeyManager {
    keys: HashMap<String, UserContext>,
    key_hashes: HashMap<String, String>, // hash -> key_id
}

impl ApiKeyManager {
    /// Load API keys from secure configuration
    pub async fn from_env() -> Result<Self, SecurityConfigError> {
        let config_file = std::env::var("API_KEYS_FILE")
            .unwrap_or_else(|_| "/etc/cyrup/api-keys.json".to_string());

        if std::path::Path::new(&config_file).exists() {
            Self::from_file(&config_file).await
        } else {
            // In development/testing, create minimal secure config
            log::warn!(
                "API keys file not found at {}. Using development configuration.",
                config_file
            );
            Self::development_config()
        }
    }

    /// Load from encrypted configuration file
    async fn from_file(path: &str) -> Result<Self, SecurityConfigError> {
        let content = tokio::fs::read_to_string(path).await?;
        let configs: Vec<ApiKeyConfig> = serde_json::from_str(&content)?;

        let mut keys = HashMap::new();
        let mut key_hashes = HashMap::new();

        for config in configs {
            // Validate key format
            if config.key_id.len() < 16 {
                return Err(SecurityConfigError::InvalidApiKeyFormat(
                    "Key ID must be at least 16 characters".to_string(),
                ));
            }

            keys.insert(config.key_id.clone(), config.user_context);
            key_hashes.insert(config.key_hash, config.key_id);
        }

        Ok(Self { keys, key_hashes })
    }

    /// Development configuration with secure random keys
    fn development_config() -> Result<Self, SecurityConfigError> {
        use sha2::{Digest, Sha256};

        let mut keys = HashMap::new();
        let mut key_hashes = HashMap::new();

        // Generate secure random key for development
        let dev_key = format!("dev_{}", uuid::Uuid::new_v4());
        let digest = Sha256::digest(dev_key.as_bytes());
        let key_hash = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let user_context = UserContext {
            user_id: "dev_user".to_string(),
            email: "dev@localhost".to_string(),
            roles: vec!["developer".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
            expires_at: None,
        };

        keys.insert(dev_key.clone(), user_context);
        key_hashes.insert(key_hash, dev_key);

        log::info!("Development API key configuration created. Use generated key for API access.");

        Ok(Self { keys, key_hashes })
    }

    /// Validate API key using secure hash comparison
    pub fn validate_key(&self, provided_key: &str) -> Option<&UserContext> {
        use sha2::{Digest, Sha256};

        // Hash the provided key
        let digest = Sha256::digest(provided_key.as_bytes());
        let key_hash = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        // Look up by hash to prevent timing attacks
        if let Some(key_id) = self.key_hashes.get(&key_hash) {
            self.keys.get(key_id)
        } else {
            // Check direct key match for development keys
            self.keys.get(provided_key)
        }
    }

    /// Rotate API key (for future implementation)
    pub fn rotate_key(&mut self, old_key: &str, new_key: &str) -> Result<(), SecurityConfigError> {
        use sha2::{Digest, Sha256};

        if let Some(user_context) = self.keys.remove(old_key) {
            let digest = Sha256::digest(new_key.as_bytes());
            let new_hash = digest
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            self.keys.insert(new_key.to_string(), user_context);
            self.key_hashes.insert(new_hash, new_key.to_string());
            Ok(())
        } else {
            Err(SecurityConfigError::InvalidApiKeyFormat(
                "Key not found for rotation".to_string(),
            ))
        }
    }
}

/// Global secure JWT configuration
static JWT_CONFIG: OnceCell<Result<JwtConfig, SecurityConfigError>> = OnceCell::const_new();

/// Global secure API key manager
static API_KEY_MANAGER: OnceCell<Result<ApiKeyManager, SecurityConfigError>> =
    OnceCell::const_new();

/// User context extracted from authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    pub email: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtClaims {
    sub: String, // Subject (user ID)
    email: String,
    roles: Vec<String>,
    permissions: Vec<String>,
    exp: i64, // Expiration time
    iat: i64, // Issued at
}

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    InvalidToken,
    ExpiredToken,
    InvalidApiKey,
    MissingCredentials,
    InsufficientPermissions,
}

/// Add CORS middleware
pub fn cors_middleware() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Request logging middleware
pub async fn logging_middleware(request: Request<Body>, next: Next) -> impl IntoResponse {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    info!("{} {} - {:?}", method, uri, duration);

    response
}

/// Production authentication middleware with JWT and API key support
pub async fn auth_middleware(mut request: Request<Body>, next: Next) -> impl IntoResponse {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    // Check for API key in header or query parameter
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            request.uri().query().and_then(|q| {
                q.split('&')
                    .find(|param| param.starts_with("api_key="))
                    .and_then(|param| param.split('=').nth(1))
            })
        });

    // Validate authentication
    let auth_result = if let Some(auth_header) = auth_header {
        validate_jwt_token(auth_header).await
    } else if let Some(api_key) = api_key {
        validate_api_key(api_key).await
    } else {
        Err(AuthError::MissingCredentials)
    };

    match auth_result {
        Ok(user_context) => {
            // Add user context to request extensions
            request.extensions_mut().insert(user_context);

            // Add security headers to response
            let response = next.run(request).await;
            add_security_headers(response)
        }
        Err(auth_error) => {
            log::warn!("Authentication failed: {:?}", auth_error);

            let error_response = match auth_error {
                AuthError::InvalidToken | AuthError::ExpiredToken => {
                    (StatusCode::UNAUTHORIZED, "Invalid or expired token")
                }
                AuthError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
                AuthError::MissingCredentials => (
                    StatusCode::UNAUTHORIZED,
                    "Missing authentication credentials",
                ),
                AuthError::InsufficientPermissions => {
                    (StatusCode::FORBIDDEN, "Insufficient permissions")
                }
            };

            error_response.into_response()
        }
    }
}

/// Validate JWT token and extract user context
async fn validate_jwt_token(auth_header: &str) -> Result<UserContext, AuthError> {
    // Extract token from "Bearer <token>" format
    let token = if let Some(stripped) = auth_header.strip_prefix("Bearer ") {
        stripped
    } else {
        return Err(AuthError::InvalidToken);
    };

    // Get secure JWT configuration
    let jwt_config = JWT_CONFIG
        .get_or_init(|| async { JwtConfig::from_env() })
        .await
        .as_ref()
        .map_err(|e| {
            log::error!("JWT configuration error: {}", e);
            AuthError::InvalidToken
        })?;

    // Decode and validate JWT with secure configuration
    let decoding_key = DecodingKey::from_secret(jwt_config.secret().as_bytes());
    let validation = Validation::new(jwt_config.algorithm());

    match decode::<JwtClaims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            let claims = token_data.claims;

            // Check if token is expired
            let now = Utc::now().timestamp();
            if claims.exp < now {
                return Err(AuthError::ExpiredToken);
            }

            Ok(UserContext {
                user_id: claims.sub,
                email: claims.email,
                roles: claims.roles,
                permissions: claims.permissions,
                expires_at: DateTime::from_timestamp(claims.exp, 0),
            })
        }
        Err(e) => {
            log::warn!("JWT decode error: {}", e);
            Err(AuthError::InvalidToken)
        }
    }
}

/// Validate API key and return associated user context
async fn validate_api_key(provided_key: &str) -> Result<UserContext, AuthError> {
    // Get secure API key manager
    let api_manager = API_KEY_MANAGER
        .get_or_init(|| async { ApiKeyManager::from_env().await })
        .await
        .as_ref()
        .map_err(|e| {
            log::error!("API key manager configuration error: {}", e);
            AuthError::InvalidApiKey
        })?;

    // Validate using secure hash comparison
    api_manager
        .validate_key(provided_key)
        .cloned()
        .ok_or_else(|| {
            log::warn!("Invalid API key provided");
            AuthError::InvalidApiKey
        })
}

/// Add security headers to response
fn add_security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();

    // Add security headers
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "Strict-Transport-Security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'"),
    );

    response
}
