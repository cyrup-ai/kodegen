//! App Store Connect API client for certificate provisioning

use crate::config::CertificateType;
use anyhow::{Context, Result};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeroize::{Zeroize, ZeroizeOnDrop};

const API_BASE: &str = "https://api.appstoreconnect.apple.com";

// JWT token configuration (Apple docs: 20min max)
const JWT_LIFETIME_SECS: u64 = 1200;
const JWT_SAFETY_BUFFER_SECS: u64 = 60;

// Retry configuration (pattern: fluent_voice.rs)
const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_BASE_DELAY_MS: u64 = 1000;

#[derive(Serialize)]
struct Claims {
    iss: String, // Issuer ID
    iat: u64,    // Issued at timestamp
    exp: u64,    // Expiration (max 20 minutes)
    aud: String, // Audience: "appstoreconnect-v1"
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AppleAPIClient {
    key_id: String,
    issuer_id: String,
    private_key: Vec<u8>,
    #[zeroize(skip)]
    cached_token: Mutex<Option<(String, u64)>>,
}

impl AppleAPIClient {
    /// Create client from API credentials
    pub fn new(key_id: &str, issuer_id: &str, key_path: &Path) -> Result<Self> {
        // Validate key_id (10 alphanumeric)
        if key_id.len() != 10 {
            anyhow::bail!(
                "API key ID must be 10 characters (got: {})\n\
                 Example: AB12CD34EF",
                key_id.len()
            );
        }
        if !key_id.chars().all(|c| c.is_ascii_alphanumeric()) {
            anyhow::bail!("API key ID must be alphanumeric: '{key_id}'");
        }

        // Validate issuer_id (36-char UUID)
        if issuer_id.len() != 36 {
            anyhow::bail!(
                "Issuer ID must be UUID format (got: {})\n\
                 Example: 12345678-1234-1234-1234-123456789012",
                issuer_id.len()
            );
        }
        let parts: Vec<&str> = issuer_id.split('-').collect();
        if parts.len() != 5 {
            anyhow::bail!("Issuer ID must have UUID structure: '{issuer_id}'");
        }

        // Read and validate private key
        let private_key = std::fs::read(key_path)
            .with_context(|| format!("Failed to read .p8 key: {}", key_path.display()))?;

        if private_key.is_empty() {
            anyhow::bail!("Private key file is empty: {}", key_path.display());
        }

        let key_str = String::from_utf8_lossy(&private_key);
        if !key_str.contains("-----BEGIN PRIVATE KEY-----") {
            anyhow::bail!(
                "Private key must be PEM format (.p8 file): {}",
                key_path.display()
            );
        }

        Ok(Self {
            key_id: key_id.to_string(),
            issuer_id: issuer_id.to_string(),
            private_key,
            cached_token: Mutex::new(None),
        })
    }

    /// Generate JWT token for API authentication
    fn generate_jwt(&self) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("System clock error. Check time is after 1970-01-01 and NTP is enabled.")?
            .as_secs();

        if now < 1577836800 {
            // 2020-01-01
            anyhow::bail!(
                "System clock incorrect (timestamp: {}, year ~{}). Enable NTP sync.",
                now,
                1970 + (now / 31_557_600)
            );
        }

        let claims = Claims {
            iss: self.issuer_id.clone(),
            iat: now,
            exp: now + JWT_LIFETIME_SECS,
            aud: "appstoreconnect-v1".to_string(),
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let encoding_key = EncodingKey::from_ec_pem(&self.private_key)?;

        encode(&header, &claims, &encoding_key).context("Failed to generate JWT token")
    }

    /// Get cached JWT or generate new one if expired
    fn get_or_generate_jwt(&self) -> Result<String> {
        // Phase 1: Check cache (brief lock)
        {
            let cache = match self.cached_token.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    eprintln!("[WARN] Mutex poisoned, recovering");
                    poisoned.into_inner()
                }
            };

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("Failed to get time for JWT validation")?
                .as_secs();

            if let Some((token, expiry)) = &*cache
                && now + JWT_SAFETY_BUFFER_SECS < *expiry
            {
                return Ok(token.clone());
            }
        } // Lock released - important!

        // Phase 2: Generate without lock
        let token = self.generate_jwt()?;
        let expiry = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + JWT_LIFETIME_SECS;

        // Phase 3: Update cache (brief lock)
        {
            let mut cache = match self.cached_token.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            *cache = Some((token.clone(), expiry));
        }

