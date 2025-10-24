use std::time::Duration;

use anyhow::{Result, bail};
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::EncodingKey;

use crate::catalog;
use crate::err::Error;

/// Helper function to parse PEM format and extract DER bytes
/// PEM format is base64 encoded DER between BEGIN and END markers
fn pem_to_der(pem: &str) -> Result<Vec<u8>> {
	// Remove BEGIN and END lines, strip whitespace, and decode base64
	let pem_cleaned = pem
		.lines()
		.filter(|line| !line.starts_with("-----"))
		.collect::<String>();
	
	base64::decode(&pem_cleaned)
		.map_err(|e| anyhow::anyhow!("Failed to decode PEM base64: {}", e))
}

pub(crate) fn config(alg: catalog::Algorithm, key: &str) -> Result<EncodingKey> {
	match alg {
		catalog::Algorithm::Hs256 => Ok(EncodingKey::from_secret(key.as_ref())),
		catalog::Algorithm::Hs384 => Ok(EncodingKey::from_secret(key.as_ref())),
		catalog::Algorithm::Hs512 => Ok(EncodingKey::from_secret(key.as_ref())),
		catalog::Algorithm::EdDSA => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_ed_der(&der))
		},
		catalog::Algorithm::Es256 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_ec_der(&der))
		},
		catalog::Algorithm::Es384 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_ec_der(&der))
		},
		catalog::Algorithm::Es512 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_ec_der(&der))
		},
		catalog::Algorithm::Ps256 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_rsa_der(&der))
		},
		catalog::Algorithm::Ps384 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_rsa_der(&der))
		},
		catalog::Algorithm::Ps512 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_rsa_der(&der))
		},
		catalog::Algorithm::Rs256 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_rsa_der(&der))
		},
		catalog::Algorithm::Rs384 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_rsa_der(&der))
		},
		catalog::Algorithm::Rs512 => {
			let der = pem_to_der(key)?;
			Ok(EncodingKey::from_rsa_der(&der))
		},
	}
}

pub(crate) fn expiration(d: Option<Duration>) -> Result<Option<i64>> {
	let exp = match d {
		Some(v) => {
			// The defined duration must be valid
			match ChronoDuration::from_std(v) {
				// The resulting expiration must be valid
				Ok(d) => match Utc::now().checked_add_signed(d) {
					Some(exp) => Some(exp.timestamp()),
					None => bail!(Error::AccessInvalidExpiration),
				},
				Err(_) => bail!(Error::AccessInvalidDuration),
			}
		}
		_ => None,
	};

	Ok(exp)
}
