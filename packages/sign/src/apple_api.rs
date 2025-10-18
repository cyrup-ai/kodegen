//! App Store Connect API client for certificate provisioning

use anyhow::{Context, Result};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const API_BASE: &str = "https://api.appstoreconnect.apple.com";

#[derive(Serialize)]
struct Claims {
    iss: String,  // Issuer ID
    iat: u64,     // Issued at timestamp
    exp: u64,     // Expiration (max 20 minutes)
    aud: String,  // Audience: "appstoreconnect-v1"
}

pub struct AppleAPIClient {
    key_id: String,
    issuer_id: String,
    private_key: Vec<u8>,
}

impl AppleAPIClient {
    /// Create client from API credentials
    pub fn new(key_id: &str, issuer_id: &str, key_path: &Path) -> Result<Self> {
        let private_key = std::fs::read(key_path)
            .with_context(|| format!("Failed to read .p8 key from {}", key_path.display()))?;
        
        Ok(Self {
            key_id: key_id.to_string(),
            issuer_id: issuer_id.to_string(),
            private_key,
        })
    }

    /// Generate JWT token for API authentication
    fn generate_jwt(&self) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let claims = Claims {
            iss: self.issuer_id.clone(),
            iat: now,
            exp: now + 1200, // 20 minutes
            aud: "appstoreconnect-v1".to_string(),
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let encoding_key = EncodingKey::from_ec_pem(&self.private_key)?;
        
        encode(&header, &claims, &encoding_key)
            .context("Failed to generate JWT token")
    }

    /// Request Developer ID Application certificate from Apple
    pub fn request_certificate(&self, csr_pem: &str) -> Result<Vec<u8>> {
        let jwt = self.generate_jwt()?;
        
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
                    certificate_type: "DEVELOPER_ID_APPLICATION".to_string(),
                    csr_content: csr_pem.to_string(),
                },
            },
        };

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(format!("{}/v1/certificates", API_BASE))
            .header("Authorization", format!("Bearer {}", jwt))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()?;

        if !response.status().is_success() {
            let error_text = response.text()?;
            anyhow::bail!("Certificate request failed: {}", error_text);
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
            certificate_content: String,  // base64-encoded DER
        }

        let cert_response: CertResponse = response.json()?;
        use base64::Engine;
        let cert_der = base64::engine::general_purpose::STANDARD.decode(&cert_response.data.attributes.certificate_content)?;
        
        Ok(cert_der)
    }
}

/// Generate CSR using rcgen (already in Cargo.toml)
pub fn generate_csr(common_name: &str, _email: &str) -> Result<(String, String)> {
    use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
    
    // Generate key pair
    let key_pair = KeyPair::generate()?;
    let private_key_pem = key_pair.serialize_pem();
    
    // Create certificate parameters for CSR
    let mut params = CertificateParams::new(vec![])?;
    
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, common_name);
    dn.push(DnType::CountryName, "US");
    params.distinguished_name = dn;
    
    // Generate CSR
    let csr = params.serialize_request(&key_pair)?;
    let csr_pem = csr.pem()?;
    
    Ok((csr_pem, private_key_pem))
}