        Ok(token)
    }

    /// Test API credentials by generating a JWT token
    /// This validates the `key_id`, `issuer_id`, and private key without making API calls
    pub fn test_credentials(&self) -> Result<()> {
        // Generate JWT to verify credentials are valid
        let _jwt = self.generate_jwt()?;
        Ok(())
    }

    /// Request certificate from Apple
    pub fn request_certificate(
        &self,
        csr_pem: &str,
        cert_type: CertificateType,
    ) -> Result<Vec<u8>> {
        let mut last_error = None;

        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            if attempt > 1 {
                let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt - 2);
                eprintln!("[INFO] Retry {attempt}/{MAX_RETRY_ATTEMPTS} after {delay}ms");
                std::thread::sleep(Duration::from_millis(delay));
            }

            match self.request_certificate_inner(csr_pem, cert_type) {
                Ok(cert) => return Ok(cert),
                Err(e) if is_retryable_error(&e) => {
                    eprintln!("[WARN] Transient error: {e}");
                    last_error = Some(e);
                }
                Err(e) => return Err(e), // Non-retryable
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Certificate request failed after {MAX_RETRY_ATTEMPTS} attempts")
        }))
    }

    /// Inner implementation of certificate request
    fn request_certificate_inner(
        &self,
        csr_pem: &str,
        cert_type: CertificateType,
    ) -> Result<Vec<u8>> {
        let jwt = self.get_or_generate_jwt()?;

        #[derive(Serialize)]
        struct CertRequest {
            data: CertData,
        }

        #[derive(Serialize)]
        struct CertData {
            #[serde(rename = "type")]
            type_: String,
            attributes: CertAttributes,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CertAttributes {
            certificate_type: String,
            csr_content: String,
        }

        let request = CertRequest {
            data: CertData {
                type_: "certificates".to_string(),
                attributes: CertAttributes {
                    certificate_type: cert_type.to_apple_api_string().to_string(),
                    csr_content: csr_pem.to_string(),
                },
            },
        };

        // Create HTTP client with explicit timeouts to prevent hangs
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30)) // Total request timeout
            .connect_timeout(Duration::from_secs(10)) // Connection timeout
            .build()?;

        // Send certificate request with detailed error handling
        let response = client
            .post(format!("{API_BASE}/v1/certificates"))
            .header("Authorization", format!("Bearer {jwt}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow::anyhow!(
                        "Request to Apple API timed out after {} seconds. Check your network connection.",
                        if e.is_connect() { 10 } else { 30 }
                    )
                } else if e.is_connect() {
                    anyhow::anyhow!(
                        "Could not connect to Apple API. Check network/firewall settings."
                    )
                } else {
                    anyhow::anyhow!("Network error: {e}")
                }
            })?;

        // Handle non-success responses with sanitized error messages
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_default();
            eprintln!("[DEBUG] Apple API error ({status}): {error_text}");

            let detailed_error =
                if let Ok(err_resp) = serde_json::from_str::<AppleErrorResponse>(&error_text) {
                    if let Some(err) = err_resp.errors.first() {
                        format!("{}\nCode: {}\n{}", err.title, err.code, err.detail)
                    } else {
                        format!("Request failed: {status}")
                    }
                } else {
                    match status.as_u16() {
                        400 => "Invalid CSR format".to_string(),
                        401 => "Authentication failed - check API credentials".to_string(),
                        403 => "Permission denied - check App Store Connect role".to_string(),
                        429 => "Rate limited - wait and retry".to_string(),
                        _ => format!("Request failed: {status}"),
                    }
                };

            anyhow::bail!("{detailed_error}");
        }

        #[derive(Deserialize)]
        struct CertResponse {
            data: CertResponseData,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CertResponseData {
            attributes: CertResponseAttributes,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CertResponseAttributes {
            certificate_content: String, // base64-encoded DER
        }

        let cert_response: CertResponse = response.json()?;
        use base64::Engine;
        let cert_content = &cert_response.data.attributes.certificate_content;
        let cert_der = base64::engine::general_purpose::STANDARD
            .decode(cert_content)
            .with_context(|| {
                format!(
                    "Failed to decode cert ({}bytes, starts: {:?})",
                    cert_content.len(),
                    cert_content.chars().take(20).collect::<String>()
                )
            })?;

        validate_certificate_der(&cert_der)?;

        Ok(cert_der)
    }
}

// Apple App Store Connect API error response structures
#[derive(Deserialize)]
struct AppleErrorResponse {
    errors: Vec<AppleError>,
}

#[derive(Deserialize)]
struct AppleError {
    code: String,
    title: String,
    detail: String,
}

/// Helper function to determine if an error is retryable
fn is_retryable_error(error: &anyhow::Error) -> bool {
    let msg = error.to_string().to_lowercase();
    msg.contains("timeout")
        || msg.contains("connection")
        || msg.contains("50")
        || msg.contains("429")
}

/// Validate certificate DER data is well-formed
fn validate_certificate_der(cert_der: &[u8]) -> Result<()> {
    if cert_der.is_empty() {
        anyhow::bail!("Empty certificate from Apple");
    }
    if cert_der[0] != 0x30 {
        anyhow::bail!(
            "Invalid DER format (expected 0x30, got 0x{:02x})",
            cert_der[0]
        );
    }
    if cert_der.len() < 100 || cert_der.len() > 10_000 {
        anyhow::bail!("Suspicious cert size: {} bytes", cert_der.len());
    }
    Ok(())
}

/// Generate CSR using rcgen (already in Cargo.toml)
pub fn generate_csr(common_name: &str) -> Result<(String, String)> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

    let key_pair = KeyPair::generate()?;
    let private_key_pem = key_pair.serialize_pem();

    let mut params = CertificateParams::new(vec![])?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, common_name);
    // No country code - Apple doesn't require it
    params.distinguished_name = dn;

    let csr = params.serialize_request(&key_pair)?;
    let csr_pem = csr.pem()?;

    Ok((csr_pem, private_key_pem))
}
